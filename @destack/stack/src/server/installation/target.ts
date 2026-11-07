import { Failure } from "@destack/error";
import { DeclarationReference } from "@destack/package/declare";
import { installableReleases, MAIN_DATABASE, StackDefinition } from "../../declare/index.ts";
import {
    and,
    type BlobStore,
    type DatabaseConnection,
    DatabaseState,
    eq,
    isNull,
    Snapshot,
} from "@destack/db";
import {
    Installation,
    installation,
    InstallationBindingSpec,
    type InstallationBindingState,
    InstallationKind,
    type InstallationSelection,
    ObjectDescription,
    type ObjectReconciliation,
    ObjectType,
    ObjectTypePool,
    PackageDescription,
} from "@destack/object";
import { PackageId, PackageRelease } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { RESOURCE_KIND, ResourceDescription } from "@destack/resource";
import {
    Digest,
    type DigestCrypto,
    Identifier,
    type JsonValue,
    present,
    schema,
    Version,
} from "@destack/schema";
import { type Call, ObjectTypeReference } from "@destack/sync";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { build, Build, type BuildSource } from "../../object/build.ts";
import accountPackage from "@destack/account/package";
import type { SpaceBuildStore, Registry } from "./build.ts";
import type { Forge } from "./follow.ts";
import { space } from "@destack/account/object";
import { StackEvaluation } from "../../object/stack.ts";
import { StackError } from "../../error/index.ts";
import { Binding } from "../../object/binding.ts";
import {
    InstallationRevision,
    installationRevision,
    type InstallationRollout,
    Rollout,
    rollout,
} from "../../object/rollout.ts";

