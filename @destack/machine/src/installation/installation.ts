import { InstallationDefinition } from "@destack/stack/declare";
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import { Plan } from "@destack/resource";
import { setting } from "@destack/setting/object";
import { ObjectReference, Subject } from "@destack/sync";
import { present, schema } from "@destack/schema";
import {
    InstallationId,
    PermissionReference,
    PermissionScope,
    type PermissionSet,
    principal,
} from "@destack/access";
import { type Relation, Snapshot } from "@destack/db";
import {
    type Installation,
    installation,
    type MethodContext,
    type MethodContextOf,
    type ObjectProjection,
    type ObjectWatch,
    PackageDescription,
    type ResourceType,
    type ResultOf,
    changeRequest,
} from "@destack/object";
import { entitlement, placement, Space, space } from "@destack/account/object";
import {
    build,
    deployment,
    installationRevision,
    Rollout,
    rollout,
    Build,
    type InstallationRollout,
} from "@destack/stack/object";
import { ViewDescription } from "@destack/package/manifest";
import { type ApplicationOptions, ApplicationInstallation } from "./application.ts";
import { Approval } from "./approval.ts";
import { checkout } from "../computer/checkout/object/index.ts";
import { CommitBuild } from "./follow.ts";
import { reference, tag } from "@destack/registry/object";
import { StackInstallation } from "./stack.ts";
import type { TokenIssuer } from "@destack/access/token";
import { StackError } from "@destack/stack/error";
import type { ObjectBindingKind } from "@destack/stack/object";
import { RolloutTarget, type RevisionOptions } from "./target.ts";

/** The options a machine serves installations with. */
export interface InstallationOptions extends RevisionOptions, ApplicationOptions {
    /** The objects of the resource kinds the machine serves. */
    readonly resources: readonly ResourceType<typeof space>[];
    /** The account package, reading the home a person opens views in. */
    readonly accounts: Pick<ClientOf<typeof accountPackage>, "user">;
    /** The space's token issuer, signing the tokens installations are granted. */
    readonly issuer: TokenIssuer;
}

/** Implement installations: create, update and delete them, allow capabilities, grant tokens. */
export function implementInstallations(options: InstallationOptions) {
    return installation.implement({
        // make the name other installations address a declared one by immutable, and resolve its bindings' targets
        declaration: {
            // follow the types its bindings target, retired before them
            after: options.binder.targets(),
            immutable: ["name"],
            // read it as a stack declares it
            schema: InstallationDefinition,
            resolve: async (_name, declared, plan) => {
                // follow the row the declaration pins or was built with, and resolve the bindings' targets
                const { selection, version, bindings, ...rest } =
                    InstallationDefinition.parse(declared);
                const { follows, directory } = await RolloutTarget.resolveDeclared(
                    rest.packageId,
                    { selection, version },
                    options.space.machine,
                );

                return {
                    ...rest,
                    ...ObjectReference.columns("follows", follows),
                    directory,
                    ...(bindings === undefined
                        ? {}
                        : { bindings: await options.binder.resolve(bindings, plan) }),
                };
            },
        },
        methods: {
            create: async (call, next) => {
                // refuse another account's repository
                const { name } = call.input;
                await options.space.requireFollowable(RolloutTarget.followed(call.input));

                // identify it by its scope and name, as readers derive it, then roll it out as its installer
                const id = await InstallationId.of(
                    { id: call.scope },
                    name,
                    call.environment.crypto,
                );
                const created = await next(call.identify(id));
                await call.mutate(call.caller, [
                    rollout.call("create", {
                        ...rollout.in(call.scope),
                        installationId: created.id,
                    }),
                ]);

                return await deployed(call, created);
            },
            update: async (call, next) => {
                // refuse another account's repository, then apply what it follows now
                await options.space.requireFollowable(
                    RolloutTarget.followed({ ...call.target, ...call.input }),
                );
                const updated = await next(call);

                return await deployed(call, updated);
            },
            delete: async (call, next) => {
                // refuse uninstalling one another installation needs, then drain it
                const [rolled] = await Rollout.installed(call, [call.target]);
                if (rolled !== undefined) {
                    await requireUnneeded(call, rolled, options);
                }
                const deleted = await next(call);
                await applyDirect(call, call.target);

                return deleted;
            },
            allow: (call) =>
                call.update({
                    allowed: [...new Set([...call.target.allowed, call.input.capability])],
                }),
            disallow: (call) =>
                call.update({
                    allowed: call.target.allowed.filter((name) => name !== call.input.capability),
                }),
        },
    });
}

