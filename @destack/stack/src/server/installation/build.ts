import type { Environment } from "@destack/environment";
import { principal } from "@destack/access";
import { BucketLease, type BucketReference, type BucketLeaseRequest } from "@destack/bucket/s3";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { Lease } from "@destack/resource";
import {
    type ManifestRead,
    PackageStore,
    RUNTIME_REFERRER,
    encodeManifest,
} from "@destack/build/store";
import type { Runtime } from "@destack/package/runtime";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import type { PackageId } from "@destack/package";
import { Digest, type Identifier, schema } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import {
    build,
    Build,
    BuildOutcome,
    BuildEnd,
    type BuildFile,
    type BuildUpload,
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
    /** Push a stored build with its npm units and referrers to its forge, answering its digest. */
    readonly push: (store: PackageStore, manifest: Digest, signal?: AbortSignal) => Promise<Digest>;
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
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The registry the space's builds are pushed to. */
    readonly registry: Registry;
}

/** A machine's store of its spaces' builds and its pulled releases, by manifest digest. */
export class BuildStore {
    /** The builds and releases by their manifests' digests. */
    readonly packages: PackageStore;
    /** Presign writing one file into the store. */
    readonly lease: (request: BucketLeaseRequest, now: number) => Promise<Lease>;
    /** The server runtimes the machine runs instances on, whose runtime builds a pull brings. */
    readonly runtimes: readonly Runtime[];

    /** Store builds for the machine's runtimes in a package store, presigning writes. */
    constructor(
        packages: PackageStore,
        runtimes: readonly Runtime[],
        lease: (request: BucketLeaseRequest, now: number) => Promise<Lease>,
    ) {
        this.packages = packages;
        this.runtimes = runtimes;
        this.lease = lease;
    }

    /** Open a machine's build store in a bucket at a host, created once, leasing writes into it. */
    static async of(
        host: CatalogueBucketHost,
        reference: BucketReference,
        runtimes: readonly Runtime[],
        environment: Pick<Environment, "compression" | "crypto">,
    ): Promise<BuildStore> {
        const bucket = await host.open(reference, reference.scope);
        const packages = new PackageStore(bucket, environment);

        return new BuildStore(packages, runtimes, (request, now) =>
            BucketLease.presign(host, reference, request, now, environment.crypto),
        );
    }

    /** Pull a published build into the store once, with the closure of each machine runtime. */
    async pull(
        registry: Pick<Registry, "locate" | "referrer">,
        packageId: PackageId,
        manifest: Digest,
    ): Promise<BuildReader> {
        // copy a build the store lacks from where the registry locates it
        if (!(await this.packages.contains(manifest))) {
            const { location, fetch } = await registry.locate(packageId, manifest);
            await this.packages.copy(location, fetch);
        }

        // pull the runtime build of each emitted output the machine runs
        const reader = await this.packages.contents(manifest);
        for (const output of Object.values(reader.manifest.outputs)) {
            if (output.emit && this.runtimes.includes(output.runtime)) {
                await this.#pullRuntime(registry, packageId, manifest, output.runtime);
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
        if ((await this.packages.referrer(manifest, path)) !== undefined) {
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
        const { crypto } = this.packages.environment;
        const served = new Map<Digest, BuildReader>();
        const read: ManifestRead = async (digest, located) => {
            if (await this.packages.contains(digest)) {
                return this.packages.get(digest);
            }
            const { location, fetch } = await registry.locate(located, digest);
            const opened = await BuildReader.open(location, fetch, crypto);
            served.set(digest, opened);

            return opened.manifest;
        };
        const closure = await this.packages.closure(built, packageId, read);

        // store each opened build after the builds it records, then attach the runtime build
        await this.#store(built, served, closure, new Set());
        await this.packages.attachBuild(manifest, built, path);
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
        const recorded = reader?.manifest ?? (await this.packages.get(manifest));
        for (const output of Object.values(recorded.outputs)) {
            for (const dependency of Object.values(output.dependencies)) {
                if (closure.has(dependency.manifest)) {
                    await this.#store(dependency.manifest, served, closure, stored);
                }
            }
        }

        // store an opened build, leaving one already in the store
        if (reader !== undefined) {
            await this.packages.put(reader);
        }
    }

    /** Lease writing each file the store lacks. */
    async leaseMissing(files: readonly BuildFile[], now: number): Promise<BuildUpload[]> {
        // look up each distinct file at once, and lease those the store lacks
        const distinct = [...new Map(files.map((file) => [file.digest, file])).values()];
        const leased = await Promise.all(
            distinct.map(async (file): Promise<BuildUpload | undefined> => {
                // skip a file the store has
                const key = PackageStore.fileKey(file.digest);
                if ((await this.packages.bucket.head(key)) !== null) {
                    return undefined;
                }

                // bind the file's size, type and digest into a write lease
                const headers = new Headers({
                    "content-length": String(file.size),
                    "content-type": file.mediaType,
                    "if-none-match": "*",
                    "x-amz-checksum-sha256": Uint8Array.fromHex(file.digest).toBase64(),
                });
                const written = await this.lease({ mode: "write", key, headers }, now);

                return { digest: file.digest, lease: written };
            }),
        );
        const uploads = leased.filter((upload) => upload !== undefined);

        return uploads;
    }

    /** Store the manifest a stored build ends with. */
    async adopt(end: schema.Infer<typeof BuildEnd>): Promise<schema.Infer<typeof BuildOutcome>> {
        // store nothing of a failed build
        if (end.kind === "failed") {
            return end;
        }

        // store the manifest's bytes under their digest, then attach its code index
        const bytes = encodeManifest(PackageManifest.parse(end.manifest));
        const digest = await Digest.of(bytes, this.packages.environment.crypto);
        await this.packages.adopt(digest, bytes);
        await this.packages.attach(digest, CodeIndex.file(end.code));

        return { kind: "stored", digest };
    }

    /** Expire the space's dropped builds, and root the retained ones under the space. */
    async retain(
        objects: Pick<ObjectStore, "database" | "principal" | "as" | "mutate" | "environment">,
        retention: (space: string) => Promise<number>,
        now: number,
    ): Promise<void> {
        // expire the dropped builds as the space's store
        const { database } = objects;
        const { retained, expired } = await Build.retention(database, retention);
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

        // root the retained builds under the space
        await this.packages.root(objects.principal.id, retained, new Date(now));
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
                        if (!(await options.builds.packages.contains(manifest))) {
                            throw StackError.of(
                                "RELEASE_NOT_PULLED",
                                `pull release ${manifest} into the build store before creating its build`,
                            );
                        }
                        const pulled = await options.builds.packages.get(manifest);
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
                        options.builds.packages,
                        Build.manifest(context.target),
                    ),
                }),
                handle: async (context) => context.prepared,
            },
            progress: {
                // lease the files a storing build's space lacks before the transaction
                prepare: async (context) => ({
                    uploads:
                        context.input.progress.phase === "storing"
                            ? await options.builds.leaseMissing(
                                  context.input.progress.files,
                                  context.now,
                              )
                            : [],
                }),
                handle: async (context) => {
                    // record the phase the runner reports
                    const { target, prepared } = context;
                    requireRunning(target);
                    const { progress } = context.input;
                    await context.update(reported(target, progress));

                    return { uploads: prepared.uploads };
                },
            },
            finish: {
                // store a succeeded build's manifest before the transaction
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
