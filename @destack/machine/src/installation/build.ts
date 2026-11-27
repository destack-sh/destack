import type { RevisionOptions } from "./target.ts";
import { Caller, principal } from "@destack/access";
import { LATEST_TAG } from "@destack/registry/object";
import { setting, SettingValue } from "@destack/setting/object";
import { buildRetention } from "@destack/stack/setting";
import { BuildStore } from "@destack/build/store";
import {
    type MethodContextOf,
    type NextOf,
    Limits,
    ObjectDescription,
    type ResultOf,
} from "@destack/object";
import { ObjectError } from "@destack/object/error";
import { account, Space } from "@destack/account/object";
import { SpaceAddress } from "@destack/account/address";
import type { GrantRequest } from "@destack/account/authentication";
import stackPackage from "@destack/stack/package";
import type { MountFetch, ObjectStore } from "@destack/object/server";
import { PackageManifest } from "@destack/package/manifest";
import type { PackageRelease } from "@destack/package";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope, Subject } from "@destack/sync";
import { build, Build, BuildSource, compile, installationRevision } from "@destack/stack/object";
import { StackError } from "@destack/stack/error";
import { LIVE_STATES } from "@destack/workflow";

/** The name of the limit on the pushes a machine sends into a space's builds. */
const PUSH_LIMIT = "build:push";

/** How many pushes a machine sends: 1024 writes of 64 MiB are 64 GiB an hour, as file parts are. */
const PUSH_RATE: NonNullable<Limits["rate"]> = { count: 1024, every: { hours: 1 } };

/** A repository commit source of a build. */
export type CommitSource = Extract<BuildSource, { readonly kind: "commit" }>;

/** The options of the build server. */
export interface BuildOptions {
    /** The space whose builds the server records, reading through to its machine's builds. */
    readonly space: Pick<RevisionOptions["space"], "builds" | "machine" | "runner">;
    /** Publish each of an account's packages in a stored image for a caller. */
    readonly publish: (
        store: BuildStore,
        image: Digest,
        account: Identifier<"account">,
        tag: string,
        commit: string,
        caller: Caller,
        requested: GrantRequest,
    ) => Promise<readonly PackageRelease[]>;
}