/** Deploy an installation a person or agent wrote directly, leaving a declared one to its stack. */
async function applyDirect(
    call: MethodContext,
    installed: Pick<Installation, "id" | "scope" | "managerInstallationId">,
): Promise<void> {
    if (installed.managerInstallationId === null) {
        await call.mutate(call.self, [
            rollout.call("deploy", rollout.target({ scope: installed.scope, id: installed.id })),
        ]);
    }
}

/** Deploy a written installation directly, answering it as the deploy left it. */
async function deployed<Written extends Installation>(
    call: MethodContext,
    written: Written,
): Promise<Written> {
    // deploy it, then read it again, as the deploy observes its status
    await applyDirect(call, written);
    const read = await call.read(installation).findFirst({ where: { id: written.id } });

    return { ...written, ...present(read, `the deployed installation ${written.id}`) };
}

/** Deploy each rollout again once what it waits on arrives. */
function resumeProjection(options: InstallationOptions): ObjectProjection<typeof rollout> {
    return {
        name: "resume",
        watches: [
            ...resumeWatches(options.resources, options.binder.objects()),
            ...(options.space.machine.isBuilding ? followedWatches() : []),
        ],
        reports: {
            object: installation,
            identifiers: (rows) => rows.map((row) => row.installationId),
        },
        project: async (_id, reconciliation) => {
            // read the rollout with its installation, none once uninstalled
            const snapshot = Snapshot.live(reconciliation.database);
            const [target] = await Rollout.read(snapshot, reconciliation.rows);
            if (target === undefined) {
                return;
            }

            // span each step by its space and installation
            const { scope: telemetry } = options.space.machine;
            const attributes = RolloutTarget.attributes(target);

            // pull the release it follows onto the machine and record it as the space's build
            const pulled = await telemetry.span("destack.rollout.pull", attributes, () =>
                RolloutTarget.pull(snapshot, target, options),
            );

            // record a pulled release as the space's build
            const built =
                pulled === undefined || pulled.source.kind !== "release"
                    ? undefined
                    : await Build.release(
                          snapshot,
                          (calls) => reconciliation.mutate(calls),
                          target.scope,
                          pulled.source,
                      );

            // apply it in one call, at the pulled build when one was pulled
            const applied = rollout.target({ scope: target.scope, id: target.installationId });
            await telemetry.span("destack.rollout.apply", attributes, () =>
                reconciliation.mutate([
                    rollout.call("deploy", {
                        ...applied,
                        ...(built === undefined ? {} : { build: built.id }),
                    }),
                ]),
            );

            // relate an application at the universe, as its space
            const rolled = await Snapshot.live(reconciliation.database)
                .read(rollout.table)
                .findMany({ where: { installationId: target.installationId } });
            const [current] = await Rollout.read(Snapshot.live(reconciliation.database), rolled);
            if (current === undefined) {
                return;
            } else if (current.installation.export === null) {
                await telemetry.span("destack.rollout.relate", attributes, () =>
                    new ApplicationInstallation(current, options).relate(reconciliation),
                );
            }

            // then build the commit a followed reference points at
            const { followsId, directory } = current.installation;
            const isFollowing = RolloutTarget.follows(current.installation) === "reference";
            if (options.space.machine.isBuilding && isFollowing) {
                const followed = {
                    id: present(followsId, "the followed reference"),
                    directory: present(directory, "the followed directory"),
                };
                await telemetry.span("destack.rollout.build", attributes, () =>
                    new CommitBuild(current, followed, options).reconcile(reconciliation),
                );
            }

            // keep one change request open for the plan it waits on now, none once none waits
            const [row] = rolled;
            if (row !== undefined) {
                await telemetry.span("destack.rollout.approval", attributes, async () => {
                    const waiting = await Approval.of(
                        Snapshot.live(reconciliation.database),
                        options.resources,
                        current,
                    );
                    await waiting.request(row, reconciliation);
                });
            }

            // fail an installation the space fails to serve, once it followed what could mend it
            const failure = Rollout.servingFailure(current.installation);
            if (failure !== undefined) {
                throw StackError.of(
                    "SERVING_BLOCKED",
                    `cannot serve revision '${current.targetRevisionId}': ${failure.message}`,
                );
            }
        },
    };
}