/** What targeting and describing revisions read: builds, the registry, blobs and the forge. */
export interface RevisionOptions {
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
    /** The types the kernel declares, which packages implement and their descriptions refer to. */
    readonly kernel: readonly ObjectType[];
    /** The release the machine links of each package, which its spaces run before any revision. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
}

/** The build a selection resolves to: the space's build, its manifest and source. */
interface SelectedBuild {
    /** The space's build. */
    readonly build: Identifier<"build">;
    /** The digest of the build's manifest. */
    readonly manifest: Digest;
    /** Where the build comes from. */
    readonly source: BuildSource;
}

/** The revision a rollout targets: selecting its build, evaluating its stack, recording it. */
export const RolloutTarget = {
    /** Revise each installation: its selected or targeted build at its current bindings. */
    async revise(
        reconciliation: ObjectReconciliation,
        rows: readonly InstallationRollout[],
        options: RevisionOptions,
    ): Promise<void> {
        // read the revisions the installations target and their builds
        const { database, store } = reconciliation;
        const snapshot = Snapshot.live(database);
        const targetedIds = rows.flatMap((row) =>
            row.revisionId === null ? [] : [row.revisionId],
        );
        const targeted = await snapshot
            .read(installationRevision.table)
            .findMany({ where: { id: { in: targetedIds } } });
        const built = (
            await snapshot
                .read(build.table)
                .findMany({ where: { id: { in: targeted.map((each) => each.buildId) } } })
        ).map((each) => Build.of(each));

        // revise each one: its selected build, else its targeted one, at its bindings as they stand
        const calls: Call[] = [];
        for (const row of rows) {
            // take a release or tag selection's build once built, else the targeted revision's
            const revision = targeted.find((each) => each.id === row.revisionId);
            const current = built.find((each) => each.id === revision?.buildId);
            const { kind } = row.installation.selection;
            const selected =
                kind === "release" || kind === "tag"
                    ? await selectBuild(reconciliation, row, current, options)
                    : undefined;
            const chosen =
                selected ??
                (revision === undefined || current === undefined
                    ? undefined
                    : { build: current.id, manifest: Build.manifest(current) });
            if (chosen === undefined) {
                continue;
            }

            // describe a new build's revision, reusing the targeted one's for the same build
            const reader = await options.builds.contents(chosen.manifest);
            const described =
                revision !== undefined && revision.buildId === chosen.build
                    ? { dependencies: revision.dependencies, states: revision.states }
                    : await RolloutTarget.describe(
                          row.installation,
                          reader,
                          store.objects,
                          options,
                      );

            // evaluate a stack's export of it at its inputs
            const inputs =
                row.installation.export === null
                    ? undefined
                    : await RolloutTarget.inputs(snapshot, row.installation);
            const evaluation =
                inputs === undefined
                    ? undefined
                    : await RolloutTarget.evaluate(inputs, reader, options);
            calls.push(
                rollout.call("revise", {
                    ...rollout.target({ scope: row.scope, id: row.installationId }),
                    build: chosen.build,
                    ...described,
                    ...(evaluation === undefined ? {} : { evaluation }),
                }),
            );
        }

        // revise them in one transaction, each at most once per digest
        if (calls.length > 0) {
            await reconciliation.mutate(calls);
        }
    },

    /** Read the build an installation follows, else the linked release, else its selection. */
    async reader(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId: Identifier<"installation"> | undefined,
        options: Pick<RevisionOptions, "forge" | "registry" | "linked"> & {
            readonly builds: Pick<SpaceBuildStore, "contents" | "pull">;
        },
    ): Promise<BuildReader | undefined> {
        // read the installation's rollout, refusing several of the package unless one is named
        const installed = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { installationId: { in: installed.map((row) => row.id) } } });
        const [found] = rows;
        const [only] = installed.length === 1 ? installed : [];
        const linked = options.linked.get(packageId);
        const selection =
            installed.find((row) => row.id === found?.installationId)?.selection ?? only?.selection;
        if (rows.length > 1) {
            throw StackError.of(
                "INSTALLATION_AMBIGUOUS",
                "several installations in this space follow builds of this package: pick one",
            );
        }
        // answer none for a package the space neither installs nor links
        else if (installed.length === 0 && linked === undefined) {
            return undefined;
        }
        // read the revision's build
        else if (found?.revisionId !== undefined && found.revisionId !== null) {
            return InstallationRevision.reader(snapshot, found.revisionId, options.builds);
        }
        // read the release the machine links, which runs until a revision replaces it
        else if (linked !== undefined) {
            return options.builds.pull(options.registry, packageId, linked.manifest);
        }
        // resolve the release a planned installation selects as its rollout will
        else if (selection?.kind === "release" || selection?.kind === "tag") {
            const resolved = await selectedRelease(
                snapshot,
                { selection, packageId },
                undefined,
                options,
            );
            if (resolved !== undefined) {
                return options.builds.pull(options.registry, packageId, resolved.manifest);
            }
        }

