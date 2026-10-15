import { Caller, principal } from "@destack/access";
import { BucketLease, type BucketReference, type BucketLeaseRequest } from "@destack/bucket/s3";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { Lease } from "@destack/resource";
import { PackageStore, Retention, encodeManifest } from "@destack/build/store";
import { Snapshot } from "@destack/db";
import type { CallOf, NextOf, ResultOf } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { type BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import type { PackageId } from "@destack/package";
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
export interface BuildStore {
    /** The builds and releases by their manifests' digests. */
    readonly packages: PackageStore;
    /** Presign writing one file into the store. */
    readonly lease: (request: BucketLeaseRequest, now: number) => Promise<Lease>;
}

/** The verbs on a machine's build store. */
export const BuildStore = {
    /** Read a published build, copying it from the registry into the store first. */
    async open(
        store: BuildStore,
        registry: Pick<Registry, "locate">,
        packageId: PackageId,
        manifest: string,
    ): Promise<BuildReader> {
        const { location, fetch } = await registry.locate(packageId, manifest);
        await store.packages.copy(location, fetch);

        return store.packages.contents(manifest);
    },

    /** Keep a machine's builds in a bucket at a host, created once, leasing writes into it. */
    async of(host: CatalogueBucketHost, reference: BucketReference): Promise<BuildStore> {
        const packages = new PackageStore(await host.open(reference, reference.scope));

        return {
            packages,
            lease: (request, now) => BucketLease.presign(host, reference, request, now),
        };
    },

    /** Read the build an installation of a package in a space follows. */
    async installed(
        builds: BuildStore,
        snapshot: Snapshot,
        spaceId: Identifier<"space">,
        packageId: PackageId,
        installationId?: Identifier<"installation">,
    ): Promise<BuildReader> {
        // read the installation and the revision it follows
        const [found] = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                packageId,
                ...(installationId === undefined ? {} : { id: installationId }),
            },
        });
        const revision =
            found === undefined || found.revisionId === null
                ? undefined
                : await snapshot
                      .read(installationRevision.table)
                      .findFirst({ where: { id: found.revisionId } });
        if (revision === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `no installation in ${spaceId} follows a build of ${packageId}`,
            });
        }

        return builds.packages.contents(revision.build.manifest);
    },

    /** Expire the builds retention drops, keep the rest as the store's root, and sweep. */
    async sweep(
        builds: BuildStore,
        objects: Pick<ObjectStore, "database" | "principal" | "mutate">,
        retention: (space: string) => Promise<number>,
        now: number,
        before: Date,
    ): Promise<ReadonlySet<string>> {
        // expire the dropped builds as the machine
        const { database } = objects;
        const { retained, expired } = await Build.retention(database, retention);
        if (expired.length > 0) {
            await objects.mutate(
                Caller.of(objects.principal, now),
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
        await builds.packages.root(objects.principal.id, retained);
        await builds.packages.sweep(Retention.of(await builds.packages.roots()), before);

        return retained;
    },
};

/** Serve builds: a creator creates and cancels a build. */
export function serveBuilds(options: BuildOptions) {
    return build.implement({
        methods: {
            create: {
                // relate the runner its creation selects, which the build's modules and symbols use
                creation: (call) => ({
                    relationships: [
                        { relation: "runner", subject: runnerOf(call, options.buildRunner) },
                    ],
                }),
                handle: async (call, next) => {
                    // select the runner
                    const spaceId = schema.identifier("space").parse(call.scope);
                    const runner = runnerOf(call, options.buildRunner);

                    // refuse a second live build of a commit's package directory
                    const source = BuildSource.parse(call.input.source);
                    const existing =
                        source.kind === "commit"
                            ? await Build.find(call, spaceId, source)
                            : undefined;
                    if (existing !== undefined) {
                        throw new CallError("CONFLICT", {
                            message: `build ${existing.id} builds the same commit`,
                        });
                    }

                    return next(
                        call.with({
                            input: { ...call.input, ...ObjectReference.columns("runner", runner) },
                        }),
                    );
                },
            },
            cancel,
            start,
            push: {
                // push a succeeded build's files to its package's forge before the transaction
                prepare: async (call) => {
                    const files = await options.builds.packages.contents(
                        Build.manifest(call.target),
                    );

                    return { manifest: await options.registry.push(files) };
                },
                handle: async (call) => call.prepared,
            },
            progress: {
                // lease the files a storing build's space lacks before the transaction
                prepare: async (call) => ({
                    uploads:
                        call.input.progress.phase === "storing"
                            ? await lease(options.builds, call.input.progress.files, call.now)
                            : [],
                }),
                handle: async (call) => {
                    // record the phase the runner reports
                    const { target, prepared } = call;
                    requireRunning(target);
                    const { progress } = call.input;
                    await call.update(reported(target, progress));

                    return { uploads: prepared.uploads };
                },
            },
            finish: {
                // keep a succeeded build's manifest before the transaction
                prepare: (call) => adopt(options.builds, call.input.end),
                handle: async (call) => {
                    // finish a running build with its stored manifest or its failure
                    requireRunning(call.target);
                    await call.update(ended(call.prepared, call.now));

                    return {};
                },
            },
        },
    });
}

/** Start a pending build. */
async function start(
    call: CallOf<typeof build, "start">,
): Promise<ResultOf<typeof build, "start">> {
    // refuse a build that is not pending
    const { target } = call;
    if (target.state !== "pending") {
        throw new CallError("CONFLICT", { message: `build ${target.id} is ${target.state}` });
    }

    // record it running
    await call.update({ state: "running", phase: "preparing", startedAt: call.now });

    return {};
}

/** Cancel a pending or running build for its runner to abort. */
async function cancel(
    call: CallOf<typeof build, "cancel">,
    next: NextOf<typeof build, "cancel">,
): Promise<ResultOf<typeof build, "cancel">> {
    // refuse a finished build
    const { target } = call;
    if (Build.isFinished(target)) {
        throw new CallError("CONFLICT", { message: `build ${target.id} is ${target.state}` });
    }

    return next(
        call.with({
            input: { ...call.input, state: "cancelled", phase: null, finishedAt: call.now },
        }),
    );
}

/** Read the fields a runner's reported phase records. */
function reported(
    target: Pick<Build, "source">,
    progress: CallOf<typeof build, "progress">["input"]["progress"],
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

/** Lease writing each file the build store lacks. */
async function lease(
    builds: BuildStore,
    files: readonly BuildFile[],
    now: number,
): Promise<BuildUpload[]> {
    // look up each distinct file at once, and lease those the store lacks
    const distinct = [...new Map(files.map((file) => [file.digest, file])).values()];
    const leased = await Promise.all(
        distinct.map(async (file): Promise<BuildUpload | undefined> => {
            // skip a file the store keeps
            const key = `files/${file.digest}`;
            if ((await builds.packages.bucket.head(key)) !== null) {
                return undefined;
            }

            // bind the file's size, type and digest into a write lease
            const headers = new Headers({
                "content-length": String(file.size),
                "content-type": file.mediaType,
                "if-none-match": "*",
                "x-amz-checksum-sha256": Uint8Array.fromHex(file.digest).toBase64(),
            });
            const written = await builds.lease({ mode: "write", key, headers }, now);

            return { digest: file.digest, lease: written };
        }),
    );
    const uploads = leased.filter((upload) => upload !== undefined);

    return uploads;
}

/** Keep the manifest a stored build ends with in the build store. */
async function adopt(
    builds: BuildStore,
    end: schema.Infer<typeof BuildEnd>,
): Promise<schema.Infer<typeof BuildOutcome>> {
    // keep nothing of a failed build
    if (end.kind === "failed") {
        return end;
    }

    // keep the manifest's bytes under their digest
    const bytes = encodeManifest(PackageManifest.parse(end.manifest));
    const digest = await Digest.of(bytes);
    await builds.packages.adopt(digest, bytes);

    return { kind: "stored", digest };
}

/** Read the machine running a new build. */
function runnerOf(
    call: Pick<CallOf<typeof build, "create">, "input">,
    buildRunner: Identifier<"machine"> | null,
): ObjectReference {
    const { source } = call.input;

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