/** Refuse uninstalling an installation another live installation of its space needs. */
async function requireUnneeded(
    call: MethodContext,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "space">,
): Promise<void> {
    // list the live rollouts outside the target's stack targeting a revision
    const rows = await Rollout.read(
        { read: (table) => call.read(table) },
        await call.read(rollout.table).findMany({ where: { scope: target.scope } }),
    );
    const others = rows.filter(
        (row) =>
            row.installationId !== target.installationId &&
            row.installation.deletedAt === null &&
            row.targetRevisionId !== null &&
            (row.installation.managerInstallationId === null ||
                row.installation.managerInstallationId !==
                    target.installation.managerInstallationId),
    );

    // read their revisions and builds, two reads for the space
    const revisionIds = others.map((row) =>
        present(row.targetRevisionId, `the revision of ${row.installation.name}`),
    );
    const revisions = await call
        .read(installationRevision.table)
        .findMany({ where: { id: { in: revisionIds } } });
    const built = await call
        .read(build.table)
        .findMany({ where: { id: { in: revisions.map((each) => each.buildId) } } });

    // refuse removing a package one of them needs
    for (const row of others) {
        const revision = present(
            revisions.find((each) => each.id === row.targetRevisionId),
            `the revision of ${row.installation.name}`,
        );
        const image = Build.image(
            present(
                built.find((each) => each.id === revision.buildId),
                `the build of ${row.installation.name}`,
            ),
        );
        const reader = await options.space.root(row.installation.packageId, image);
        const description = await PackageDescription.of(reader);
        const needed = Rollout.needed(row.installation, revision, description);
        if (needed.some((entry) => entry.package.id === target.installation.packageId)) {
            throw StackError.of(
                "DEPENDENT_BLOCKED",
                `cannot uninstall '${target.installation.name}': '${row.installation.name}' needs it`,
            );
        }
    }
}

/** Implement rollouts: open views, plans and approvals, and apply. */
export function implementRollouts(options: InstallationOptions) {
    return rollout.implement({
        // apply each rollout again once what it waits on arrives
        projections: [resumeProjection(options)],
        methods: {
            open: async (call) => open(call, await Rollout.of(call), options),
            request: async (call) => request(call, await Rollout.of(call), options),
            plan: async (call) => {
                // join the plans the installation and its managed, owned or bound resources wait on
                const target = await Rollout.of(call);
                const plan = (await Approval.of(call, options.resources, target)).plan();
                const digest = await Plan.digest(plan, call.environment.crypto);

                return { plan, digest, approval: await Approval.asked(call, call.target, digest) };
            },
            deploy: async (call) => {
                // target a stack's evaluated revision, or apply an application whole
                const target = await Rollout.of(call);
                if (target.installation.export !== null) {
                    await new StackInstallation(target, options).deploy(call);
                } else {
                    await new ApplicationInstallation(target, options).deploy(call);
                }

                // record a person deploying as the submitter, who consents to what the revision requests
                const { subject } = call.caller;
                const isSubmitted =
                    principal.user.is(subject) &&
                    Subject.key(subject) !==
                        (target.submittedById === null
                            ? undefined
                            : Subject.key(ObjectReference.at("submittedBy", target)));

                return isSubmitted
                    ? call.update(ObjectReference.columns("submittedBy", subject))
                    : readRollout(call, target);
            },
            follow: {
                handle: (context) =>
                    context.update({
                        spaceStack: context.input.spaceStack,
                        ...ObjectReference.columns("submittedBy", undefined),
                    }),
            },
            approve: async (call) => {
                // accept each plan waiting as the approver reviewed them, then apply
                const target = await Rollout.of(call);
                await (
                    await Approval.of(call, options.resources, target)
                ).approve(call.input.plan, call);
                await call.mutate(call.self, [
                    rollout.call(
                        "deploy",
                        rollout.target({ scope: target.scope, id: target.installationId }),
                    ),
                ]);

                return readRollout(call, target);
            },
        },
    });
}

/** Read a rollout's row as it stands in the call. */
async function readRollout(call: MethodContext, target: InstallationRollout): Promise<Rollout> {
    return present(
        await call
            .read(rollout.table)
            .findFirst({ where: { installationId: target.installationId } }),
        `the rollout of ${target.installationId}`,
    );
}