        // refuse an installation following nothing yet
        throw StackError.of(
            "INSTALLATION_NOT_FOUND",
            "no installation in this space follows a build of this package",
        );
    },

    /** List the live installations with a matching selection, in one space or all. */
    async selecting(
        database: DatabaseConnection,
        spaceId: Identifier<"space"> | undefined,
        matches: (selection: InstallationSelection) => boolean,
    ): Promise<
        {
            readonly id: Identifier<"installation">;
            readonly scope: string;
            readonly packageId: PackageId;
            readonly managerInstallationId: Identifier<"installation"> | null;
            readonly selection: InstallationSelection;
        }[]
    > {
        // read the live rollouts with their installations' selections, of the space if given
        const live = isNull(installation.table.deletedAt);
        const rows = await database
            .select({
                id: rollout.table.installationId,
                scope: rollout.table.scope,
                packageId: installation.table.packageId,
                managerInstallationId: installation.table.managerInstallationId,
                selection: installation.table.selection,
            })
            .from(rollout.table)
            .innerJoin(
                installation.table,
                and(
                    eq(installation.table.id, rollout.table.installationId),
                    eq(installation.table.scope, rollout.table.scope),
                ),
            )
            .where(spaceId === undefined ? live : and(eq(rollout.table.scope, spaceId), live));

        return rows.filter((each) => matches(each.selection));
    },

    /** Evaluate a build's stack at an export from its inputs, with the releases it reads. */
    async evaluate(
        inputs: Omit<StackEvaluation, "definition">,
        reader: BuildReader,
        options: {
            readonly builds: Pick<SpaceBuildStore, "read">;
            readonly registry: Pick<Registry, "locate">;
        },
    ): Promise<StackEvaluation> {
        // read the stack the build declares at the export, with the settings it places
        const definition = await StackDefinition.declared(
            reader,
            inputs.export,
            inputs.settings,
            (packageId, manifest) => options.builds.read(options.registry, packageId, manifest),
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

        // take the setting values of the space's stack while the installation follows its package
        const [owner] = await snapshot.read(space.table).findMany({
            where: { id: schema.identifier("space").parse(installed.scope) },
            limit: 1,
        });
        const stack = present(owner, `the space ${installed.scope}`).stack;
        const settings = stack.packageId === installed.packageId ? stack.settings : {};

        return { export: exported, parameters, settings };
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
                    .parse(await SettingValue.resolve(Snapshot.live(database), setting, target)),
            ]),
        );

        return Object.fromEntries(resolved);
    },

    /** Describe a build's revision of an installation: its dependencies and required states. */
    async describe(
        target: Pick<Installation, "name" | "capabilities" | "allowed">,
        reader: BuildReader,
        objects: readonly ObjectType[],
        options: Pick<RevisionOptions, "builds" | "registry" | "blobs" | "kernel">,
    ): Promise<Pick<InstallationRevision, "dependencies" | "states">> {
        // read the installable packages it depends on, at the releases it compiled against
        const { builds, registry, blobs } = options;
        const read = (dependency: PackageId, manifest: Digest) =>
            builds.read(registry, dependency, manifest);
        const dependencies = await installableReleases(reader, read);

        // describe the state each declaration requires: main's tables, an installation's pinned release, a resource's own
        const required = new Map<string, JsonValue>();
        const tables = await mainTables(target, reader, { dependencies }, objects, options);
        if (tables !== undefined) {
            required.set(MAIN_DATABASE, tables);
        }
        for (const [name, state] of await declaredStates(reader)) {
            required.set(name, state);
        }

        // write each state as a blob, keyed by its declaration
        const states = await Promise.all(
            [...required].map(
                async ([name, state]) => [name, await blobs.writeJson(state)] as const,
            ),
        );

        return { dependencies, states: Object.fromEntries(states) };
    },

    /** Describe the call that records a space's build and its stack evaluation as a revision. */
    async create(
        target: Pick<Installation, "id" | "scope" | "packageId" | "generation">,
        evaluated: { readonly id: Identifier<"build">; readonly manifest: Digest },
        described: Pick<InstallationRevision, "dependencies" | "states">,
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
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            bindings: [...bindings],
            generation: target.generation,
            digest: await InstallationRevision.digest(
                evaluated.manifest,
                evaluation,
                bindings,
                crypto,
            ),
        });
    },
};

