import type { PackageFile } from "@destack/package/file";
import { principal } from "@destack/access";
import { and, eq } from "@destack/db";
import {
    type ManifestRead,
    PackageStore,
    MAPS_REFERRER,
    RUNTIME_REFERRER,
    encodeManifest,
} from "@destack/build/store";
import type { Runtime } from "@destack/package/runtime";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import type { AdapterFetch, ObjectStore } from "@destack/object/server";
import {
    BuildReader,
    type ImportMap,
    type PackageLocation,
    PackageManifest,
} from "@destack/package/manifest";
import { PackageId } from "@destack/package";
import { Digest, type Identifier, schema } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import {
    build,
    Build,
    BuildOutcome,
    BuildEnd,
    BuildSource,
    CodeIndex,
} from "../../object/index.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** A checkout source of a build. */
export type CheckoutSource = Extract<BuildSource, { readonly kind: "checkout" }>;

/** A repository commit source of a build. */
export type CommitSource = Extract<BuildSource, { readonly kind: "commit" }>;

/** The registry of published releases. */
export interface Registry {
    /** Find the digest of a package release's manifest, by its version or a distribution tag. */
    readonly find: (packageId: PackageId, release: string) => Promise<Digest>;
    /** Push a stored build with its npm units to its forge, answering its digest. */
    readonly push: (
        store: Pick<PackageStore, "contents" | "units">,
        manifest: Digest,
        signal?: AbortSignal,
    ) => Promise<Digest>;
    /** Read the build a published build attached at a path, such as its runtime build. */
    readonly referrer: (
        packageId: PackageId,
        manifest: Digest,
        path: string,
    ) => Promise<Digest | undefined>;
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
    readonly builds: BuildStore;
    /** The registry the space's builds are pushed to. */
    readonly registry: Registry;
}

/** A machine's cache of the builds it runs: pulled releases and copies of its spaces' builds. */
export class BuildCache {
    /** The store keeping the cached builds. */
    readonly store: PackageStore;
    /** The runtimes the machine serves: its instances' server runtime and its views' browser. */
    readonly runtimes: readonly Runtime[];

    /** Cache builds in a store for the runtimes the machine serves. */
    constructor(store: PackageStore, runtimes: readonly Runtime[]) {
        this.store = store;
        this.runtimes = runtimes;
    }

    /** Bring a published build into the cache with the runtime builds its registry attached. */
    async pull(
        registry: Pick<Registry, "locate" | "referrer">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // copy the build from where the registry locates it, unless cached
        if (!(await this.store.contains(manifest))) {
            const { location, fetch } = await registry.locate(packageId, manifest);
            await this.store.copy(location, fetch);
        }

        // pull the runtime build of each emitted output the machine runs
        const reader = await this.store.contents(manifest);
        for (const output of Object.values(reader.manifest.outputs)) {
            if (output.emit && this.runtimes.includes(output.runtime)) {
                await this.#pullRuntime(registry, packageId, manifest, output.runtime);
            }
        }

        return reader;
    }

    /** Copy a build of a space into the cache, deriving the runtime builds the machine runs. */
    async adopt(source: BuildReader, manifest: Digest): Promise<BuildReader> {
        // copy the build unless cached, then derive each runtime build it needs here
        if (!(await this.store.contains(manifest))) {
            await this.store.put(source);
        }
        const reader = await this.store.contents(manifest);
        for (const output of Object.values(reader.manifest.outputs)) {
            if (output.emit && this.runtimes.includes(output.runtime)) {
                await this.store.attachRuntime(manifest, output.runtime);
            }
        }

        return reader;
    }

    /** Pull a build's runtime build and its runtime closure once, attaching it to the build. */
    async #pullRuntime(
        registry: Pick<Registry, "locate" | "referrer">,
        packageId: PackageId,
        manifest: Digest,
        runtime: Runtime,
    ): Promise<void> {
        // take a runtime build attached here, which the store attaches once its closure is stored
        const path = `${RUNTIME_REFERRER}/${runtime}`;
        if ((await this.store.referrer(manifest, path)) !== undefined) {
            return;
        }

        // find the runtime build the registry attached
        const built = await registry.referrer(packageId, manifest, path);
        if (built === undefined) {
            throw StackError.of(
                "RUNTIME_BUILD_MISSING",
                `${packageId} publishes no ${path} build of ${manifest}`,
            );
        }

        // read its closure, opening the builds the store lacks where the registry locates them
        const { crypto } = this.store.environment;
        const served = new Map<Digest, BuildReader>();
        const read: ManifestRead = async (digest, located) => {
            if (await this.store.contains(digest)) {
                return this.store.get(digest);
            }
            const { location, fetch } = await registry.locate(located, digest);
            const opened = await BuildReader.open(location, fetch, crypto);
            served.set(digest, opened);

            return opened.manifest;
        };
        const closure = await this.store.closure(built, packageId, read);

        // store each opened build after the builds it records
        await this.#store(built, served, closure, new Set());

        // copy the source maps the registry attached to the runtime build, when its output has any
        const maps = await registry.referrer(packageId, built, MAPS_REFERRER);
        if (maps !== undefined) {
            const { location, fetch } = await registry.locate(packageId, maps);
            await this.store.copy(location, fetch);
            await this.store.attachBuild(built, maps, MAPS_REFERRER);
        }

        // attach the runtime build, which the store answers once its closure and maps are stored
        await this.store.attachBuild(manifest, built, path);
    }

    /** Store the opened builds of a closure after the builds they record, visiting each once. */
    async #store(
        manifest: Digest,
        served: ReadonlyMap<Digest, BuildReader>,
        closure: ReadonlySet<Digest>,
        stored: Set<Digest>,
    ): Promise<void> {
        // visit each build once, storing what it records first
        if (stored.has(manifest)) {
            return;
        }
        stored.add(manifest);
        const reader = served.get(manifest);
        const recorded = reader?.manifest ?? (await this.store.get(manifest));
        for (const output of Object.values(recorded.outputs)) {
            for (const dependency of Object.values(output.dependencies)) {
                if (closure.has(dependency.manifest)) {
                    await this.#store(dependency.manifest, served, closure, stored);
                }
            }
        }

        // store an opened build, leaving one already in the store
        if (reader !== undefined) {
            await this.store.put(reader);
        }
    }
}

