import { principal } from "@destack/access";
import { BucketLease, type BucketReference, type BucketLeaseRequest } from "@destack/bucket/s3";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { Lease } from "@destack/resource";
import { PackageStore, Retention, encodeManifest } from "@destack/build/store";
import { Snapshot } from "@destack/db";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import type { PackageRelease, PackageId } from "@destack/package";
import { PackageDescription } from "@destack/runtime";
import { Digest, type Identifier, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { ObjectReference, Scope } from "@destack/sync";
import {
    build,
    Build,
    BuildOutcome,
    BuildEnd,
    type BuildFile,
    type BuildUpload,
    BuildSource,
    installation,
    installationRevision,
} from "../../object/index.ts";

/** A checkout source of a build. */
export type CheckoutSource = Extract<BuildSource, { readonly kind: "checkout" }>;

/** A repository commit source of a build. */
export type CommitSource = Extract<BuildSource, { readonly kind: "commit" }>;

/** The registry of published releases. */
export interface Registry {
    /** Find the digest of a package release's manifest, by its version or a distribution tag. */
    readonly find: (packageId: PackageId, release: string) => Promise<string>;
    /** Push a build's files to its package's forge, answering its manifest's digest. */
    readonly push: (build: BuildReader, signal?: AbortSignal) => Promise<string>;
    /** Locate the files of a package's published build by its manifest's digest. */
    readonly locate: (
        packageId: PackageId,
        manifest: string,
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

    /** Keep builds in a package store, presigning writes with a lease. */
    constructor(
        packages: PackageStore,
        lease: (request: BucketLeaseRequest, now: number) => Promise<Lease>,
    ) {
        this.packages = packages;
        this.lease = lease;
    }

    /** Keep a machine's builds in a bucket at a host, created once, leasing writes into it. */
    static async of(host: CatalogueBucketHost, reference: BucketReference): Promise<BuildStore> {
        const packages = new PackageStore(await host.open(reference, reference.scope));

        return new BuildStore(packages, (request, now) =>
            BucketLease.presign(host, reference, request, now),
        );
    }

    /** Read a published build from the store, or through the registry without storing it. */
    async read(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: string,
    ): Promise<BuildReader> {
        // read a stored build, else stream the published one's files from where the registry locates them
        if (await this.packages.contains(manifest)) {
            return this.packages.contents(manifest);
        }
        const { location, fetch } = await registry.locate(packageId, manifest);

        return BuildReader.open(location, fetch);
    }

    /** Read a published build, copying it from the registry into the store unless stored. */
    async open(
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: string,
    ): Promise<BuildReader> {
        // copy a build the store lacks from where the registry locates it
        if (!(await this.packages.contains(manifest))) {
            const { location, fetch } = await registry.locate(packageId, manifest);
            await this.packages.copy(location, fetch);
        }

        return this.packages.contents(manifest);
    }

    /** Read the build an installation of a package in a space follows. */
    async installed(
        snapshot: Snapshot,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId?: Identifier<"installation">,
    ): Promise<BuildReader> {
        // read the installation's revision
        const [found] = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        if (found?.revisionId === undefined || found.revisionId === null) {
            throw new CallError("NOT_FOUND", {
                message: `no installation in ${spaceId} follows a build of ${packageId}`,
            });
        }

        return this.revision(snapshot, found.revisionId);
    }

    /** Read the build a revision evaluated. */
    async revision(
        snapshot: Pick<Snapshot, "read">,
        revisionId: Identifier<"installation-revision">,
    ): Promise<BuildReader> {
        // read the revision, refusing an unknown one
        const revision = await snapshot
            .read(installationRevision.table)
            .findFirst({ where: { id: revisionId } });
        if (revision === undefined) {
            throw new CallError("NOT_FOUND", { message: `no revision ${revisionId}` });
        }

        // read its build from the store
        const built = await Build.require(snapshot, revision.scope, revision.buildId);

        return this.packages.contents(built.manifest);
    }

    /** Lease writing each file the store lacks. */
    async leaseMissing(files: readonly BuildFile[], now: number): Promise<BuildUpload[]> {
        // look up each distinct file at once, and lease those the store lacks
        const distinct = [...new Map(files.map((file) => [file.digest, file])).values()];
        const leased = await Promise.all(
            distinct.map(async (file): Promise<BuildUpload | undefined> => {
                // skip a file the store keeps
                const key = `files/${file.digest}`;
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

    /** Keep the manifest a stored build ends with. */
    async adopt(end: schema.Infer<typeof BuildEnd>): Promise<schema.Infer<typeof BuildOutcome>> {
        // keep nothing of a failed build
        if (end.kind === "failed") {
            return end;
        }

        // keep the manifest's bytes under their digest
        const bytes = encodeManifest(PackageManifest.parse(end.manifest));
        const digest = await Digest.of(bytes);
        await this.packages.adopt(digest, bytes);

        return { kind: "stored", digest };
    }

    /** Expire the builds retention drops, root the rest, sweep, and answer the kept builds. */
    async sweep(
        objects: Pick<ObjectStore, "database" | "principal" | "serving" | "mutate">,
        retention: (space: string) => Promise<number>,
        now: number,
        before: Date,
    ): Promise<ReadonlySet<string>> {
        // expire the dropped builds as the machine
        const { database } = objects;
        const { retained, expired } = await Build.retention(database, retention);
        if (expired.length > 0) {
            await objects.mutate(
                objects.serving(build),
                {
                    id: RequestId.create(now),
                    calls: expired.map((row) =>
                        build.call("update", { ...build.target(row), expiredAt: now }),
                    ),
                },
                { component: "retention" },
            );
        }

        // keep the store's retained builds as its root and sweep what no root retains
        await this.packages.root(objects.principal.id, retained);
        const roots = await this.packages.roots();
        await this.packages.sweep(Retention.of(roots), before);

        return roots;
    }
}

/** List the releases a build depends on that declare a package, leaving libraries out. */
export async function installableReleases(
    reader: BuildReader,
    read: (packageId: PackageId, manifest: string) => Promise<BuildReader>,
): Promise<PackageRelease[]> {
    const installable = await Promise.all(
        (await reader.releases()).map(async (release) => {
            const dependency = await read(release.package.id, release.manifest);
            const declared = await PackageDescription.of(dependency);

            return declared === undefined ? [] : [release];
        }),
    );

    return installable.flat();
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
                    // pull a release into the build store at its pinned manifest, refusing another package or version
                    const spaceId = schema.identifier("space").parse(context.scope);
                    const source = BuildSource.parse(context.input.source);
                    if (source.kind === "release") {
                        const { builds, registry } = options;
                        const { manifest } = source;
                        const reader = await builds.open(registry, source.package, manifest);
                        const built = reader.manifest.package;
                        if (built.id !== source.package || built.version !== source.version) {
                            throw new CallError("BAD_REQUEST", {
                                message: `manifest ${manifest} builds ${built.id} ${built.version}, not ${source.package} ${source.version}`,
                            });
                        }

                        // record it succeeded at that manifest, with nothing to run and the outputs it has
                        return next(
                            context.with({
                                input: {
                                    ...context.input,
                                    state: "succeeded",
                                    manifest,
                                    outputs: Object.keys(reader.manifest.outputs),
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
                        throw new CallError("CONFLICT", {
                            message: `build ${existing.id} builds the same commit`,
                        });
                    }

                    // compile it on the runner its source selects
                    const runner = runnerOf(context, options.buildRunner);

                    return next(
                        context.with({
                            input: {
                                ...context.input,
                                ...ObjectReference.columns("runner", runner),
                            },
                        }),
                    );
                },
            },
            cancel,
            start,
            push: {
                // push a succeeded build's files to its package's forge before the transaction
                prepare: async (context) => {
                    const files = await options.builds.packages.contents(
                        Build.manifest(context.target),
                    );

                    return { manifest: await options.registry.push(files) };
                },
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
                // keep a succeeded build's manifest before the transaction
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
        throw new CallError("CONFLICT", { message: `build ${target.id} is ${target.state}` });
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
        throw new CallError("CONFLICT", { message: `build ${target.id} is ${target.state}` });
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
        throw new CallError("PRECONDITION_FAILED", {
            message: "no build runner serves this space",
        });
    }
}

/** Refuse a build that is not running. */
function requireRunning(target: Pick<Build, "id" | "state">): void {
    if (target.state !== "running") {
        throw new CallError("CONFLICT", { message: `build ${target.id} is ${target.state}` });
    }
}