/** Resolve a selection to its build now, recording a pulled release, none until built. */
async function selectBuild(
    reconciliation: ObjectReconciliation,
    row: InstallationRollout,
    current: Build | undefined,
    options: RevisionOptions,
): Promise<SelectedBuild | undefined> {
    // resolve a release or tag to its release, keeping the targeted build of that manifest
    const { selection, packageId } = row.installation;
    const snapshot = Snapshot.live(reconciliation.database);
    if (selection.kind === "release" || selection.kind === "tag") {
        const selected = await selectedRelease(snapshot, row.installation, current, options);
        if (selected === undefined) {
            return undefined;
        }
        const source = current?.source;
        if (
            source?.kind === "release" &&
            current !== undefined &&
            Build.manifest(current) === selected.manifest
        ) {
            return { manifest: selected.manifest, source, build: current.id };
        }

        // pull the release and record it as the space's build once, at the version its manifest has
        const { builds, registry } = options;
        const { manifest } = selected;
        const pulled = await builds.pull(registry, packageId, manifest);
        const released = {
            kind: "release" as const,
            package: packageId,
            version: pulled.manifest.package.version,
            manifest,
        };
        const recorded = await Build.release(
            snapshot,
            (calls) => reconciliation.mutate(calls),
            row.scope,
            released,
        );

        return { build: recorded.id, manifest, source: released };
    }
    // take the succeeded build of the commit a repository reference points at
    else if (selection.kind === "repository") {
        const { forge } = options;
        const commit =
            forge === null
                ? undefined
                : await forge.commit(
                      reconciliation.database,
                      selection.repository,
                      selection.reference,
                  );
        if (commit === undefined) {
            return undefined;
        }
        const source = {
            kind: "commit" as const,
            repository: selection.repository,
            commit,
            directory: selection.directory,
        };
        const found = await Build.find(snapshot, row.scope, source);

        return found?.state === "succeeded"
            ? { manifest: Build.manifest(found), source, build: found.id }
            : undefined;
    }
    // take the checkout's newest succeeded build
    else {
        const newest = (
            await snapshot.read(build.table).findMany({
                where: { scope: row.scope, state: "succeeded", expiredAt: { isNull: true } },
            })
        )
            .map((entry) => Build.of(entry))
            .filter(
                (entry) =>
                    entry.source.kind === "checkout" &&
                    entry.source.machine === selection.machine &&
                    entry.source.checkout === selection.checkout &&
                    entry.source.directory === selection.directory,
            )
            .toSorted((left, right) => right.createdAt - left.createdAt)
            .at(0);

        return newest === undefined
            ? undefined
            : { manifest: Build.manifest(newest), source: newest.source, build: newest.id };
    }
}

/** Resolve a release or tag selection to its version and manifest, none while its tag is unset. */
async function selectedRelease(
    snapshot: Pick<Snapshot, "read">,
    installed: Pick<Installation, "selection" | "packageId">,
    current: Build | undefined,
    options: Pick<RevisionOptions, "forge" | "registry"> & {
        readonly builds: Pick<SpaceBuildStore, "pull">;
    },
): Promise<{ readonly version: string; readonly manifest: Digest } | undefined> {
    // take a release's version and its pinned manifest, else the registry's
    const { selection, packageId } = installed;
    const { forge, builds, registry } = options;
    if (selection.kind === "release") {
        const manifest =
            selection.manifest ?? (await published(registry, packageId, selection.version));

        return manifest === undefined ? undefined : { version: selection.version, manifest };
    }
    // take the version a tag points at in the forge's replica
    else if (selection.kind === "tag" && forge !== null) {
        const version = await forge.version(snapshot, packageId, selection.tag);

        return version === undefined
            ? undefined
            : { version, manifest: await registry.find(packageId, version) };
    }
    // keep the targeted release of a tag on a machine following no forge
    else if (selection.kind === "tag" && current?.source.kind === "release") {
        return { version: current.source.version, manifest: Build.manifest(current) };
    }
    // resolve a tag at the registry once, on a machine following no forge
    else if (selection.kind === "tag") {
        const manifest = await published(registry, packageId, selection.tag);
        if (manifest === undefined) {
            return undefined;
        }
        const version = (await builds.pull(registry, packageId, manifest)).manifest.package.version;

        return { version, manifest };
    }
    // follow nothing else here
    else {
        return undefined;
    }
}

/** Find a published release's manifest by version or tag, absent while none is published. */
async function published(
    registry: Pick<Registry, "find">,
    packageId: PackageId,
    release: string,
): Promise<Digest | undefined> {
    return await registry.find(packageId, release).catch((error: unknown) => {
        // answer absent for the forge's missing release, and fail on any other refusal
        if (Failure.of(error).reason === "RELEASE_NOT_FOUND") {
            return undefined;
        }
        throw error;
    });
}

