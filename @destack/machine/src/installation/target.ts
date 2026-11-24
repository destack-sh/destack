import {
    Installation,
    installation,
    build,
    Build,
    type BuildSource,
    StackEvaluation,
    Binding,
    InstallationRevision,
    installationRevision,
    type RevisionDescription,
    type PackageRequirement,
} from "@destack/stack/object";
import type { Machine } from "../machine/machine.ts";
import { callRegistry, findPublisherHome } from "../machine/registry.ts";
import type { Space } from "../space/space.ts";
import { Failure } from "@destack/error";
import { DeclarationReference } from "@destack/package/declare";
import {
    BOOTSTRAP_PACKAGES,
    installableBuilds,
    MAIN_DATABASE,
    StackDefinition,
} from "@destack/stack/declare";
import { TABLE, DatabaseState, Table, TableState } from "@destack/db";
import type { BlobStore, DatabaseConnection, Snapshot } from "@destack/db/database";
import {
    InstallationBindingSpec,
    type InstallationBindingState,
    InstallationKind,
    ObjectType,
    PackageDescription,
    type InstallationVersion,
    type MethodContext,
} from "@destack/object";
import { type DependencyRelease, PackageId, PackageRelease } from "@destack/package";
import { type BuildReader, BuildImage, PackageManifest } from "@destack/package/manifest";
import { ResourceDescription } from "@destack/resource";
import {
    Digest,
    type DigestCrypto,
    Identifier,
    type JsonValue,
    present,
    schema,
    Version,
} from "@destack/schema";
import { ObjectReference, ObjectTypeReference } from "@destack/sync";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { followedReference, followedVersion } from "./follow.ts";
import { packageObject, release, reference, repository, tag } from "@destack/registry/object";
import { RegistryError } from "@destack/registry/error";
import { checkout } from "../computer/checkout/object/index.ts";
import type { Binder } from "../resource/binding.ts";
import { space } from "@destack/account/object";
import { StackError } from "@destack/stack/error";

