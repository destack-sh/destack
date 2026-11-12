import { Failure } from "@destack/error";
import { DeclarationReference } from "@destack/package/declare";
import { installableBuilds, MAIN_DATABASE, StackDefinition } from "../../declare/index.ts";
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
    InstallationSelection,
    ObjectDescription,
    type ObjectReconciliation,
    ObjectType,
    ObjectTypePool,
    PackageDescription,
} from "@destack/object";
import { type DependencyRelease, PackageId, PackageRelease } from "@destack/package";
import { type BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
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
import { type Call, ObjectTypeReference } from "@destack/sync";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { build, Build, type BuildSource } from "../../object/build.ts";
import type { SpaceBuildStore, Registry } from "./build.ts";
import type { Forge } from "./follow.ts";
import type { Binder } from "../resource/binding.ts";
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
    /** The kinds of objects bindings target. */
    readonly binder: Binder;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
    /** The types every space knows beside those it serves, which descriptions refer to. */
    readonly known: readonly ObjectType[];
    /** The release the machine links of each package, which its spaces run before any revision. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The releases the machine ships, by package. */
    readonly shipped: ReadonlyMap<PackageId, PackageRelease>;
    /** The locks of the releases the machine runs in its process: those it ships or links. */
    readonly trusted: ReadonlySet<Digest>;
}

/** The build a selection resolves to: the space's build, its lock and source. */
interface SelectedBuild {
    /** The space's build. */
    readonly build: Identifier<"build">;
    /** The digest of the build's lock. */
    readonly lock: Digest;
    /** Where the build comes from. */
    readonly source: BuildSource;
}

