import type { RevisionOptions } from "./target.ts";
import { type Caller, principal } from "@destack/access";
import { LATEST_TAG } from "@destack/registry/object";
import { and, eq, Projection, Snapshot } from "@destack/db";
import { SettingValue } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { type BuildObject, BuildStore } from "@destack/build/store";
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
import stackPackage from "@destack/stack/package";
import type { MountFetch, ObjectStore } from "@destack/object/server";
import { Lock, PackageManifest } from "@destack/package/manifest";
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
} from "@destack/stack/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "@destack/stack/error";

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
    /** The space whose builds the server records, reading through to its machine's builds. */
    readonly space: Pick<RevisionOptions["space"], "builds" | "machine">;
    /** The registry the space's builds are pushed to. */
    readonly registry: Registry;
    /** Publish each of an account's packages in a stored lock for a caller. */
    readonly publish: (
        store: PublishStore,
        lock: Digest,
        account: Identifier<"account">,
        tag: string,
        commit: string | undefined,
        caller: Caller,
        requested: GrantRequest,
    ) => Promise<readonly PackageRelease[]>;
}

/** Take a build runner's pushes of its files, builds and lock into the space's builds. */
export function pushMount(builds: BuildStore): MountFetch {
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

        return await builds.fetch(request, mount, {
            pull: async () => {
                throw StackError.of("BUILD_NOT_FOUND", "build for a runner to pull not found");
            },
            push: async () => {},
        });
    };
}

/** Take a finished build's pushed lock, attaching its code index to its root build. */
async function adopt(
    builds: BuildStore,
    end: schema.Infer<typeof BuildEnd>,
): Promise<schema.Infer<typeof BuildOutcome>> {
    // store nothing of a failed build
    if (end.kind === "failed") {
        return end;
    }

    // require the lock its runner pushed
    if (!(await builds.contains(end.lock))) {
        throw StackError.of(
            "BUILD_NOT_FOUND",
            `lock '${end.lock}' not found: it was not pushed to the space`,
        );
    }
    await builds.attach((await builds.lock(end.lock)).root, CodeIndex.file(end.code));

    return { kind: "stored", lock: end.lock };
}

/** Declare the projection expiring the space's builds past its retention as builds change. */
export function retainBuilds(store: ObjectStore): Projection {
    const { database, scope } = store;

    return new Projection(database, {
        name: "retention",
        tables: [build.table, installationRevision.table],
        keys: async () => [scope],
        all: async () => [scope],
        project: async () =>
            expireBuilds(
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
async function expireBuilds(
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

/** Implement builds: a creator creates and cancels a build. */
export function implementBuild(options: BuildOptions) {
    return build.implement({
        methods: {
            create: {
                // relate a compiled build's runner, which its modules and symbols use
                relate: (context) => ({
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
                        const store = options.space.machine.builds;
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
                            options.space.builds,
                            Build.lock(context.target),
                            context.input.account,
                            context.input.tag ?? LATEST_TAG,
                            Build.commit(context.target),
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
                    await options.registry.push(options.space.builds, lock, context.input.account);

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
                prepare: (context) => adopt(options.space.builds, context.input.end),
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