/** What targeting and describing revisions read: space, blobs and the known types. */
export interface RevisionOptions {
    /** The kinds of objects bindings target. */
    readonly binder: Binder;
    /** The space whose builds are read, through to its machine's builds and registries. */
    readonly space: Pick<
        Space,
        "builds" | "machine" | "root" | "pull" | "read" | "requireFollowable" | "requireRepository"
    >;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The types every space knows beside those it serves, which descriptions refer to. */
    readonly known: readonly ObjectType[];
    /** The release the machine links of each package, which its spaces run before any revision. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The releases the machine ships, by package. */
    readonly shipped: ReadonlyMap<PackageId, PackageRelease>;
    /** The images of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
}

/** A build chosen for an installation, with what its revision records of it. */
export interface ChosenBuild extends SelectedBuild, RevisionDescription {}

/** The build a selection resolves to: the space's build, its image and source. */
export interface SelectedBuild {
    /** The space's build, absent for a release its apply records. */
    readonly build?: Identifier<"build">;
    /** The digest of the build's image. */
    readonly image: Digest;
    /** Where the build comes from. */
    readonly source: BuildSource;
}

/** The kind of row an installation's version is, or the host's release without one. */
export type VersionKind = "host" | "release" | "tag" | "reference" | "checkout" | "build";

/** The revision an installation targets: its build selected, its stack evaluated, recorded. */
export const InstallationTarget = {
    /** Report whether an installation targets the build its version names, by its space's rows. */
    async isCurrent(
        reader: Pick<Snapshot, "read">,
        installed: Installation,
        options: RevisionOptions,
    ): Promise<boolean> {
        // follow nothing without a targeted build
        const current =
            installed.targetRevisionId === null
                ? undefined
                : await InstallationRevision.build(reader, installed.targetRevisionId);
        if (current === undefined) {
            return false;
        }
        const kind = InstallationTarget.kind(installed);
        const { isBuilding } = options.space.machine;
        const released = current.source.kind === "release" ? current.source.version : undefined;

        // compare its release's or tag's version where the space has its registry's rows
        if (kind === "release" || kind === "tag") {
            const id = present(installed.versionId, "its version's row");
            const version = !isBuilding
                ? undefined
                : kind === "tag"
                  ? await followedVersion(reader, id)
                  : (
                        await reader
                            .read(release.table)
                            .findFirst({ where: { id: schema.identifier("release").parse(id) } })
                    )?.version;

            return version !== undefined && version === released;
        }
        // compare the build the host, a pin, a reference or a checkout selects
        else {
            const selected = await selectBuild(reader, installed, undefined, options);

            return selected?.image === current.image;
        }
    },

    /** Read the kind of row an installation's version is, the host's release without one. */
    kind(installed: Pick<Installation, "versionType">): VersionKind {
        // run the host's release without a version row
        const type = installed.versionType;
        if (type === null) {
            return "host";
        }

        // name the version row's type, refusing a type no version is
        const kinds: Readonly<Record<string, VersionKind>> = {
            [release.name]: "release",
            [tag.name]: "tag",
            [reference.name]: "reference",
            [checkout.name]: "checkout",
            [build.name]: "build",
        };
        const kind = kinds[type];
        if (kind === undefined) {
            throw new TypeError(`an installation cannot follow a ${type}`);
        }

        return kind;
    },

    /** Read the row an installation's version is, absent for the host's release. */
    version(columns: {
        readonly versionPackageId?: string | null;
        readonly versionType?: string | null;
        readonly versionScope?: string | null;
        readonly versionId?: string | null;
    }): ObjectReference | undefined {
        return columns.versionId === undefined || columns.versionId === null
            ? undefined
            : ObjectReference.at("version", columns);
    },

    /** Read a space's live installations and what each one's targeted revision requires. */
    async requirements(
        call: Pick<MethodContext, "read">,
        scope: string,
        options: Pick<RevisionOptions, "space">,
    ): Promise<{
        readonly installed: readonly Installation[];
        readonly requirements: readonly PackageRequirement[];
    }> {
        // read the live installations, their targeted revisions and those revisions' builds
        const rows = await call.read(installation.table).findMany({ where: { scope } });
        const installed = rows.filter((row) => row.deletedAt === null);
        const targeted = installed.flatMap((row) =>
            row.targetRevisionId === null ? [] : [row.targetRevisionId],
        );
        const revisions = await call
            .read(installationRevision.table)
            .findMany({ where: { id: { in: targeted } } });
        const builds = await call
            .read(build.table)
            .findMany({ where: { id: { in: revisions.map((each) => each.buildId) } } });

        // list what each requires, by its package description, but the space's own packages
        const own = new Set<PackageId>(Object.values(BOOTSTRAP_PACKAGES).map((each) => each.id));
        const requirements: PackageRequirement[] = [];
        for (const row of installed) {
            const revision = revisions.find((each) => each.id === row.targetRevisionId);
            const built = builds.find((each) => each.id === revision?.buildId);
            if (revision !== undefined && built !== undefined) {
                const reader = await options.space.root(row.packageId, Build.image(built));
                const description = await PackageDescription.of(reader);
                const required = Installation.requires(row, revision, description);
                requirements.push(...required.filter((each) => !own.has(each.package.id)));
            }
        }

        return { installed, requirements };
    },

    /** Resolve the row a declared version names, or the host's image for a shipped release. */
    async resolveDeclared(
        packageId: PackageId,
        version: InstallationVersion,
        options: Machine,
    ): Promise<{
        readonly version: ObjectReference | undefined;
        readonly directory: string | null;
        /** The host's image it runs, absent unless the host provides its package. */
        readonly hostImage: Digest | null;
    }> {
        // run a release the host ships as the host's build of it, which needs no registry row
        const shipped = options.shipped.get(packageId)?.package.version;
        if (
            version.kind === "host" ||
            (version.kind === "release" && version.version === shipped)
        ) {
            const provided = InstallationTarget.provided(packageId, options);

            return { version: undefined, directory: null, hostImage: provided?.image ?? null };
        } else if (version.kind === "checkout") {
            const row = checkout.reference(version.machine, version.checkout);

            return { version: row, directory: version.directory, hostImage: null };
        }

        // read a release, tag or reference at its registry's home, and a checkout on its machine
        const row = await InstallationTarget.resolveVersion(options, packageId, version);

        return {
            version: row,
            directory: version.kind === "repository" ? version.directory : null,
            hostImage: null,
        };
    },

    /** Find the release, tag or repository reference a version names, at its registry's home. */
    async resolveVersion(
        machine: Machine,
        packageId: PackageId,
        selection: Exclude<InstallationVersion, { readonly kind: "host" | "checkout" }>,
    ): Promise<ObjectReference> {
        // take the release a version names at its package's registry
        if (selection.kind === "release") {
            const home = await findPublisherHome(machine, packageObject, packageId);
            const found = await callRegistry(machine, home).release.resolve({
                spaceId: home,
                parentId: packageId,
                release: selection.version,
            });

            return release.reference(found.scope, found.id);
        }
        // find the package's tag by its name
        else if (selection.kind === "tag") {
            const home = await findPublisherHome(machine, packageObject, packageId);
            const { items } = await callRegistry(machine, home).tag.list({
                spaceId: home,
                where: { parentId: packageId, name: selection.tag },
                limit: 1,
            });
            const [row] = items;
            if (row === undefined) {
                throw RegistryError.of(
                    "RELEASE_NOT_FOUND",
                    `tag '${selection.tag}' of '${packageId}' not found`,
                );
            }

            return tag.reference(row.scope, row.id);
        }
        // find the repository's reference by its name
        else {
            const home = await findPublisherHome(machine, repository, selection.repository);
            const { items } = await callRegistry(machine, home).reference.list({
                spaceId: home,
                where: { parentId: selection.repository, name: selection.reference },
                limit: 1,
            });
            const [row] = items;
            if (row === undefined) {
                throw RegistryError.of(
                    "RELEASE_NOT_FOUND",
                    `reference '${selection.reference}' of repository '${selection.repository}' not found`,
                );
            }

            return reference.reference(row.scope, row.id);
        }
    },

    /** Report whether an installation's build resolves without a build here: a release or a pin. */
    isPinned(installed: Pick<Installation, "versionType">): boolean {
        const kind = InstallationTarget.kind(installed);

        return kind === "host" || kind === "release" || kind === "tag" || kind === "build";
    },

    /** Attribute a reconcile step's span to the target's space and installation. */
    attributes(target: Pick<Installation, "scope" | "id">) {
        return {
            "destack.space.id": target.scope,
            "destack.id": target.id,
        };
    },

    /** Read the release this machine links or ships of a package, none for one it lacks. */
    provided(
        packageId: PackageId,
        options: Pick<RevisionOptions, "linked" | "shipped">,
    ): PackageRelease | undefined {
        return options.linked.get(packageId) ?? options.shipped.get(packageId);
    },

    /** Read the release this machine provides of a package, which one following the host's runs. */
    host(
        packageId: PackageId,
        options: Pick<RevisionOptions, "linked" | "shipped">,
    ): PackageRelease {
        const provided = InstallationTarget.provided(packageId, options);
        if (provided === undefined) {
            throw StackError.of(
                "TARGET_BLOCKED",
                `cannot follow the host's release of '${packageId}': this machine provides none`,
            );
        }

        return provided;
    },

    /** Resolve the image an installation runs, undefined for a host release the machine lacks. */
    running(
        installed: Pick<Installation, "packageId" | "versionType">,
        recorded: Digest,
        options: Pick<RevisionOptions, "linked" | "shipped">,
    ): Digest | undefined {
        const { packageId } = installed;

        return InstallationTarget.kind(installed) === "host"
            ? InstallationTarget.provided(packageId, options)?.image
            : (options.linked.get(packageId)?.image ?? recorded);
    },

    /** Choose the build an installation runs and describe its revision, none while none is here. */
    async choose(
        reader: Pick<Snapshot, "read">,
        row: Installation,
        requested: Identifier<"build"> | undefined,
        options: RevisionOptions,
    ): Promise<ChosenBuild | undefined> {
        // read the build its targeted revision evaluates
        const revision =
            row.targetRevisionId === null
                ? undefined
                : await reader
                      .read(installationRevision.table)
                      .findFirst({ where: { id: row.targetRevisionId } });
        const built =
            revision === undefined
                ? undefined
                : await reader.read(build.table).findFirst({ where: { id: revision.buildId } });
        const current = built === undefined ? undefined : Build.of(built);

        // take the requested build, else its version's build, else its targeted one
        const { scope } = options.space.machine;
        const attributes = InstallationTarget.attributes(row);
        const selected =
            requested === undefined
                ? await scope.span("destack.installation.select", attributes, () =>
                      selectBuild(reader, row, current, options),
                  )
                : await Build.require(reader, Installation.space(row), requested).then((found) => ({
                      build: found.id,
                      image: found.image,
                      source: found.source,
                  }));
        const targeted =
            current === undefined
                ? undefined
                : { build: current.id, image: Build.image(current), source: current.source };
        const chosen = selected ?? targeted;

        // choose none without a build, or with one whose image the machine lacks
        if (
            chosen === undefined ||
            (await options.space.builds.storeOf(chosen.image)) === undefined
        ) {
            return undefined;
        }

        // describe its revision, reusing the targeted one's for the same build
        const image = await options.space.builds.image(chosen.image);
        const described =
            revision !== undefined && revision.buildId === chosen.build
                ? {
                      dependencies: revision.dependencies,
                      states: revision.states,
                      release: revision.release,
                      declarations: revision.declarations,
                  }
                : await scope.span("destack.installation.describe", attributes, () =>
                      InstallationTarget.describe(image, options),
                  );

        return { ...chosen, ...described };
    },

    /** Pull the release an installation follows, none while its tag is unset. */
    async pull(
        reader: Pick<Snapshot, "read">,
        row: Installation,
        options: RevisionOptions,
    ): Promise<SelectedBuild | undefined> {
        // leave an installation following a build of the space
        const installed = row;
        const { packageId } = installed;
        const kind = InstallationTarget.kind(installed);
        if (kind !== "host" && kind !== "release" && kind !== "tag") {
            return undefined;
        }

        // leave a host's release already on the machine
        const { isBuilding } = options.space.machine;
        const hosted = kind === "host" ? InstallationTarget.host(packageId, options) : undefined;
        if (
            hosted !== undefined &&
            (await options.space.builds.storeOf(hosted.image)) !== undefined
        ) {
            return undefined;
        }

        // leave a tag still pointing at the targeted release
        const current =
            row.targetRevisionId === null
                ? undefined
                : await InstallationRevision.build(reader, row.targetRevisionId);
        const tagged =
            kind === "tag" && isBuilding
                ? await followedVersion(reader, present(installed.versionId, "its version's tag"))
                : undefined;
        if (
            tagged !== undefined &&
            current?.source.kind === "release" &&
            current.source.version === tagged
        ) {
            return undefined;
        }

        // resolve the host's release, or its release or tag at its registry
        const resolved = hosted ?? (await selectedRelease(reader, installed, options));
        if (resolved === undefined) {
            return undefined;
        }

        // pull its image onto the machine
        const { image } = resolved;
        const pulled = await options.space.pull(packageId, image);
        const version = (await options.space.builds.get(pulled.root)).package.version;

        return { image, source: { kind: "release", package: packageId, version, image } };
    },

    /** Read the build an installation follows, else the linked release, else its selection. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId: Identifier<"installation"> | undefined,
        options: Pick<RevisionOptions, "linked" | "shipped"> & {
            readonly space: Pick<Space, "builds" | "machine" | "root" | "pull" | "read">;
        },
    ): Promise<BuildReader | undefined> {
        // read the package's installations, refusing several unless one is named
        const installed = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        const rows = await snapshot
            .read(installation.table)
            .findMany({ where: { id: { in: installed.map((row) => row.id) } } });
        const [found] = rows;
        const [only] = installed.length === 1 ? installed : [];
        const linked = options.linked.get(packageId);
        const followed = installed.find((row) => row.id === found?.id);
        const following = followed ?? only;
        const kind = following === undefined ? undefined : InstallationTarget.kind(following);
        if (rows.length > 1) {
            throw StackError.of(
                "TARGET_BLOCKED",
                "cannot pick an installation: several installations in this space follow builds of this package",
            );
        }
        // answer none for a package the space neither installs nor links
        else if (installed.length === 0 && linked === undefined) {
            return undefined;
        }
        // read the release the machine provides for one following the host's
        else if (kind === "host") {
            return options.space.root(packageId, InstallationTarget.host(packageId, options).image);
        }
        // read the revision's build
        else if (found?.targetRevisionId !== undefined && found.targetRevisionId !== null) {
            return InstallationRevision.reader(
                snapshot,
                found.targetRevisionId,
                packageId,
                options.space,
            );
        }
        // read the release the machine links, which runs until a revision replaces it
        else if (linked !== undefined) {
            return options.space.root(packageId, linked.image);
        }
        // resolve a planned installation's release
        else if (following !== undefined && (kind === "release" || kind === "tag")) {
            const resolved = await selectedRelease(snapshot, following, options);
            if (resolved !== undefined) {
                return options.space.root(packageId, resolved.image);
            }
        }

        // refuse an installation following nothing yet
        throw StackError.of(
            "INSTALLATION_NOT_FOUND",
            "installation in this space following a build of this package not found",
        );
    },

    /** Evaluate a locked build's stack at an export, with the builds its image has. */
    async evaluate(
        inputs: Omit<StackEvaluation, "definition">,
        image: BuildImage,
        options: {
            readonly space: Pick<Space, "builds" | "machine" | "root" | "pull" | "read">;
        },
    ): Promise<StackEvaluation> {
        // take the builds the machine links, then the image's
        const { machine } = options.space;
        const images = await machine.linkedImages();
        const linked = [...machine.linked.values()].map((provided) => {
            const { root } = present(
                images.get(provided.package.name),
                `the image of ${provided.package.name}`,
            );
            const entry: DependencyRelease = {
                kind: "destack",
                package: provided.package,
                manifest: root,
            };

            return entry;
        });
        const releases = [
            ...linked,
            ...(await BuildImage.releases(image, (digest) => options.space.builds.get(digest))),
        ];

        // read the stack the image's root declares at the export
        const definition = await StackDefinition.declared(
            await options.space.builds.contents(image.root),
            releases,
            inputs.export,
            inputs,
        );

        return { ...inputs, definition };
    },

    /** Read what a stack installation's evaluation reads: its export, arguments and settings. */
    async inputs(
        snapshot: Pick<Snapshot, "read">,
        installed: Pick<Installation, "scope" | "packageId" | "export" | "parameters">,
    ): Promise<Omit<StackEvaluation, "definition">> {
        // read the export and its arguments, which every stack installation has
        const exported = present(installed.export, `the export of ${installed.packageId}`);
        const parameters = present(installed.parameters, `the arguments of ${exported}`);

        // take the settings and routes the space's stack places while the installation follows its package
        const [owner] = await snapshot.read(space.table).findMany({
            where: { id: schema.identifier("space").parse(installed.scope) },
            limit: 1,
        });
        const stack = present(owner, `the space ${installed.scope}`).stack;
        const isFollowing = stack.packageId === installed.packageId;

        return {
            export: exported,
            parameters,
            settings: isFollowing ? stack.settings : {},
            routes: isFollowing ? stack.routes : {},
        };
    },

    /** Resolve an installation's value of each space setting its build declares, by setting key. */
    async settings(
        database: DatabaseConnection,
        reader: BuildReader,
        target: {
            readonly scope: Identifier<"space">;
            readonly installation: Identifier<"installation">;
        },
    ): Promise<Record<string, JsonValue>> {
        // resolve each space setting the build declares for the installation
        const declared = await Setting.list(reader);
        const placed = declared.filter((setting) => setting.definition.scope === "space");
        const resolved = await Promise.all(
            placed.map(async (setting): Promise<[string, JsonValue]> => [
                DeclarationReference.key(setting.reference),
                schema
                    .json()
                    .parse(await SettingValue.resolve(database.snapshot(), setting, target)),
            ]),
        );

        return Object.fromEntries(resolved);
    },

    /** Describe a build's revision of an installation: its dependencies and required states. */
    async describe(
        image: BuildImage,
        options: Pick<RevisionOptions, "space" | "blobs" | "known">,
    ): Promise<RevisionDescription> {
        // read the image's root and the builds of the installable packages the image has
        const { blobs } = options;
        const { scope } = options.space.machine;
        const reader = await options.space.builds.contents(image.root);
        const entries = await scope.span("destack.installation.describe.entries", {}, () =>
            BuildImage.entries(image, (digest) => options.space.builds.get(digest)),
        );
        const read = (dependency: PackageId, manifest: Digest) =>
            options.space.read(dependency, manifest);
        const dependencies = await scope.span(
            "destack.installation.describe.dependencies",
            {},
            () => installableBuilds(entries, PackageManifest.package(reader.manifest).id, read),
        );

        // describe the state each declaration requires: main's tables, an installation's release, a resource's own
        const required = new Map<string, JsonValue>();
        const tables = await scope.span("destack.installation.describe.tables", {}, () =>
            mainState(reader, options),
        );
        if (tables !== undefined) {
            required.set(MAIN_DATABASE, tables);
        }
        const declared = await scope.span("destack.installation.describe.states", {}, () =>
            declaredStates(reader, entries),
        );
        for (const [name, state] of declared) {
            required.set(name, state);
        }

        // write each state as a blob, keyed by its declaration
        const states = await scope.span("destack.installation.describe.write", {}, () =>
            Promise.all(
                [...required].map(
                    async ([name, state]) => [name, await blobs.writeJson(state)] as const,
                ),
            ),
        );

        // record the release and declarations the next upgrade plans from
        const declarations = await blobs.writeJson(await reader.declarations());

        return {
            dependencies,
            states: Object.fromEntries(states),
            release: reader.manifest.package.version,
            declarations,
        };
    },

    /** Describe the call that records a space's build and its stack evaluation as a revision. */
    async create(
        target: Pick<Installation, "id" | "scope" | "packageId" | "generation">,
        evaluated: { readonly id: Identifier<"build">; readonly image: Digest },
        described: RevisionDescription,
        evaluation: StackEvaluation | undefined,
        bindings: readonly Binding[],
        crypto: DigestCrypto,
    ) {
        return installationRevision.call("create", {
            ...installationRevision.in(target.scope),
            installationId: target.id,
            packageId: target.packageId,
            buildId: evaluated.id,
            dependencies: described.dependencies,
            states: described.states,
            release: described.release,
            declarations: described.declarations,
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            bindings: [...bindings],
            generation: target.generation,
            digest: await InstallationRevision.digest(
                evaluated.image,
                evaluation,
                bindings,
                crypto,
            ),
        });
    },
};

/** Resolve what an installation follows to its build, none until built or pulled. */
async function selectBuild(
    snapshot: Pick<Snapshot, "read">,
    row: Installation,
    current: Build | undefined,
    options: RevisionOptions,
): Promise<SelectedBuild | undefined> {
    // read its version
    const installed = row;
    const { packageId } = installed;
    const kind = InstallationTarget.kind(installed);

    // take the release the machine provides for one following the host's, reusing its current build
    if (kind === "host") {
        const { image } = InstallationTarget.host(packageId, options);
        const source = current?.source;
        if (source?.kind === "release" && current !== undefined && Build.image(current) === image) {
            return { image, source, build: current.id };
        }
        const stored = await options.space.builds.storeOf(image);
        if (stored === undefined) {
            return undefined;
        }
        const root = (await options.space.builds.image(image)).root;
        const version = (await options.space.builds.get(root)).package.version;

        return { image, source: { kind: "release", package: packageId, version, image } };
    }
    // leave a release or tag version to its pull
    else if (kind === "release" || kind === "tag") {
        return undefined;
    }
    // take the build pinned by a rollback
    else if (kind === "build") {
        const pinned = await Build.require(
            snapshot,
            Installation.space(row),
            schema.identifier("build").parse(installed.versionId),
        );

        return pinned.state === "succeeded"
            ? { image: Build.image(pinned), source: pinned.source, build: pinned.id }
            : undefined;
    }
    // take the succeeded build of the commit its reference points at
    else if (kind === "reference") {
        const followed = options.space.machine.isBuilding
            ? await followedReference(
                  snapshot,
                  present(installed.versionId, "its version's reference"),
              )
            : undefined;
        if (followed?.commit === undefined) {
            return undefined;
        }
        const source = {
            kind: "commit" as const,
            repository: followed.repository,
            commit: followed.commit,
            directory: present(installed.directory, "its version's directory"),
        };
        const found = await Build.find(snapshot, Installation.space(row), source);

        return found?.state === "succeeded"
            ? { image: Build.image(found), source, build: found.id }
            : undefined;
    }
    // take its checkout's newest succeeded build
    else {
        const newest = (
            await snapshot.read(build.table).findMany({
                where: {
                    scope: Installation.space(row),
                    state: "succeeded",
                    expiredAt: { isNull: true },
                },
            })
        )
            .map((entry) => Build.of(entry))
            .filter(
                (entry) =>
                    entry.source.kind === "checkout" &&
                    entry.source.checkout === installed.versionId &&
                    entry.source.directory === installed.directory,
            )
            .toSorted((left, right) => right.createdAt - left.createdAt)
            .at(0);

        return newest === undefined
            ? undefined
            : { image: Build.image(newest), source: newest.source, build: newest.id };
    }
}

/** Resolve a followed release or tag to its version and image, none while its tag is unset. */
async function selectedRelease(
    snapshot: Pick<Snapshot, "read">,
    installed: Pick<
        Installation,
        "versionPackageId" | "versionType" | "versionScope" | "versionId" | "packageId"
    >,
    options: {
        readonly space: Pick<Space, "builds" | "machine" | "root" | "pull" | "read">;
    },
): Promise<{ readonly version: string; readonly image: Digest } | undefined> {
    // read the version a tag points at in the space's replica of its registry
    const { packageId } = installed;
    const { machine } = options.space;
    const followed = ObjectReference.at("version", installed);
    const kind = InstallationTarget.kind(installed);
    if (kind === "tag" && machine.isBuilding) {
        const version = await followedVersion(snapshot, followed.id);

        return version === undefined
            ? undefined
            : { version, image: await machine.resolve(packageId, version) };
    }
    // read a release's version, or a tag's on a machine replicating no tags, at its registry
    else {
        const version = await readVersion(machine, followed);
        const image = await published(machine, packageId, version);

        return image === undefined ? undefined : { version, image };
    }
}

/** Find a published release's image by version or tag, absent while none is published. */
async function published(
    machine: Pick<Machine, "resolve">,
    packageId: PackageId,
    named: string,
): Promise<Digest | undefined> {
    return await machine.resolve(packageId, named).catch((error: unknown) => {
        // answer absent for the registry's missing release, and fail on any other refusal
        if (Failure.of(error).reason === "RELEASE_NOT_FOUND") {
            return undefined;
        }
        throw error;
    });
}

/** The tables a build lists its package's types keep, by dialect, none for one without types. */
const ListedTables = schema.object({
    /** The SQLite table states. */
    sqlite: schema.array(TableState).default([]),
    /** The PostgreSQL table states. */
    postgresql: schema.array(TableState).default([]),
});

/** Read the tables a build keeps in the space's main database, followed ones included. */
async function mainState(
    reader: BuildReader,
    options: Pick<RevisionOptions, "space" | "known">,
): Promise<DatabaseState | undefined> {
    // read the tables its build lists of its own package
    const released = PackageManifest.package(reader.manifest);
    const listed = ListedTables.parse(await reader.tables());
    const isOwn = (state: TableState) => state.package.id === released.id;
    const own = {
        sqlite: listed.sqlite.filter(isOwn),
        postgresql: listed.postgresql.filter(isOwn),
    };

    // take the tables of the protocols it implements, for a package with tables of its own
    const description = await PackageDescription.of(reader);
    const protocols = description?.protocols ?? [];
    const isOwning = own.sqlite.length > 0 || own.postgresql.length > 0;
    const implemented = Table.affected(
        options.known
            .filter((type) => isOwning && protocols.includes(type.package.id))
            .filter((type) => type.storage === "durable" || type.storage === "events")
            .flatMap((type) => type.tables),
    );

    // take the tables of the types it follows, refusing a type this machine does not know
    const followed = (description?.follows ?? []).map((followedType) => {
        // find the type among those this machine knows
        const key = ObjectTypeReference.key(followedType);
        const type = options.known.find(
            (each) => ObjectTypeReference.key(each.typeReference) === key,
        );
        if (type === undefined) {
            throw StackError.of(
                "PACKAGE_INVALID",
                `package '${released.name}' is invalid: it follows ${followedType.type} objects of '${followedType.packageId}', which this machine does not know`,
            );
        }

        return type;
    });
    const followedTables = Table.affected(followed.flatMap((type) => type.tables));

    // read the tables the space replicates already
    const { volumes } = options.space.machine;
    const replicated = new Set([
        ...volumes.replicated.map((table) => table[TABLE].sqlName),
        ...volumes.main.replicated,
    ]);
    const isReplicated = (state: TableState) => replicated.has(state.table.name);

    // own the protocols' replicated tables, and replicate the followed tables the space lacks
    const described = DatabaseState.of([...implemented], { packages: protocols }).tables;
    const added = [...followedTables].filter((table) => !replicated.has(table[TABLE].sqlName));
    const replicas = DatabaseState.of(added, {
        replicated: new Set(added.map((table) => table[TABLE].sqlName)),
        packages: [...new Set(followed.map((type) => type.package.id))],
    }).tables;

    // list them beside its own, none for a build without tables
    const state = {
        sqlite: [...own.sqlite, ...described.sqlite.filter(isReplicated), ...replicas.sqlite],
        postgresql: [
            ...own.postgresql,
            ...described.postgresql.filter(isReplicated),
            ...replicas.postgresql,
        ],
    };

    return state.sqlite.length === 0 && state.postgresql.length === 0
        ? undefined
        : { tables: state };
}

/** Describe the state each resource declaration of a build requires of its target, by name. */
async function declaredStates(
    reader: BuildReader,
    entries: Readonly<Record<string, DependencyRelease>>,
): Promise<Map<string, JsonValue>> {
    // read the build's resource declarations
    const own = PackageManifest.package(reader.manifest);
    const resources = (await reader.declarations())
        .filter((declaration) => declaration.provisioned === true)
        .map((declaration) => ({
            isInstallation: InstallationKind.has(declaration),
            ...ResourceDescription.catchall(schema.json()).parse(declaration.description),
        }));

    // record an installation's callee release
    const states = new Map<string, JsonValue>();
    for (const { name, isInstallation, spec, ...state } of resources) {
        // record the callee's version in the image
        if (isInstallation) {
            const callee = InstallationBindingSpec.parse(spec).package;
            const version =
                callee === own.id
                    ? own.version
                    : Object.values(entries).find(
                          (entry) => entry.kind === "destack" && entry.package.id === callee,
                      )?.package.version;
            if (version === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `binding of installation '${name}' is invalid: the build binds it without requiring '${callee}'`,
                );
            }
            const recorded: InstallationBindingState = { release: Version.parse(version) };
            states.set(name, recorded);
        }
        // take the state another resource requires, none for a stateless one
        else if (Object.keys(state).length > 0) {
            states.set(name, state);
        }
    }

    return states;
}

/** Read the version a followed release or tag points at, at the registry in its home. */
async function readVersion(machine: Machine, followed: ObjectReference): Promise<string> {
    // read the tag, or the release itself, by its identifier where it lives
    const home = schema.identifier("space").parse(followed.scope);
    const client = callRegistry(machine, home);
    const row =
        followed.type === tag.name
            ? await client.tag.get({
                  spaceId: home,
                  id: schema.identifier("tag").parse(followed.id),
              })
            : await client.release.get({
                  spaceId: home,
                  id: schema.identifier("release").parse(followed.id),
              });

    return row.version;
}