/** Take a machine's pushes of builds into the space's builds, for a build's image to reference. */
export function pushMount(builds: BuildStore): MountFetch {
    return async (request, admitted, _context, store) => {
        // refuse a machine past its rate, saying when the next write is due
        const now = store.clock.now();
        const caller = Subject.key(admitted.subject);
        const retryAfter = await Limits.take(store.database, PUSH_LIMIT, caller, PUSH_RATE, now);
        if (retryAfter !== undefined) {
            throw ObjectError.of(
                "RATE_EXHAUSTED",
                `caller '${caller}' exceeds its limit of ${PUSH_RATE.count} pushes per hour`,
                { data: { retryAfter } },
            );
        }

        // take its files, builds and images, leased until a build references them, serving no pulls
        const mount = new URL(`${new URL(request.url).origin}/`);

        return await builds.fetch(request, mount, {
            pull: async () => {
                throw StackError.of("BUILD_NOT_FOUND", "build for a machine to pull not found");
            },
            push: async () => {},
        });
    };
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
        controllers: [
            {
                // expire a space's builds past its retention
                name: "expire",
                by: ["scope"],
                reads: [
                    { table: installationRevision.table, fields: [], on: { scope: "scope" } },
                    {
                        table: setting.table,
                        where: { name: buildRetention.name },
                        fields: ["value"],
                        isListing: true,
                    },
                ],
                reconcile: ({ store }) =>
                    expireBuilds(
                        store,
                        (space) =>
                            SettingValue.resolve(store.database.snapshot(), buildRetention, {
                                scope: space,
                            }),
                        store.clock.now(),
                    ),
            },
        ],
        methods: {
            create: {
                handle: async (context, next) => {
                    // key the build by its source, outputs and version, whose running or succeeded build the creation answers
                    const source = BuildSource.parse(context.input.source);
                    const { outputs = null, version = null } = context.input;
                    const key = Build.key({ source, outputs, version });

                    // record a release its caller pulled, refusing another package or version, with nothing to run
                    if (source.kind === "release") {
                        const { image } = source;
                        const { builds: store, targets } = options.space.machine;
                        if (!(await store.isPulled({ kind: "image", digest: image }, targets))) {
                            throw StackError.of(
                                "RELEASE_BLOCKED",
                                `cannot create a build of release '${image}': pull it into the build store first`,
                            );
                        }
                        const root = await store.get((await store.image(image)).root);
                        const built = PackageManifest.package(root);
                        if (built.id !== source.package || built.version !== source.version) {
                            throw StackError.of(
                                "MANIFEST_INVALID",
                                `image '${image}' is invalid: its root builds ${built.id} '${built.version}', not ${source.package} '${source.version}'`,
                            );
                        }

                        return next(
                            context.with({
                                input: {
                                    ...context.input,
                                    key,
                                    state: "succeeded",
                                    image,
                                    outputs: Object.keys(root.outputs),
                                    startedAt: context.now,
                                    finishedAt: context.now,
                                },
                            }),
                        );
                    }

                    // run it on the machine asked for, else on the space's own runner
                    const runnerId = context.input.runnerId ?? options.space.runner();

                    // record it running under its source's key
                    const created = await next(
                        context.with({
                            input: {
                                ...context.input,
                                key,
                                runnerId,
                                commit: source.commit,
                                state: "running",
                                startedAt: context.now,
                            },
                        }),
                    );

                    // start its compile as the package's code, leaving a succeeded build the creation answers
                    if (created.state === "running") {
                        const started = compile.start(
                            { build: created.id },
                            { actor: context.self.subject, key: created.id },
                        );
                        await context.mutate(context.self, [started]);
                    }

                    return created;
                },
            },
            cancel,
            update: {
                handle: async (context, next) => {
                    // refuse an image the space's builds lack, so no build references missing bytes
                    const { image } = context.input;
                    const isStored =
                        image === undefined ||
                        image === null ||
                        (await options.space.builds.contains(image));
                    if (!isStored) {
                        throw StackError.of(
                            "BUILD_NOT_FOUND",
                            `image '${image}' not found: it was not pushed to the space`,
                        );
                    }

                    return next(context);
                },
            },
            publish: {
                // publish a succeeded build of a commit and its account's unpublished dependencies before the transaction
                prepare: async (context) => {
                    // refuse a release's build, as every release is built from a commit
                    const { source } = context.target;
                    if (source.kind !== "commit") {
                        throw StackError.of(
                            "RELEASE_BLOCKED",
                            `cannot publish build '${context.target.id}': it is a release`,
                        );
                    }

                    return {
                        releases: [
                            ...(await options.publish(
                                options.space.builds,
                                Build.image(context.target),
                                context.input.account,
                                context.input.tag ?? LATEST_TAG,
                                source.commit,
                                context.caller,
                                await publishRequest(context),
                            )),
                        ],
                    };
                },
                handle: async (context) => context.prepared,
            },
            push: {
                prepare: async (context) => {
                    // push a succeeded build's image and its builds to its account's registry before the transaction
                    const image = Build.image(context.target);
                    const { machine, builds } = options.space;
                    await machine.push(builds, image, context.input.account);

                    return { image };
                },
                handle: async (context) => context.prepared,
            },
        },
    });
}

/** Cancel a running build and its compile, whose runner then stops. */
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

    // record it cancelled
    const cancelled = await next(
        context.with({ input: { ...context.input, state: "cancelled", finishedAt: context.now } }),
    );

    // cancel its live compile run as the package's code
    const live = await context
        .read(compile.run.table)
        .findMany({ where: { key: target.id, state: { in: [...LIVE_STATES] } } });
    await context.mutate(
        context.self,
        live.map((run) =>
            compile.run.call("cancel", compile.run.target({ scope: target.scope, id: run.id })),
        ),
    );

    return cancelled;
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