/** The revision a rollout targets: selecting its build, evaluating its stack, recording it. */
export const RolloutTarget = {
    /** Read the release this machine provides of a package, which one following the host's runs. */
    host(
        packageId: PackageId,
        options: Pick<RevisionOptions, "linked" | "shipped">,
    ): PackageRelease {
        const provided = options.linked.get(packageId) ?? options.shipped.get(packageId);
        if (provided === undefined) {
            throw StackError.of(
                "TARGET_BLOCKED",
                `cannot follow the host's release of '${packageId}': this machine provides none`,
            );
        }

        return provided;
    },

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
            const selected = InstallationSelection.isRelease(row.installation.selection)
                ? await selectBuild(reconciliation, row, current, options)
                : undefined;
            const chosen =
                selected ??
                (revision === undefined || current === undefined
                    ? undefined
                    : { build: current.id, lock: Build.lock(current) });
            // wait for an installation selecting a target of a kind no server here serves yet
            const selections = Object.values(row.installation.bindings).flatMap((each) =>
                Object.values(each),
            );
            const unserved = selections
                .map(({ target }) => target)
                .filter((target) => "id" in target && !options.binder.isBindable(target.id))
                .map((target) => target.type);
            if (unserved.length > 0) {
                const message = `waiting for a provider of ${[...new Set(unserved)].join(", ")} resources`;
                await reconciliation.observe(installation, row.installation, {
                    conditions: {
                        Bound: Rollout.waiting("WaitingForResources", message),
                        Ready: Rollout.waiting("WaitingForResources", message),
                    },
                });
                continue;
            }
            // pass over one with no build to revise yet
            else if (chosen === undefined) {
                continue;
            }
            // wait for a missing build
            else if (!(await options.builds.contains(chosen.lock))) {
                await reconciliation.observe(installation, row.installation, {
                    conditions: { Ready: Rollout.waitingForBuild(chosen.lock) },
                });
                continue;
            }

            // pass over a stack whose targeted revision has this build at these inputs, as stacks bind nothing
            const inputs =
                row.installation.export === null
                    ? undefined
                    : await RolloutTarget.inputs(snapshot, row.installation);
            const isRevised =
                inputs !== undefined &&
                revision !== undefined &&
                revision.buildId === chosen.build &&
                revision.digest ===
                    (await InstallationRevision.digest(
                        chosen.lock,
                        inputs,
                        [],
                        store.environment.crypto,
                    ));
            if (isRevised) {
                continue;
            }

            // describe a new build's revision, reusing the targeted one's for the same build
            const lock = await options.builds.lock(chosen.lock);
            const described =
                revision !== undefined && revision.buildId === chosen.build
                    ? { dependencies: revision.dependencies, states: revision.states }
                    : await RolloutTarget.describe(row.installation, lock, store.objects, options);

            // evaluate a stack's export of it at its inputs
            const evaluation =
                inputs === undefined
                    ? undefined
                    : await RolloutTarget.evaluate(inputs, lock, options);
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
        options: Pick<RevisionOptions, "forge" | "registry" | "linked" | "shipped"> & {
            readonly builds: Pick<SpaceBuildStore, "get" | "pull" | "pullRoot" | "root">;
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
        const followed = installed.find((row) => row.id === found?.installationId);
        const selection = followed?.selection ?? only?.selection;
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
        // read the revision's build
        else if (followed?.revisionId !== undefined && followed.revisionId !== null) {
            return InstallationRevision.reader(snapshot, followed.revisionId, options.builds);
        }
        // read the release the machine links, which runs until a revision replaces it
        else if (linked !== undefined) {
            return options.builds.pullRoot(packageId, linked.lock);
        }
        // read the release the machine provides for one following the host's
        else if (selection?.kind === "host") {
            return options.builds.pullRoot(packageId, RolloutTarget.host(packageId, options).lock);
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
                return options.builds.pullRoot(packageId, resolved.lock);
            }
        }

        // refuse an installation following nothing yet
        throw StackError.of(
            "INSTALLATION_NOT_FOUND",
            "installation in this space following a build of this package not found",
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

    /** Evaluate a locked build's stack at an export, with the builds its lock has. */
    async evaluate(
        inputs: Omit<StackEvaluation, "definition">,
        lock: Lock,
        options: {
            readonly builds: Pick<SpaceBuildStore, "cache" | "contents" | "get" | "read">;
            readonly registry: Pick<Registry, "store">;
        },
    ): Promise<StackEvaluation> {
        // take the lock's builds
        const { builds } = options;
        const linked = await Promise.all(
            [...builds.cache.linked.values()].map(async (release) => {
                const { root } = await builds.cache.store.lock(release.lock);
                const entry: DependencyRelease = {
                    kind: "destack",
                    package: release.package,
                    manifest: root,
                };

                return entry;
            }),
        );
        const releases = [
            ...linked,
            ...(await Lock.releases(lock, (digest) => builds.get(digest))),
        ];

        // read the stack the lock's root declares at the export
        const definition = await StackDefinition.declared(
            await builds.contents(lock.root),
            releases,
            inputs.export,
            inputs.settings,
            (packageId, manifest) => options.builds.read(packageId, manifest),
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
        lock: Lock,
        objects: readonly ObjectType[],
        options: Pick<RevisionOptions, "builds" | "registry" | "blobs" | "known">,
    ): Promise<Pick<InstallationRevision, "dependencies" | "states">> {
        // read the lock's root and the builds of the installable packages the lock has
        const { builds, blobs } = options;
        const reader = await builds.contents(lock.root);
        const entries = await Lock.entries(lock, (digest) => builds.get(digest));
        const read = (dependency: PackageId, manifest: Digest) => builds.read(dependency, manifest);
        const dependencies = await installableBuilds(entries, lock.root, read);

        // describe the state each declaration requires: main's tables, an installation's release, a resource's own
        const required = new Map<string, JsonValue>();
        const tables = await mainTables(target, reader, { dependencies }, objects, options);
        if (tables !== undefined) {
            required.set(MAIN_DATABASE, tables);
        }
        for (const [name, state] of await declaredStates(reader, entries)) {
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
        evaluated: { readonly id: Identifier<"build">; readonly lock: Digest },
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
            digest: await InstallationRevision.digest(evaluated.lock, evaluation, bindings, crypto),
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
    // take the release the machine provides for one following the host's, else resolve a release or tag
    const { selection, packageId } = row.installation;
    const snapshot = Snapshot.live(reconciliation.database);
    if (selection.kind === "host" || selection.kind === "release" || selection.kind === "tag") {
        const resolved =
            selection.kind === "host"
                ? RolloutTarget.host(packageId, options)
                : await selectedRelease(snapshot, row.installation, current, options);
        if (resolved === undefined) {
            return undefined;
        }

        // pull the release, then reuse its current build
        const { builds } = options;
        const { lock } = resolved;
        const pulled = await builds.pull(packageId, lock);
        const source = current?.source;
        if (source?.kind === "release" && current !== undefined && Build.lock(current) === lock) {
            return { lock, source, build: current.id };
        }

        // record the release
        const released = {
            kind: "release" as const,
            package: packageId,
            version: (await builds.get(pulled.root)).package.version,
            lock,
        };
        const recorded = await Build.release(
            snapshot,
            (calls) => reconciliation.mutate(calls),
            row.scope,
            released,
        );

        return { build: recorded.id, lock, source: released };
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
            ? { lock: Build.lock(found), source, build: found.id }
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
            : { lock: Build.lock(newest), source: newest.source, build: newest.id };
    }
}

/** Resolve a release or tag selection to its version and lock, none while its tag is unset. */
async function selectedRelease(
    snapshot: Pick<Snapshot, "read">,
    installed: Pick<Installation, "selection" | "packageId">,
    current: Build | undefined,
    options: Pick<RevisionOptions, "forge" | "registry"> & {
        readonly builds: Pick<SpaceBuildStore, "get" | "pull">;
    },
): Promise<{ readonly version: string; readonly lock: Digest } | undefined> {
    // take a release's version and the registry's lock of it
    const { selection, packageId } = installed;
    const { forge, builds, registry } = options;
    if (selection.kind === "release") {
        const lock = await published(registry, packageId, selection.version);

        return lock === undefined ? undefined : { version: selection.version, lock };
    }
    // take the version a tag points at in the forge's replica
    else if (selection.kind === "tag" && forge !== null) {
        const version = await forge.version(snapshot, packageId, selection.tag);

        return version === undefined
            ? undefined
            : { version, lock: await registry.find(packageId, version) };
    }
    // keep the targeted release of a tag on a machine following no forge
    else if (selection.kind === "tag" && current?.source.kind === "release") {
        return { version: current.source.version, lock: Build.lock(current) };
    }
    // resolve a tag at the registry once, on a machine following no forge
    else if (selection.kind === "tag") {
        const lock = await published(registry, packageId, selection.tag);
        if (lock === undefined) {
            return undefined;
        }
        const version = (await builds.get((await builds.pull(packageId, lock)).root)).package
            .version;

        return { version, lock };
    }
    // follow nothing else here
    else {
        return undefined;
    }
}

/** Find a published release's lock by version or tag, absent while none is published. */
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
    options: Pick<RevisionOptions, "builds" | "registry" | "known">,
): Promise<DatabaseState | undefined> {
    // declare no tables for a package declaring no object types
    const packageId = PackageManifest.package(reader.manifest).id;
    const owned = await ObjectDescription.read(reader);
    if (owned.length === 0) {
        return undefined;
    }

    // read the types the builds of the packages it needs declare, reading none of a package the pool knows
    const pooled = ObjectType.known(objects, options.known);
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
                const read = await options.builds.read(id, dependency.manifest);

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
        // record the callee's version in the lock
        if (isInstallation) {
            const callee = InstallationBindingSpec.parse(spec).package;
            const release =
                callee === own.id
                    ? own.version
                    : Object.values(entries).find(
                          (entry) => entry.kind === "destack" && entry.package.id === callee,
                      )?.package.version;
            if (release === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `binding of installation '${name}' is invalid: the build binds it without requiring '${callee}'`,
                );
            }
            const recorded: InstallationBindingState = { release: Version.parse(release) };
            states.set(name, recorded);
        }
        // take the state another resource requires, none for a stateless one
        else if (Object.keys(state).length > 0) {
            states.set(name, state);
        }
    }

    return states;
}