/** A build's browser closure: the import map of its modules and the builds the map reaches. */
export interface BrowserClosure {
    /** The import map of the build's browser modules and its dependencies'. */
    readonly map: ImportMap;
    /** The builds the map's URLs reach: the browser runtime build and its dependencies'. */
    readonly builds: ReadonlySet<Digest>;
}

/** A space's builds: its runners' pushes, read through to its machine's cache. */
export class BuildStore {
    /** The space's own builds, kept while its build rows reference them. */
    readonly space: PackageStore;
    /** The machine's cache of what it runs. */
    readonly cache: BuildCache;
    /** The browser closure of each build served so far, by build and base path. */
    readonly #browsers = new Map<string, Promise<BrowserClosure>>();

    /** Keep a space's builds in its blobs, read through to the machine's cache. */
    constructor(space: PackageStore, cache: BuildCache) {
        this.space = space;
        this.cache = cache;
    }

    /** Report whether the space or the machine stores a build. */
    async contains(manifest: Digest): Promise<boolean> {
        return (await this.space.contains(manifest)) || (await this.cache.store.contains(manifest));
    }

    /** Open a build the space stores, else the machine's copy of it. */
    async contents(manifest: Digest): Promise<BuildReader> {
        return (await this.store(manifest)).contents(manifest);
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

            // take uploads of its files and its manifest into the space's builds, reading nothing back
            const mount = new URL(`${url.origin}/${spaceId}/${buildId}/`);

            return await this.space.fetch(request, mount, {
                authorize: async () => {
                    throw StackError.of("BUILD_NOT_FOUND", "a runner pushes builds and reads none");
                },
                upload: async () => {},
            });
        };
    }

    /** Map a browser output's imports to its dependencies' files below a base path. */
    browser(manifest: Digest, base: string): Promise<BrowserClosure> {
        // map each build once, as builds never change
        const key = `${manifest} ${base}`;
        let found = this.#browsers.get(key);
        if (found === undefined) {
            found = this.#browser(manifest, base);
            this.#browsers.set(key, found);
            found.catch(() => this.#browsers.delete(key));
        }

        return found;
    }

    /** Walk a build's browser runtime build and its pinned dependencies, mapping their exports. */
    async #browser(manifest: Digest, base: string): Promise<BrowserClosure> {
        // start from the browser runtime build a pull attached
        const runtime = await this.referrer(manifest, `${RUNTIME_REFERRER}/browser`);
        if (runtime === undefined) {
            throw StackError.of("RUNTIME_BUILD_MISSING", `${manifest} has no browser build here`);
        }

        // read the builds its views' modules reach, as a pull of its runtime closure stores them
        const started = await this.get(runtime.digest);
        const reached = await this.cache.store.closure(
            runtime.digest,
            PackageManifest.package(started).id,
            (digest) => this.get(digest),
        );

        // map each reached dependency's exports in its dependent's scope, visiting each build once
        const map: ImportMap = { imports: {}, scopes: {} };
        const builds = new Set<Digest>([runtime.digest]);
        const pending = [{ digest: runtime.digest, scope: map.imports }];
        for (const { digest, scope } of pending) {
            for (const output of Object.values((await this.get(digest)).outputs)) {
                const dependencies = Object.entries(output.dependencies).filter(([, release]) =>
                    reached.has(release.manifest),
                );
                for (const [name, release] of dependencies) {
                    // map the dependency's exports to its files below its build's prefix
                    const prefix = `${base}${release.manifest}/`;
                    const dependency = await this.get(release.manifest);
                    for (const each of Object.values(dependency.outputs)) {
                        for (const [subpath, exported] of Object.entries(each.exports)) {
                            const specifier = subpath === "." ? name : `${name}${subpath.slice(1)}`;
                            scope[specifier] = `${prefix}${exported.path}`;
                        }
                    }

                    // map the dependency's own imports in its scope
                    if (!builds.has(release.manifest)) {
                        builds.add(release.manifest);
                        map.scopes[prefix] = {};
                        pending.push({ digest: release.manifest, scope: map.scopes[prefix] });
                    }
                }
            }
        }

        return { map, builds };
    }

    /** Bring a build into the machine's cache to run it: the space's own, else a published one. */
    async pull(
        registry: Pick<Registry, "locate" | "referrer">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // pull a published build with its runtime closure
        if (!(await this.space.contains(manifest))) {
            return await this.cache.pull(registry, packageId, manifest);
        }

        // bring the builds the space's own build pins first, its npm units under its package, then adopt it
        const own = await this.space.contents(manifest);
        for (const output of Object.values(own.manifest.outputs)) {
            for (const dependency of Object.values(output.dependencies)) {
                const owner = dependency.kind === "destack" ? dependency.package.id : packageId;
                await this.pull(registry, PackageId.parse(owner), dependency.manifest);
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
        const bytes = encodeManifest(PackageManifest.parse(end.manifest));
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
    async store(manifest: Digest): Promise<PackageStore> {
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