/** The installations following a moved tag or reference, by the followed row. */
const FOLLOWERS: Relation = {
    table: installation.table,
    cardinality: "many",
    on: { kind: "key", column: "followsId", parent: "id" },
};

/** The spaces of the account an entitlement limits. */
const ENTITLED_SPACES: Relation = {
    table: space.table,
    cardinality: "many",
    on: { kind: "key", column: "scope", parent: "scope" },
};

/** The rollouts of a tag's scope that have not applied the revision they target. */
const WAITING: Relation = {
    table: rollout.table,
    cardinality: "many",
    on: { kind: "key", column: "scope", parent: "scope" },
    where: {
        OR: [
            { appliedRevisionId: { isNull: true } },
            { appliedRevisionId: { ne: { field: "targetRevisionId" } } },
        ],
    },
};

/** Wake a moved tag's or reference's followers and their managers, and a tag's waiting rollouts. */
function followedWatches(): ObjectWatch[] {
    return [
        ...[tag.table, reference.table].flatMap((table): ObjectWatch[] => [
            { table, path: [FOLLOWERS], on: { installationId: "id" } },
            { table, path: [FOLLOWERS], on: { installationId: "managerInstallationId" } },
        ]),
        // a rollout whose plan waits for a release once a tag of its space moves
        { table: tag.table, path: [WAITING], on: { installationId: "installationId" } },
        // the followers of a reference, and their managers, once the build of its commit succeeds
        ...["id", "managerInstallationId"].map((column): ObjectWatch => ({
            table: build.table,
            where: { state: "succeeded" },
            path: [
                {
                    table: reference.table,
                    cardinality: "many",
                    on: { kind: "key", column: "commit", parent: "commit" },
                },
                FOLLOWERS,
            ],
            on: { installationId: column },
        })),
    ];
}

/** Wake a rollout once what it waits on arrives. */
function resumeWatches(
    resources: readonly ResourceType<typeof space>[],
    objects: readonly ObjectBindingKind[],
): ObjectWatch[] {
    return [
        // the rollout once its installation changes or reports, as it reads its conditions
        { table: installation.table, isObserving: true, on: { installationId: "id" } },
        // the stack's rollout once a deleted installation it manages changes or goes
        {
            table: installation.table,
            where: { deletedAt: { isNotNull: true } },
            on: { installationId: "managerInstallationId" },
        },
        // the rollout once it targets or applied another revision, or follows another stack
        {
            table: rollout.table,
            fields: ["targetRevisionId", "appliedRevisionId", "servedRevisionId", "spaceStack"],
            on: { installationId: "installationId" },
        },
        // the rollout once its change request is decided
        {
            table: changeRequest.table,
            where: { parentType: rollout.name },
            fields: ["state"],
            on: { id: "parentId" },
        },
        // the rollout once one of its deployments changes
        { table: deployment.table, on: { installationId: "installationId" } },
        // the installations pinned to a build once it succeeds
        {
            table: build.table,
            where: { state: "succeeded" },
            path: [FOLLOWERS],
            on: { installationId: "id" },
        },
        // the space's installations following a checkout once a build succeeds
        {
            table: build.table,
            where: { state: "succeeded" },
            path: [
                {
                    table: installation.table,
                    cardinality: "many",
                    on: { kind: "key", column: "scope", parent: "scope" },
                    where: { followsType: checkout.name },
                },
            ],
            on: { installationId: "id" },
        },
        // the space's waiting rollouts once a build succeeds, as one may wait for it
        {
            table: build.table,
            where: { state: "succeeded" },
            path: [WAITING],
            on: { installationId: "installationId" },
        },
        // the space's rollouts once its row, its placement or a setting value changes
        { table: space.table, on: { scope: "id" } },
        {
            table: placement.table,
            path: [{ ...WAITING, on: { kind: "key", column: "scope", parent: "spaceId" } }],
            on: { installationId: "installationId" },
        },
        { table: setting.table, on: { scope: "scope" } },
        // the account's rollouts once an entitlement changes
        { table: entitlement.table, path: [ENTITLED_SPACES], on: { scope: "id" } },
        // the space's waiting rollouts once a resource's status moves, and its rollouts once a bindable object changes
        ...resources.map((object): ObjectWatch => ({
            table: object.table,
            fields: ["reference", "appliedStates", "plan", "conditions"],
            isObserving: true,
            path: [WAITING],
            on: { installationId: "installationId" },
        })),
        ...objects.map((kind): ObjectWatch => ({
            table: kind.object.table,
            on: { scope: "scope" },
        })),
    ];
}

