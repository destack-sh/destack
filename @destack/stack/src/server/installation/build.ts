import type { PackageFile } from "@destack/package/file";
import { type Caller, principal } from "@destack/access";
import { LATEST_TAG } from "@destack/registry/object";
import { and, eq, Projection, Snapshot } from "@destack/db";
import { SettingValue } from "@destack/setting/object";
import { buildRetention } from "../../setting/index.ts";
import { type BuildObject, type ImportClosure, BuildStore } from "@destack/build/store";
import type { Runtime } from "@destack/package/runtime";
import {
    type MethodContextOf,
    type NextOf,
    ObjectDescription,
    type ResultOf,
} from "@destack/object";
import { account, Space } from "@destack/account/object";
import { SpaceAddress } from "@destack/account/address";
import type { GrantRequest } from "@destack/account/authentication";
import stackPackage from "../../package.ts";
import type { MountFetch, ObjectStore } from "@destack/object/server";
import { BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
import { PackageId, type PackageRelease } from "@destack/package";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import {
    build,
    Build,
    BuildOutcome,
    BuildEnd,
    BuildSource,
    CodeIndex,
    installationRevision,
} from "../../object/index.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** The prefix of the root retaining a space's installed builds in a machine's cache. */
const SPACE_ROOT_PREFIX = "space:";

/** A checkout source of a build. */
export type CheckoutSource = Extract<BuildSource, { readonly kind: "checkout" }>;

/** A repository commit source of a build. */
export type CommitSource = Extract<BuildSource, { readonly kind: "commit" }>;

/** The build store a push or publish reads a lock and its builds from. */
export type PublishStore = Pick<
    BuildStore,
    "closure" | "contents" | "get" | "head" | "lock" | "putLock" | "read"
>;

/** The registry of published releases. */
export interface Registry {
    /** Find the digest of a package release's lock, by its version or a distribution tag. */
    readonly find: (packageId: PackageId, release: string) => Promise<Digest>;
    /** The releases the distribution ships, which a scope runs of a package it installs none of. */
    readonly distribution: readonly PackageRelease[];
    /** Push a stored lock and its builds to its account's forge. */
    readonly push: (
        store: PublishStore,
        lock: Digest,
        account: Identifier<"account">,
        signal?: AbortSignal,
    ) => Promise<void>;
    /** Pull a published lock into a store for some runtimes, refusing another package's root. */
    readonly pull: (
        store: Pick<BuildStore, "get" | "lock" | "pull">,
        packageId: PackageId,
        lock: Digest,
        runtimes: readonly Runtime[] | "all",
    ) => Promise<Lock>;
    /** Open a published lock's or build's objects at its package's forge, as a build store. */
    readonly store: (
        packageId: PackageId,
        root: Pick<BuildObject, "kind" | "digest">,
    ) => Promise<Pick<BuildStore, "read">>;
}

/** The options of the build server. */
export interface BuildOptions {
    /** The machine running the spaces' commit builds. */
    readonly buildRunner: Identifier<"machine"> | null;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The registry the space's builds are pushed to. */
    readonly registry: Registry;
    /** Publish each of an account's packages in a stored lock for a caller. */
    readonly publish: (
        store: PublishStore,
        lock: Digest,
        account: Identifier<"account">,
        tag: string,
        caller: Caller,
        requested: GrantRequest,
    ) => Promise<readonly PackageRelease[]>;
}

/** A machine's cache of the builds it runs: pulled releases and copies of its spaces' builds. */
export class BuildCache {
    /** The store with the cached builds. */
    readonly store: BuildStore;
    /** The runtimes the machine serves: its instances' server runtime and its views' browser. */
    readonly runtimes: readonly Runtime[];
    /** The builtins the machine links, by package, whose locks it runs in place of any other. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The published locks pulled with their runtime closures, by digest, each pulled once. */
    readonly #pulled = new Map<Digest, Promise<Lock>>();

    /** Cache builds in a store for the runtimes the machine serves and the builtins it links. */
    constructor(
        store: BuildStore,
        runtimes: readonly Runtime[],
        linked: ReadonlyMap<PackageId, PackageRelease>,
    ) {
        this.store = store;
        this.runtimes = runtimes;
        this.linked = linked;
    }

    /** Retain the locks a space runs and the builds they list, replacing its earlier set. */
    async retain(space: Identifier<"space">, locks: Iterable<Digest>): Promise<void> {
        await this.store.retain(`${SPACE_ROOT_PREFIX}${space}`, new Set(locks));
    }

    /** Forget the builds of every space but those the machine still stores. */
    async prune(spaces: readonly Identifier<"space">[]): Promise<void> {
        // forget each space root of a space no longer stored
        const kept = new Set(spaces.map((space) => `${SPACE_ROOT_PREFIX}${space}`));
        for (const name of (await this.store.roots()).keys()) {
            if (name.startsWith(SPACE_ROOT_PREFIX) && !kept.has(name)) {
                await this.store.forget(name);
            }
        }
    }

    /** Resolve a release to the lock the machine runs: a builtin's linked one, else its own. */
    resolve(packageId: PackageId, lock: Digest): Digest {
        return this.linked.get(packageId)?.lock ?? lock;
    }

    /** Pull what the machine's runtimes load of a published lock, with its runtime closures. */
    pull(
        registry: Pick<Registry, "pull" | "store">,
        packageId: PackageId,
        lock: Digest,
    ): Promise<Lock> {
        // pull a lock once
        lock = this.resolve(packageId, lock);
        const known = this.#pulled.get(lock);
        if (known !== undefined) {
            return known;
        }
        const pulling = registry.pull(this.store, packageId, lock, this.runtimes);
        this.#pulled.set(lock, pulling);
        pulling.catch(() => this.#pulled.delete(lock));

        return pulling;
    }

    /** Read the lock of each builtin the machine links, by package name. */
    async linkedLocks(): Promise<Map<string, Lock>> {
        const locks = await Promise.all(
            [...this.linked.values()].map(
                async (release) =>
                    [release.package.name, await this.store.lock(release.lock)] as const,
            ),
        );

        return new Map(locks);
    }
}

/** A space's build store: its runners' pushes, read through to its machine's cache and registry. */
export class SpaceBuildStore {
    /** The space's own builds, retained while its build rows reference them. */
    readonly space: BuildStore;
    /** The machine's cache of what it runs. */
    readonly cache: BuildCache;
    /** The registry the space's published builds come from, whole or a file at a time. */
    readonly registry: Pick<Registry, "pull" | "store">;
    /** The closure of each build mapped so far, by build, runtime and base path. */
    readonly #closures = new Map<string, Promise<ImportClosure>>();
    /** The builds opened so far, by manifest, each opened once as a build never changes. */
    readonly #opened = new Map<Digest, Promise<BuildReader>>();

    /** Store a space's builds in its blobs, reading through to the machine's cache and registry. */
    constructor(space: BuildStore, cache: BuildCache, registry: Pick<Registry, "pull" | "store">) {
        this.space = space;
        this.cache = cache;
        this.registry = registry;
    }

    /** Report whether the space or the machine stores a build or lock. */
    async contains(digest: Digest): Promise<boolean> {
        return (await this.space.contains(digest)) || (await this.cache.store.contains(digest));
    }

    /** Open a build the space stores, else the machine's copy, a missing file from the registry. */
    async contents(manifest: Digest): Promise<BuildReader> {
        // open each build once, forgetting a failed open
        const known = this.#opened.get(manifest);
        if (known !== undefined) {
            return known;
        }
        const opening = this.#open(manifest);
        this.#opened.set(manifest, opening);
        opening.catch(() => this.#opened.delete(manifest));

        return opening;
    }

    /** Open a build, fetching a file its store lacks once from its registry into the cache. */
    async #open(manifest: Digest): Promise<BuildReader> {
        // open the build where it is stored, an npm unit there alone
        const store = await this.store(manifest);
        const reader = await store.contents(manifest);
        const owner = reader.manifest.package;
        if (!("id" in owner)) {
            return reader;
        }

        // read each file from the store, else once from the build's registry
        const { crypto } = this.cache.store.environment;
        const stream: BuildReader["stream"] = async (path, signal) => {
            // read a file the store holds, else write it into the cache from the build's forge first
            const file = await reader.file(path);
            const [isStored] = await store.has([file.digest]);
            if (isStored !== true) {
                const source = await this.registry.store(owner.id, {
                    kind: "manifest",
                    digest: manifest,
                });
                await this.cache.store.write(source.read([{ kind: "file", digest: file.digest }]));
            }

            return await (
                isStored === true ? reader : await this.cache.store.contents(manifest)
            ).stream(path, signal);
        };

        return new BuildReader(reader.manifest, stream, crypto);
    }

    /** Read a build's manifest from the space, else from the machine. */
    async get(manifest: Digest): Promise<PackageManifest> {
        return (await this.store(manifest)).get(manifest);
    }

    /** Read the file a build attached at a path, from the space, else from the machine. */
    async referrer(manifest: Digest, path: string): Promise<PackageFile | undefined> {
        return (await this.store(manifest)).referrer(manifest, path);
    }

    /** Read one complete stored file, from the space, else from the machine. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        const [isSpace] = await this.space.has([file.digest]);
        const store = isSpace === true ? this.space : this.cache.store;

        return store.file(file);
    }

    /** Serve the mount taking the files, builds and lock a running build's runner pushes. */
    mount(): MountFetch {
        return async (request, admitted, _context, store) => {
            // read the build below the mount, refusing a caller acting for none of its runners
            const url = new URL(request.url);
            const [, space = "", built = ""] = url.pathname.split("/");
            const spaceId = schema.identifier("space").parse(space);
            const buildId = schema.identifier("build").parse(built);
            const [row] = await store.database
                .select({
                    id: build.table.id,
                    state: build.table.state,
                    runnerId: build.table.runnerId,
                })
                .from(build.table)
                .where(and(eq(build.table.scope, spaceId), eq(build.table.id, buildId)));
            const acting = [admitted.subject, ...admitted.actors].map((subject) => subject.id);
            if (
                row?.runnerId === undefined ||
                row.runnerId === null ||
                !acting.includes(row.runnerId)
            ) {
                throw StackError.of(
                    "BUILD_NOT_FOUND",
                    `build '${buildId}' running for the caller not found`,
                );
            }
            requireRunning(row);

            // take pushes of its files, builds and lock into the space's builds, serving no pulls
            const mount = new URL(`${url.origin}/${spaceId}/${buildId}/`);

            return await this.space.fetch(request, mount, {
                pull: async () => {
                    throw StackError.of("BUILD_NOT_FOUND", "build for a runner to pull not found");
                },
                push: async () => {},
            });
        };
    }

    /** Map a lock's imports on a runtime below a base path, builtins at their linked roots. */
    imports(lock: Digest, runtime: Runtime, base: string): Promise<ImportClosure> {
        // map each lock once per runtime and base
        const key = `${lock} ${runtime} ${base}`;
        let found = this.#closures.get(key);
        if (found === undefined) {
            found = this.#imports(lock, runtime, base);
            this.#closures.set(key, found);
            found.catch(() => this.#closures.delete(key));
        }

        return found;
    }

    /** Map a lock's imports on a runtime below a base path, with the builtins the machine links. */
    async #imports(lock: Digest, runtime: Runtime, base: string): Promise<ImportClosure> {
        // read the lock and the builtins the machine links
        const read = await this.lock(lock);
        const linked = await this.cache.linkedLocks();

        return await this.cache.store.imports(read, runtime, base, linked, (digest) =>
            this.get(digest),
        );
    }

    /** Open a lock's root build: the space's own, else the machine's copy. */
    async root(lock: Digest): Promise<BuildReader> {
        return await this.contents((await this.lock(lock)).root);
    }

    /** Bring a lock into the machine's cache, then open its root build. */
    async pullRoot(packageId: PackageId, lock: Digest): Promise<BuildReader> {
        return await this.contents((await this.pull(packageId, lock)).root);
    }

    /** Read a lock the space stores, else the machine's copy. */
    async lock(digest: Digest): Promise<Lock> {
        return (await this.space.contains(digest))
            ? await this.space.lock(digest)
            : await this.cache.store.lock(digest);
    }

    /** Read a build in place to inspect it: the space's own, else the cached or published one. */
    async read(packageId: PackageId, manifest: Digest): Promise<BuildReader> {
        // read the space's own build, else one the machine caches or the registry publishes
        if (await this.space.contains(manifest)) {
            return await this.space.contents(manifest);
        }

        // pull a published build's manifest and lists once
        const linked = this.cache.linked.get(packageId);
        const resolved =
            linked === undefined ? manifest : (await this.cache.store.lock(linked.lock)).root;
        if ((await this.cache.store.find(resolved)) === undefined) {
            const root = { kind: "manifest", digest: resolved } as const;
            await this.cache.store.pull(await this.registry.store(packageId, root), root, []);
        }

        return await this.contents(resolved);
    }

    /** Bring a lock into the machine's cache to run it: the space's own, else a published one. */
    async pull(packageId: PackageId, lock: Digest): Promise<Lock> {
        // pull a published lock with its runtime closures
        if (!(await this.space.contains(lock))) {
            return await this.cache.pull(this.registry, packageId, lock);
        }

        // pull the space's own lock from the space's store
        await this.cache.store.pull(
            this.space,
            { kind: "lock", digest: lock },
            this.cache.runtimes,
        );

        return await this.cache.store.lock(lock);
    }

    /** Take the build a stored build ends with, which its runner pushed. */
    async adopt(end: schema.Infer<typeof BuildEnd>): Promise<schema.Infer<typeof BuildOutcome>> {
        // store nothing of a failed build
        if (end.kind === "failed") {
            return end;
        }

        // require the lock its runner pushed
        if (!(await this.space.contains(end.lock))) {
            throw StackError.of(
                "BUILD_NOT_FOUND",
                `lock '${end.lock}' not found: it was not pushed to the space`,
            );
        }
        await this.space.attach((await this.space.lock(end.lock)).root, CodeIndex.file(end.code));

        return { kind: "stored", lock: end.lock };
    }

    /** Delete the space's builds as it leaves the machine, with their attached graph indexes. */
    async clear(): Promise<void> {
        await this.space.clear();
    }

    /** Declare the projection expiring the space's builds past its retention as builds change. */
    retention(store: ObjectStore): Projection {
        const { database, scope } = store;

        return new Projection(database, {
            name: "retention",
            tables: [build.table, installationRevision.table],
            keys: async () => [scope],
            all: async () => [scope],
            project: async () =>
                this.retain(
                    store,
                    (space) =>
                        SettingValue.resolve(Snapshot.live(database), buildRetention, {
                            scope: space,
                        }),
                    store.clock.now(),
                ),
        });
    }

    /** Expire the space's builds its retention drops, leaving their blobs to the collection. */
    async retain(
        objects: Pick<ObjectStore, "database" | "as" | "mutate" | "environment">,
        retention: (space: string) => Promise<number>,
        now: number,
    ): Promise<void> {
        const { expired } = await Build.retention(objects.database, retention);
        if (expired.length > 0) {
            await objects.mutate(
                objects.as(build),
                {
                    id: RequestId.create(objects.environment),
                    calls: expired.map((row) =>
                        build.call("update", { ...build.target(row), expiredAt: now }),
                    ),
                },
                { component: "retention" },
            );
        }
    }

    /** Pick the store with a build: the space's when it has it, else the machine's cache. */
    async store(manifest: Digest): Promise<BuildStore> {
        return (await this.space.contains(manifest)) ? this.space : this.cache.store;
    }
}

/** Implement builds: a creator creates and cancels a build. */
export function implementBuild(options: BuildOptions) {
    return build.implement({
        methods: {
            create: {
                // relate a compiled build's runner, which its modules and symbols use
                creation: (context) => ({
                    relationships:
                        context.input.source.kind === "release"
                            ? []
                            : [
                                  {
                                      relation: "runner",
                                      subject: runnerOf(context, options.buildRunner),
                                  },
                              ],
                }),
                handle: async (context, next) => {
                    // record a release its caller pulled, refusing another package or version, with nothing to run
                    const spaceId = schema.identifier("space").parse(context.scope);
                    const source = BuildSource.parse(context.input.source);
                    if (source.kind === "release") {
                        const { lock } = source;
                        const { store } = options.builds.cache;
                        if (!(await store.contains(lock))) {
                            throw StackError.of(
                                "RELEASE_BLOCKED",
                                `cannot create a build of release '${lock}': pull it into the build store first`,
                            );
                        }
                        const root = await store.get((await store.lock(lock)).root);
                        const built = PackageManifest.package(root);
                        if (built.id !== source.package || built.version !== source.version) {
                            throw StackError.of(
                                "MANIFEST_INVALID",
                                `lock '${lock}' is invalid: its root builds ${built.id} '${built.version}', not ${source.package} '${source.version}'`,
                            );
                        }

                        return next(
                            context.with({
                                input: {
                                    ...context.input,
                                    state: "succeeded",
                                    lock,
                                    outputs: Object.keys(root.outputs),
                                    startedAt: context.now,
                                    finishedAt: context.now,
                                },
                            }),
                        );
                    }

                    // refuse a second live build of a commit's package directory
                    const existing =
                        source.kind === "commit"
                            ? await Build.find(context, spaceId, source)
                            : undefined;
                    if (existing !== undefined) {
                        throw StackError.of(
                            "BUILD_EXISTS",
                            `build '${existing.id}' of the same commit already exists`,
                        );
                    }

                    // compile it on the runner its source selects, keyed by its checkout
                    const runner = runnerOf(context, options.buildRunner);

                    return next(
                        context.with({
                            input: {
                                ...context.input,
                                ...ObjectReference.columns("runner", runner),
                                ...(source.kind === "commit"
                                    ? { checkout: Build.checkout(source) }
                                    : {}),
                            },
                        }),
                    );
                },
            },
            cancel,
            start,
            publish: {
                // publish a succeeded build and its account's unpublished dependencies for the caller before the transaction, asking their person for the account's publish
                prepare: async (context) => ({
                    releases: [
                        ...(await options.publish(
                            options.builds.space,
                            Build.lock(context.target),
                            context.input.account,
                            context.input.tag ?? LATEST_TAG,
                            context.caller,
                            await publishRequest(context),
                        )),
                    ],
                }),
                handle: async (context) => context.prepared,
            },
            push: {
                // push a succeeded build's lock and its builds to its package's forge before the transaction
                prepare: async (context) => {
                    const lock = Build.lock(context.target);
                    await options.registry.push(options.builds.space, lock, context.input.account);

                    return { lock };
                },
                handle: async (context) => context.prepared,
            },
            progress: async (context) => {
                // record the phase the runner reports
                const { target } = context;
                requireRunning(target);
                await context.update(reported(target, context.input.progress));

                return {};
            },
            finish: {
                // take a succeeded build's pushed manifest before the transaction
                prepare: (context) => options.builds.adopt(context.input.end),
                handle: async (context) => {
                    // finish a running build with its stored manifest or its failure
                    requireRunning(context.target);
                    await context.update(ended(context.prepared, context.now));

                    return {};
                },
            },
        },
    });
}

/** Start a pending build. */
async function start(
    context: MethodContextOf<typeof build, "start">,
): Promise<ResultOf<typeof build, "start">> {
    // refuse a build that is not pending
    const { target } = context;
    if (target.state !== "pending") {
        throw StackError.of(
            "BUILD_BLOCKED",
            `cannot start build '${target.id}': it is '${target.state}'`,
        );
    }

    // record it running
    await context.update({ state: "running", phase: "preparing", startedAt: context.now });

    return {};
}

/** Cancel a pending or running build for its runner to abort. */
async function cancel(
    context: MethodContextOf<typeof build, "cancel">,
    next: NextOf<typeof build, "cancel">,
): Promise<ResultOf<typeof build, "cancel">> {
    // refuse a finished build
    const { target } = context;
    if (Build.isFinished(target)) {
        throw StackError.of(
            "BUILD_BLOCKED",
            `cannot cancel build '${target.id}': it is '${target.state}'`,
        );
    }

    return next(
        context.with({
            input: { ...context.input, state: "cancelled", phase: null, finishedAt: context.now },
        }),
    );
}

/** Read the fields a runner's reported phase records. */
function reported(
    target: Pick<Build, "source">,
    progress: MethodContextOf<typeof build, "progress">["input"]["progress"],
) {
    // record the phase alone while storing
    if (progress.phase !== "building") {
        return { phase: "storing" as const };
    }

    // record the outputs and a checkout's commit while building
    const isCommitted = target.source.kind === "checkout" && progress.commit !== undefined;

    return {
        phase: "building" as const,
        outputs: [...progress.outputs],
        ...(isCommitted ? { commit: progress.commit } : {}),
    };
}

/** Read the fields a finished build records: its failure, or the digest of its stored lock. */
function ended(adopted: schema.Infer<typeof BuildOutcome>, now: number) {
    // record a failed build's failure
    if (adopted.kind === "failed") {
        return { state: "failed" as const, phase: null, failure: adopted.failure, finishedAt: now };
    }
    // record a stored build's lock
    else {
        return {
            state: "succeeded" as const,
            phase: null,
            lock: adopted.lock,
            finishedAt: now,
        };
    }
}

/** Read the machine running a new build. */
function runnerOf(
    context: Pick<MethodContextOf<typeof build, "create">, "input">,
    buildRunner: Identifier<"machine"> | null,
): ObjectReference {
    const { source } = context.input;

    // run a checkout's build on its machine
    if (source.kind === "checkout") {
        return principal.machine.reference(Scope.universe.id, source.machine);
    }
    // run a commit's build on the space's build runner
    else if (buildRunner !== null) {
        return principal.machine.reference(Scope.universe.id, buildRunner);
    }
    // refuse a commit's build no runner serves
    else {
        throw StackError.of(
            "BUILD_RUNNER_BLOCKED",
            "cannot build the commit: this space has no build runner",
        );
    }
}

/** Refuse a build that is not running. */
function requireRunning(target: Pick<Build, "id" | "state">): void {
    if (target.state !== "running") {
        throw StackError.of(
            "BUILD_BLOCKED",
            `cannot update build '${target.id}': it is '${target.state}', not running`,
        );
    }
}

/** Describe the grant the space's stack asks its caller's person for to publish. */
async function publishRequest(
    context: Pick<
        MethodContextOf<typeof build, "publish">,
        "scope" | "environment" | "read" | "input"
    >,
): Promise<GrantRequest> {
    // read the space, which asks for itself, and its account's replica, which has its handle
    const spaceId = schema.identifier("space").parse(context.scope);
    const placed = await Space.read(context, spaceId);
    const owner = present(
        await context.read(account).findFirst({ where: { id: placed.scope } }),
        `the account of ${spaceId}`,
    );

    // ask for the publish on the account by its identifier, as the account's type describes it
    const description = ObjectDescription.describe(ObjectDescription.of(account), "publish");

    return {
        delegate: principal.space.reference(Scope.universe.id, spaceId),
        package: stackPackage.name,
        space: SpaceAddress.of({ space: placed.name, handle: owner.handle }),
        permissions: [
            {
                ...account.permission("publish"),
                objectId: context.input.account,
                within: "account",
                description,
            },
        ],
    };
}
