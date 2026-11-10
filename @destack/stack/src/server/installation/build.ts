import type { PackageFile } from "@destack/package/file";
import { type Caller, principal } from "@destack/access";
import { LATEST_TAG } from "@destack/registry/object";
import { and, eq, Projection, Snapshot } from "@destack/db";
import { SettingValue } from "@destack/setting/object";
import { buildRetention } from "../../setting/index.ts";
import { type ImportClosure, type ManifestRead, BuildStore } from "@destack/build/store";
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
import type { AdapterFetch, ObjectStore } from "@destack/object/server";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import { DependencyRelease, PackageId, type PackageRelease } from "@destack/package";
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

/** The registry of published releases. */
export interface Registry {
    /** Find the digest of a package release's manifest, by its version or a distribution tag. */
    readonly find: (packageId: PackageId, release: string) => Promise<Digest>;
    /** The releases the distribution ships, which a scope runs of a package it installs none of. */
    readonly distribution: readonly PackageRelease[];
    /** Push a stored build with its npm units to its forge, answering its digest. */
    readonly push: (
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
        signal?: AbortSignal,
    ) => Promise<Digest>;
    /** Locate the files of a package's published build by its manifest's digest. */
    readonly locate: (
        packageId: PackageId,
        manifest: Digest,
    ) => Promise<{
        readonly location: PackageLocation;
        readonly fetch: (input: URL, init: RequestInit) => Promise<Response>;
    }>;
}