/** Describe a permission the installation's package lets it ask its opener for. */
async function request(
    context: MethodContextOf<typeof rollout, "request">,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "accounts" | "space" | "known">,
): Promise<ResultOf<typeof rollout, "request">> {
    // read the applied revision's build, refusing an installation applying none
    const { permission } = context.input;
    const key = PermissionReference.key(permission);
    const revision =
        target.appliedRevisionId === null
            ? undefined
            : await context
                  .read(installationRevision)
                  .findFirst({ where: { id: target.appliedRevisionId } });
    if (revision === undefined) {
        throw StackError.of(
            "PERMISSION_DENIED",
            `permission '${key}' is forbidden: '${target.installation.name}' applies no revision declaring it`,
        );
    }
    const built = await Build.require(context, target.scope, revision.buildId);
    const { packageId } = target.installation;
    const declared = await PackageDescription.of(await options.space.root(packageId, built.image));

    // keep the permission where the package asks it on use or on first use, refusing one it never asks
    const sets = (["user", "optional"] as const).map((delegator) =>
        Rollout.permissions(target.installation, declared, delegator),
    );
    const asked: PermissionSet = Object.fromEntries(
        PermissionScope.options.flatMap((scope) => {
            const found = sets
                .flatMap((set) => set[scope] ?? [])
                .find((each) => PermissionReference.key(each) === key);

            return found === undefined ? [] : [[scope, [found]]];
        }),
    );
    if (Object.keys(asked).length === 0) {
        throw StackError.of(
            "PERMISSION_DENIED",
            `permission '${key}' is forbidden: '${target.installation.name}' never asks a person for it`,
        );
    }

    return (await describeAsked(context, target, asked, options)).permissions;
}

/** Describe what an installation asks its opener for, in the space, their home and the account. */
async function describeAsked(
    context: MethodContextOf<typeof rollout, "open" | "request">,
    target: InstallationRollout,
    asked: PermissionSet,
    options: Pick<InstallationOptions, "accounts" | "known">,
) {
    // read the opening person's home
    const opener = present(
        context.caller.subject,
        `the person opening ${target.installation.name}`,
    );
    if (!principal.user.is(opener)) {
        throw StackError.of("HOME_REQUIRED", `home of '${Subject.key(opener)}' is required`);
    }
    const { homeId: home } = await options.accounts.user.get({
        id: schema.identifier("user").parse(opener.id),
    });

    // grant them in the space, the opener's home and the space's account, each as the opener reads it
    const placed = await Space.read(context, target.scope);
    const permissions = Rollout.describe(
        asked,
        { space: target.scope, home: [opener.id, home], account: placed.scope },
        [...context.objects, ...options.known],
    );

    return { home, permissions };
}

/** Open a view the installation's applied revision declares at a location, in the opener's home. */
async function open(
    context: MethodContextOf<typeof rollout, "open">,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "accounts" | "space" | "known">,
): Promise<ResultOf<typeof rollout, "open">> {
    // require the revision the installation applies, whose build serves its views
    const { view, path } = context.input;
    const revision =
        target.appliedRevisionId === null
            ? undefined
            : await context
                  .read(installationRevision)
                  .findFirst({ where: { id: target.appliedRevisionId } });
    if (revision === undefined) {
        throw StackError.of(
            "VIEW_NOT_FOUND",
            `view '${view}' of '${target.installation.name}' not found`,
        );
    }

    // read the revision's build and the view it mounts
    const built = await Build.require(context, target.scope, revision.buildId);
    const reader = await options.space.root(target.installation.packageId, built.image);
    const mounted = ViewDescription.list(reader.manifest)[view];
    if (mounted === undefined) {
        throw StackError.of(
            "VIEW_NOT_FOUND",
            `view '${view}' of '${target.installation.name}' not found`,
        );
    }

    // describe what the opening person lends the installation, as its package and enabled capabilities ask
    const declared = await PackageDescription.of(reader);
    const asked = Rollout.permissions(target.installation, declared, "user");
    const { home, permissions } = await describeAsked(context, target, asked, options);

    return {
        installationId: target.installationId,
        revisionId: revision.id,
        image: built.image,
        view,
        package: reader.manifest.package.name,
        description: mounted,
        path,
        home,
        permissions,
    };
}