/** Describe the tables a build's types keep in the space's main database, none without types. */
async function mainTables(
    target: Pick<Installation, "name" | "capabilities" | "allowed">,
    reader: BuildReader,
    revision: Pick<InstallationRevision, "dependencies">,
    objects: readonly ObjectType[],
    options: Pick<RevisionOptions, "builds" | "registry" | "kernel">,
): Promise<DatabaseState | undefined> {
    // declare no tables for a package declaring no object types
    const packageId = PackageManifest.package(reader.manifest).id;
    const owned = await ObjectDescription.read(reader);
    if (owned.length === 0) {
        return undefined;
    }

    // read the types the builds of the packages it needs declare, reading none of a package the pool knows
    const pooled = ObjectType.known(objects, [
        ...Object.values(accountPackage.objects),
        ...options.kernel,
    ]);
    const known = new Set(objects.map((type) => ObjectTypeReference.key(type.typeReference)));
    const pooledPackages = new Set(pooled.map((type) => type.package.id));
    const description = await PackageDescription.of(reader);
    const needed = new Set(
        Rollout.needed(target, revision, description).map((entry) => entry.package.id),
    );
    const depended = await Promise.all(
        revision.dependencies
            .filter(
                (dependency) =>
                    needed.has(dependency.package.id) && !pooledPackages.has(dependency.package.id),
            )
            .map(async (dependency) => {
                const { id } = dependency.package;
                const read = await options.builds.read(options.registry, id, dependency.manifest);

                return await ObjectDescription.read(read);
            }),
    );
    const unserved = depended.flat().filter((each) => !known.has(ObjectDescription.key(each)));

    // declare the package's own types against the served ones and those of its dependencies
    const declared = ObjectTypePool.of(pooled).declare([...unserved, ...owned]);

    // describe its durable and events objects' tables, placing the other packages' they refer to
    const others = objects.filter((type) => type.package.id !== packageId);
    const tables = new Set(
        [...others, ...declared]
            .filter((type) => type.storage === "durable" || type.storage === "events")
            .flatMap((type) => type.tables),
    );

    return DatabaseState.of([...tables], { package: packageId });
}

/** Describe the state each resource declaration of a build requires of its target, by name. */
async function declaredStates(reader: BuildReader): Promise<Map<string, JsonValue>> {
    // read the build's resource declarations and its exact dependencies once an installation of another package is declared
    const own = PackageManifest.package(reader.manifest);
    const resources = (await reader.declarations())
        .filter((declaration) => RESOURCE_KIND.has(declaration))
        .map((declaration) =>
            ResourceDescription.catchall(schema.json()).parse(declaration.description),
        );
    const callees = resources
        .filter((resource) => resource.kind === InstallationKind.name)
        .map((resource) => InstallationBindingSpec.parse(resource.spec).package);
    const dependencies = callees.some((callee) => callee !== own.id)
        ? Object.values(await reader.dependencies())
        : [];

    // pin an installation to the release of its package the reader compiled against, else take a resource's own state
    const states = new Map<string, JsonValue>();
    for (const { name, kind, spec, ...state } of resources) {
        // pin an installation's callee release
        if (kind === InstallationKind.name) {
            const callee = InstallationBindingSpec.parse(spec).package;
            const release =
                callee === own.id
                    ? own.version
                    : dependencies.find(
                          (entry) => "id" in entry.package && entry.package.id === callee,
                      )?.package.version;
            if (release === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `the reader binds installation ${name} of ${callee} without depending on it`,
                );
            }
            const pinned: InstallationBindingState = { release: Version.parse(release) };
            states.set(name, pinned);
        }
        // take the state another resource requires, none for a stateless one
        else if (Object.keys(state).length > 0) {
            states.set(name, state);
        }
    }

    return states;
}