/** The options of the build server. */
export interface BuildOptions {
    /** The machine running the spaces' commit builds. */
    readonly buildRunner: Identifier<"machine"> | null;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: SpaceBuildStore;
    /** The registry the space's builds are pushed to. */
    readonly registry: Registry;
    /** Publish a stored build and its account's unpublished dependencies for a caller. */
    readonly publish: (
        store: Pick<BuildStore, "contents" | "units">,
        manifest: Digest,
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
    /** The builtins the machine links, by package, whose builds it loads in place of any other. */
    readonly linked: ReadonlyMap<PackageId, PackageRelease>;
    /** The published builds opened in place, by manifest, each opened once. */
    readonly #opened = new Map<Digest, Promise<BuildReader>>();
    /** The published builds pulled with their runtime closures, by manifest, each pulled once. */
    readonly #pulled = new Map<Digest, Promise<BuildReader>>();

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

    /** Retain the builds a space runs and what they reference, replacing its earlier set. */
    async root(space: Identifier<"space">, manifests: Iterable<Digest>): Promise<void> {
        await this.store.root(`${SPACE_ROOT_PREFIX}${space}`, new Set(manifests));
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

    /** Resolve a release to the build the machine loads: a builtin's linked build, else its own. */
    resolve(packageId: PackageId, manifest: Digest): Digest {
        return this.linked.get(packageId)?.manifest ?? manifest;
    }

    /** Read a build in place, the cached one else the published one, without copying its files. */
    async read(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // take the cached build, else the published one opened once where the registry locates it
        manifest = this.resolve(packageId, manifest);
        if (await this.store.contains(manifest)) {
            return await this.store.contents(manifest);
        }
        let opened = this.#opened.get(manifest);
        if (opened === undefined) {
            opened = registry
                .locate(packageId, manifest)
                .then(({ location, fetch }) =>
                    BuildReader.open(location, fetch, this.store.environment.crypto),
                );
            this.#opened.set(manifest, opened);

            // forget a failed open, so a later read tries again
            opened.catch(() => this.#opened.delete(manifest));
        }

        return await opened;
    }

    /** Pull what the machine's runtimes load of a published build, with its runtime closures. */
    pull(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // pull a build once, as its files and closure stay cached while it is served, forgetting a failed pull
        manifest = this.resolve(packageId, manifest);
        const known = this.#pulled.get(manifest);
        if (known !== undefined) {
            return known;
        }
        const pulling = this.#pullClosure(registry, packageId, manifest);
        this.#pulled.set(manifest, pulling);
        pulling.catch(() => this.#pulled.delete(manifest));

        return pulling;
    }

    /** Pull a build whole with what its emitted outputs' runtime closures pin. */
    async #pullClosure(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // locate each build once, and read its manifest from the cache, else where it is located
        const located = new Map<Digest, Promise<Located>>();
        const locate = (digest: Digest, at: PackageId) => {
            const found = located.get(digest) ?? registry.locate(at, digest);
            located.set(digest, found);

            return found;
        };
        const read: ManifestRead = async (digest, at) => {
            // read a stored build's manifest, else open the located build
            const stored = await this.store.find(digest);
            if (stored !== undefined) {
                return stored;
            }
            const { location, fetch } = await locate(digest, at);

            return (await BuildReader.open(location, fetch, this.store.environment.crypto))
                .manifest;
        };

        // copy the release whole, as its space reads its declarations and serves its files
        if (!(await this.store.contains(manifest))) {
            const { location, fetch } = await locate(manifest, packageId);
            await this.store.copy(location, fetch);
        }

        // pull what each emitted output the machine runs pins, as far as its runtime closure reaches
        const reader = await this.store.contents(manifest);
        const emitted = Object.values(reader.manifest.outputs).filter(
            (output) => output.emit && this.runtimes.includes(output.runtime),
        );
        for (const output of emitted) {
            const closure = await this.store.closure(manifest, packageId, output.runtime, read);
            const pulled = new Set<Digest>([manifest]);
            const pulling = { runtime: output.runtime, closure, read, locate, pulled };
            const pinned = Object.values(output.dependencies).filter((dependency) =>
                DependencyRelease.hasManifest(dependency),
            );
            for (const dependency of pinned) {
                if (closure.has(dependency.manifest)) {
                    const at = dependency.kind === "destack" ? dependency.package.id : packageId;
                    await this.#pull({ manifest: dependency.manifest, packageId: at }, pulling);
                }
            }
        }

        return reader;
    }

    /** Copy a build of a space into the cache whole. */
    async adopt(source: BuildReader, manifest: Digest): Promise<BuildReader> {
        if (!(await this.store.contains(manifest))) {
            await this.store.put(source);
        }

        return await this.store.contents(manifest);
    }

    /** Pull a closure build's output on a runtime after the builds it pins, visiting each once. */
    async #pull(
        release: { readonly manifest: Digest; readonly packageId: PackageId },
        pulling: Pulling,
    ): Promise<void> {
        // visit each build once, leaving an output the cache has, whose pinned builds it has too
        const { runtime, closure, read, locate, pulled } = pulling;
        if (pulled.has(release.manifest)) {
            return;
        }
        pulled.add(release.manifest);
        const recorded = present(await read(release.manifest, release.packageId), release.manifest);
        const [name, output] = present(
            Object.entries(recorded.outputs).find(([, each]) => each.runtime === runtime),
            `the ${runtime} output of ${release.manifest}`,
        );
        if (await this.store.containsOutput(release.manifest, name)) {
            return;
        }

        // pull the builds it pins first, an npm unit found at its dependent's registry
        const pinned = Object.values(output.dependencies).filter((dependency) =>
            DependencyRelease.hasManifest(dependency),
        );
        for (const dependency of pinned) {
            if (closure.has(dependency.manifest)) {
                const at =
                    dependency.kind === "destack" ? dependency.package.id : release.packageId;
                await this.#pull({ manifest: dependency.manifest, packageId: at }, pulling);
            }
        }

        // pull the output's files
        const { location, fetch } = await locate(release.manifest, release.packageId);
        await this.store.pull(location, fetch, name);
    }
}

/** Where a registry locates a build: its location and the fetch reading it. */
type Located = Awaited<ReturnType<Registry["locate"]>>;

/** One runtime's pull of a release: its closure, reads, locations and the builds visited. */
interface Pulling {
    /** The runtime whose outputs the pull takes. */
    readonly runtime: Runtime;
    /** The builds the release's runtime closure reaches. */
    readonly closure: ReadonlySet<Digest>;
    /** Read a build's manifest from the cache, else where it is located. */
    readonly read: ManifestRead;
    /** Locate a build once by its digest, at a package's registry. */
    readonly locate: (digest: Digest, at: PackageId) => Promise<Located>;
    /** The builds the pull visited. */
    readonly pulled: Set<Digest>;
}

/** A space's build store: its runners' pushes, read through to its machine's cache and registry. */
export class SpaceBuildStore {
    /** The space's own builds, retained while its build rows reference them. */
    readonly space: BuildStore;
    /** The machine's cache of what it runs. */
    readonly cache: BuildCache;
    /** The closure of each build mapped so far, by build, runtime and base path. */
    readonly #closures = new Map<string, Promise<ImportClosure>>();
    /** The builds opened so far, by manifest, each opened once as a build never changes. */
    readonly #opened = new Map<Digest, Promise<BuildReader>>();

    /** Store a space's builds in its blobs, read through to the machine's cache. */
    constructor(space: BuildStore, cache: BuildCache) {
        this.space = space;
        this.cache = cache;
    }

    /** Report whether the space or the machine stores a build. */
    async contains(manifest: Digest): Promise<boolean> {
        return (await this.space.contains(manifest)) || (await this.cache.store.contains(manifest));
    }

    /** Open a build the space stores, else the machine's copy of it. */
    async contents(manifest: Digest): Promise<BuildReader> {
        // open each build once, forgetting a failed open
        const known = this.#opened.get(manifest);
        if (known !== undefined) {
            return known;
        }
        const opening = this.store(manifest).then((store) => store.contents(manifest));
        this.#opened.set(manifest, opening);
        opening.catch(() => this.#opened.delete(manifest));

        return opening;
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
        const store = (await this.space.has(file.digest)) ? this.space : this.cache.store;

        return store.file(file);
    }

    /** List the npm units a build depends on, reading each from the space or the machine. */
    units(manifest: Digest, packageId: PackageId): Promise<Digest[]> {
        return this.cache.store.units(manifest, packageId, (digest) => this.get(digest));
    }

    /** Serve the adapter taking the files and manifest a running build's runner pushes. */
    adapter(): AdapterFetch {
        return async (request, admitted, _context, store) => {
            // read the build below the adapter, refusing a caller acting for none of its runners
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
                throw StackError.of("BUILD_NOT_FOUND", `no build ${buildId} runs for the caller`);
            }
            requireRunning(row);

            // take pushes of its files and its manifest into the space's builds, serving no pulls
            const mount = new URL(`${url.origin}/${spaceId}/${buildId}/`);

            return await this.space.fetch(request, mount, {
                pull: async () => {
                    throw StackError.of("BUILD_NOT_FOUND", "a runner pushes builds and pulls none");
                },
                push: async () => {},
            });
        };
    }

    /** Map a build's imports on a runtime below a base path, builtins at their linked releases. */
    imports(manifest: Digest, runtime: Runtime, base: string): Promise<ImportClosure> {
        // map each build once per runtime and base, as builds never change
        const key = `${manifest} ${runtime} ${base}`;
        let found = this.#closures.get(key);
        if (found === undefined) {
            found = this.cache.store.imports(manifest, runtime, base, this.cache.linked, (digest) =>
                this.get(digest),
            );
            this.#closures.set(key, found);
            found.catch(() => this.#closures.delete(key));
        }

        return found;
    }

    /** Read a build in place to inspect it: the space's own, else the cached or published one. */
    async read(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // read the space's own build, else one the machine caches or the registry publishes
        if (await this.space.contains(manifest)) {
            return await this.space.contents(manifest);
        }

        return await this.cache.read(registry, packageId, manifest);
    }

    /** Bring a build into the machine's cache to run it: the space's own, else a published one. */
    pull(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        return this.#pull(registry, packageId, manifest, new Map());
    }

    /** Bring a build into the cache after the builds it pins, visiting each build once per pull. */
    #pull(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
        visited: Map<Digest, Promise<BuildReader>>,
    ): Promise<BuildReader> {
        // share the visit of a build another dependent reached first
        const known = visited.get(manifest);
        if (known !== undefined) {
            return known;
        }
        const visiting = this.#visit(registry, packageId, manifest, visited);
        visited.set(manifest, visiting);

        return visiting;
    }

    /** Pull a published build with its closure, or the space's own after the builds it pins. */
    async #visit(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: Digest,
        visited: Map<Digest, Promise<BuildReader>>,
    ): Promise<BuildReader> {
        // pull a published build with its runtime closure
        if (!(await this.space.contains(manifest))) {
            return await this.cache.pull(registry, packageId, manifest);
        }

        // bring the builds the space's own build pins first, its npm units under its package, then adopt it
        const own = await this.space.contents(manifest);
        for (const output of Object.values(own.manifest.outputs)) {
            const pinned = Object.values(output.dependencies).filter((dependency) =>
                DependencyRelease.hasManifest(dependency),
            );
            for (const dependency of pinned) {
                const owner = dependency.kind === "destack" ? dependency.package.id : packageId;
                await this.#pull(registry, PackageId.parse(owner), dependency.manifest, visited);
            }
        }

        return await this.cache.adopt(own, manifest);
    }

    /** Take the build a stored build ends with, which its runner pushed. */
    async adopt(end: schema.Infer<typeof BuildEnd>): Promise<schema.Infer<typeof BuildOutcome>> {
        // store nothing of a failed build
        if (end.kind === "failed") {
            return end;
        }

        // require the build its runner pushed, then attach its code index
        const bytes = PackageManifest.serialize(end.manifest);
        const digest = await Digest.of(bytes, this.space.environment.crypto);
        if (!(await this.space.contains(digest))) {
            throw StackError.of("BUILD_NOT_STORED", `build ${digest} was not pushed to the space`);
        }
        await this.space.attach(digest, CodeIndex.file(end.code));

        return { kind: "stored", digest };
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
                        const { manifest } = source;
                        if (!(await options.builds.cache.store.contains(manifest))) {
                            throw StackError.of(
                                "RELEASE_NOT_PULLED",
                                `pull release ${manifest} into the build store before creating its build`,
                            );
                        }
                        const pulled = await options.builds.cache.store.get(manifest);
                        const built = PackageManifest.package(pulled);
                        if (built.id !== source.package || built.version !== source.version) {
                            throw StackError.of(
                                "MANIFEST_MISMATCH",
                                `manifest ${manifest} builds ${built.id} ${built.version}, not ${source.package} ${source.version}`,
                            );
                        }

                        return next(
                            context.with({
                                input: {
                                    ...context.input,
                                    state: "succeeded",
                                    manifest,
                                    outputs: Object.keys(pulled.outputs),
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
                            "BUILD_DUPLICATE",
                            `build ${existing.id} already builds the same commit`,
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
                            options.builds,
                            Build.manifest(context.target),
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
                // push a succeeded build with its npm units and referrers to its package's forge before the transaction
                prepare: async (context) => ({
                    manifest: await options.registry.push(
                        options.builds,
                        Build.manifest(context.target),
                    ),
                }),
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
        throw StackError.of("BUILD_STATE_CONFLICT", `build ${target.id} is ${target.state}`);
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
        throw StackError.of("BUILD_STATE_CONFLICT", `build ${target.id} is ${target.state}`);
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

/** Read the fields a finished build records: its failure, or the digest of its stored manifest. */
function ended(adopted: schema.Infer<typeof BuildOutcome>, now: number) {
    // record a failed build's failure
    if (adopted.kind === "failed") {
        return { state: "failed" as const, phase: null, failure: adopted.failure, finishedAt: now };
    }
    // record a stored build's manifest
    else {
        return {
            state: "succeeded" as const,
            phase: null,
            manifest: adopted.digest,
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
        throw StackError.of("BUILD_RUNNER_MISSING", "this space has no build runner");
    }
}

/** Refuse a build that is not running. */
function requireRunning(target: Pick<Build, "id" | "state">): void {
    if (target.state !== "running") {
        throw StackError.of("BUILD_STATE_CONFLICT", `build ${target.id} is ${target.state}`);
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
