import {
    Installation,
    installation,
    build,
    deployment,
    installationRevision,
    Build,
} from "@destack/stack/object";
import { InstallationDefinition } from "@destack/stack/declare";
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import { Plan } from "@destack/resource";
import { setting } from "@destack/setting/object";
import { ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
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
    type MethodContext,
    type MethodContextOf,
    type ObjectProjection,
    type ObjectReconciliation,
    type ObjectReport,
    type ObjectType,
    type ObjectWatch,
    PackageDescription,
    type ResourceType,
    type ResultOf,
    changeRequest,
} from "@destack/object";
import { entitlement, placement, Space, space } from "@destack/account/object";
import { ViewDescription } from "@destack/package/manifest";
import { type ApplicationOptions, ApplicationInstallation } from "./application.ts";
import { Approval } from "./approval.ts";
import { waitedTypes } from "./wait.ts";
import { checkout } from "../computer/checkout/object/index.ts";
import { CommitBuild } from "./follow.ts";
import { reference, tag } from "@destack/registry/object";
import { StackInstallation } from "./stack.ts";
import type { TokenIssuer } from "@destack/access/token";
import { StackError } from "@destack/stack/error";
import { InstallationTarget, type RevisionOptions } from "./target.ts";

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
        // reconcile each installation once its spec changes, and again once what it waits on arrives
        projections: [reconcileProjection(options)],
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
                const { follows, directory } = await InstallationTarget.resolveDeclared(
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
                await options.space.requireFollowable(InstallationTarget.followed(call.input));

                // identify it by its scope and name, as readers derive it, then roll it out as its installer
                const id = await InstallationId.of(
                    { id: call.scope },
                    name,
                    call.environment.crypto,
                );
                const created = await next(call.identify(id));

                return await deployed(call, created);
            },
            update: async (call, next) => {
                // refuse another account's repository, then apply what it follows now
                await options.space.requireFollowable(
                    InstallationTarget.followed({ ...call.target, ...call.input }),
                );
                const updated = await next(call);

                return await deployed(call, updated);
            },
            delete: async (call, next) => {
                // refuse uninstalling one another installation needs, then drain it
                await requireUnneeded(call, call.target, options);
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
            open: async (call) => open(call, call.target, options),
            request: async (call) => request(call, call.target, options),
            plan: async (call) => {
                // join the plans the installation and its managed, owned or bound resources wait on
                const target = call.target;
                const plan = (await Approval.of(call, options.resources, target)).plan();
                const digest = await Plan.digest(plan, call.environment.crypto);

                return { plan, digest, approval: await Approval.asked(call, call.target, digest) };
            },
            reconcile: async (call) => {
                // reconcile a stack with what its document declares, or an application
                const target = call.target;
                if (target.export !== null) {
                    await new StackInstallation(target, options).reconcile(call, call.input.build);
                } else {
                    await new ApplicationInstallation(target, options).reconcile(call);
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
                    ? call.updateStatus(ObjectReference.columns("submittedBy", subject))
                    : stored(call);
            },
            follow: {
                handle: (context) =>
                    context.updateStatus({
                        spaceStack: context.input.spaceStack,
                        ...ObjectReference.columns("submittedBy", undefined),
                    }),
            },
            approve: async (call) => {
                // accept each plan waiting as the approver reviewed them, then apply
                const target = call.target;
                await (
                    await Approval.of(call, options.resources, target)
                ).approve(call.input.plan, call);
                await call.mutate(call.self, [
                    installation.call(
                        "reconcile",
                        installation.target({ scope: target.scope, id: target.id }),
                    ),
                ]);

                return stored(call);
            },
        },
    });
}

/** Reconcile an installation written directly, leaving a declared one to its stack. */
async function applyDirect(
    call: MethodContext,
    installed: Pick<Installation, "id" | "scope" | "managerInstallationId">,
): Promise<void> {
    if (installed.managerInstallationId === null) {
        await call.mutate(call.self, [
            installation.call(
                "reconcile",
                installation.target({ scope: installed.scope, id: installed.id }),
            ),
        ]);
    }
}

/** Reconcile a written installation directly, answering it as the reconcile left it. */
async function deployed<Written extends Installation>(
    call: MethodContext,
    written: Written,
): Promise<Written> {
    // deploy it, then read it again, as the deploy observes its status
    await applyDirect(call, written);
    const read = await call.read(installation).findFirst({ where: { id: written.id } });

    return { ...written, ...present(read, `the deployed installation ${written.id}`) };
}

/** Read a call's installation after its writes, as loaded once the call removed it. */
async function stored(
    call: MethodContext & { readonly target: Installation },
): Promise<Installation> {
    const read = await call.read(installation.table).findFirst({ where: { id: call.target.id } });

    return read ?? call.target;
}

/** The installations a pass reports Synced on: all but those their space refuses. */
const REPORTS: ObjectReport<typeof installation> = {
    object: installation,
    identifiers: (rows) =>
        rows.filter((row) => Installation.servingFailure(row) === undefined).map((row) => row.id),
};

/** Reconcile an installation once its spec changes or something it waits on arrives. */
function reconcileProjection(options: InstallationOptions): ObjectProjection<typeof installation> {
    return {
        name: "reconcile",
        // key by identifier, as a name addresses an installation only within its space
        key: "id",
        watches: [
            // the installation once its spec changes, unless the call changing it reconciled it
            {
                table: installation.table,
                where: { generation: { gt: { field: "syncedGeneration" } } },
                on: { id: "id" },
            },
            // a stack without a target once it follows its space's stack
            {
                table: installation.table,
                where: { export: { isNotNull: true }, targetRevisionId: { isNull: true } },
                fields: ["spaceStack"],
                isObserving: true,
                on: { id: "id" },
            },
            ...resumeWatches(waitedTypes(options)),
            ...(options.space.machine.isBuilding ? followedWatches() : []),
        ],
        reports: REPORTS,
        project: async (id, reconciliation) => {
            // leave an uninstalled one
            const [target] = reconciliation.rows;
            if (target === undefined) {
                return;
            }

            // reconcile it, leaving a failure of one uninstalled meanwhile, as nothing is left to reconcile
            try {
                await resume(target, reconciliation, options);
            } catch (error) {
                const left = await Snapshot.live(reconciliation.database)
                    .read(installation.table)
                    .findFirst({ where: { id: installation.identifier(id) } });
                if (left !== undefined) {
                    throw error;
                }
            }
        },
    };
}

/** Pull what an installation follows, reconcile it, then relate it, build and ask approval. */
async function resume(
    target: Installation,
    reconciliation: ObjectReconciliation<typeof installation>,
    options: InstallationOptions,
): Promise<void> {
    // span each step by its space and installation
    const snapshot = Snapshot.live(reconciliation.database);
    const { scope: telemetry } = options.space.machine;
    const attributes = InstallationTarget.attributes(target);

    // pull the release it follows onto the machine
    const pulled = await telemetry.span("destack.installation.pull", attributes, () =>
        InstallationTarget.pull(snapshot, target, options),
    );

    // record a pulled release as the space's build
    const released =
        pulled !== undefined && pulled.source.kind === "release" ? pulled.source : undefined;
    const built =
        released === undefined
            ? undefined
            : await telemetry.span("destack.installation.record", attributes, () =>
                  Build.release(
                      snapshot,
                      (calls) => reconciliation.mutate(calls),
                      Installation.space(target),
                      released,
                  ),
              );

    // reconcile it in one call, at the pulled build when one was pulled, recording how it left it
    const reconciled = installation.target({ scope: target.scope, id: target.id });
    const current = await telemetry.span(
        "destack.installation.reconcile",
        attributes,
        async (span) => {
            // reconcile it
            await reconciliation.mutate([
                installation.call("reconcile", {
                    ...reconciled,
                    ...(built === undefined ? {} : { build: built.id }),
                }),
            ]);

            // record its readiness and what it waits for
            const left = await Snapshot.live(reconciliation.database)
                .read(installation.table)
                .findFirst({ where: { id: target.id } });
            span.setAttributes({
                "destack.installation.ready": left?.conditions["Ready"]?.reason ?? "",
                "destack.installation.wait": left?.waitType ?? "",
            });

            return left;
        },
    );

    // relate an application at the universe, as its space
    if (current === undefined) {
        return;
    } else if (current.export === null) {
        await telemetry.span("destack.installation.relate", attributes, () =>
            new ApplicationInstallation(current, options).relate(reconciliation),
        );
    }

    // then build the commit a followed reference points at
    const { followsId, directory } = current;
    const isFollowing = InstallationTarget.follows(current) === "reference";
    if (options.space.machine.isBuilding && isFollowing) {
        const followed = {
            id: present(followsId, "the followed reference"),
            directory: present(directory, "the followed directory"),
        };
        await telemetry.span("destack.installation.build", attributes, () =>
            new CommitBuild(current, followed, options).reconcile(reconciliation),
        );
    }

    // keep one change request open for the plan it waits on now, none once none waits
    await telemetry.span("destack.installation.approval", attributes, async () => {
        const waiting = await Approval.of(
            Snapshot.live(reconciliation.database),
            options.resources,
            current,
        );
        await waiting.request(current, reconciliation);
    });
}

/** Refuse uninstalling an installation another live installation of its space needs. */
async function requireUnneeded(
    call: MethodContext,
    target: Installation,
    options: Pick<InstallationOptions, "space">,
): Promise<void> {
    // list the live installations outside the target's stack targeting a revision
    const rows = await call.read(installation.table).findMany({ where: { scope: target.scope } });
    const others = rows.filter(
        (row) =>
            row.id !== target.id &&
            row.deletedAt === null &&
            row.targetRevisionId !== null &&
            (row.managerInstallationId === null ||
                row.managerInstallationId !== target.managerInstallationId),
    );

    // read their revisions and builds, two reads for the space
    const revisionIds = others.map((row) =>
        present(row.targetRevisionId, `the revision of ${row.name}`),
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
            `the revision of ${row.name}`,
        );
        const image = Build.image(
            present(
                built.find((each) => each.id === revision.buildId),
                `the build of ${row.name}`,
            ),
        );
        const reader = await options.space.root(row.packageId, image);
        const description = await PackageDescription.of(reader);
        const needed = Installation.needed(row, revision, description);
        if (needed.some((entry) => entry.package.id === target.packageId)) {
            throw StackError.of(
                "DEPENDENT_BLOCKED",
                `cannot uninstall '${target.name}': '${row.name}' needs it`,
            );
        }
    }
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

/** The installations of a tag's scope that have not applied the revision they target. */
const WAITING: Relation = {
    table: installation.table,
    cardinality: "many",
    on: { kind: "key", column: "scope", parent: "scope" },
    where: {
        OR: [
            { appliedRevisionId: { isNull: true } },
            { appliedRevisionId: { ne: { field: "targetRevisionId" } } },
        ],
    },
};

/** Wake the followers of a moved tag or reference, and a tag's waiters. */
function followedWatches(): ObjectWatch[] {
    return [
        ...[tag.table, reference.table].map((table): ObjectWatch => ({
            table,
            path: [FOLLOWERS],
            on: { id: "id" },
        })),
        // an installation whose plan waits for a release once a tag of its space moves
        { table: tag.table, path: [WAITING], on: { id: "id" } },
        // the followers of a reference once the build of its commit succeeds
        {
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
            on: { id: "id" },
        },
    ];
}

/** Wake an installation once something outside it that it waits on arrives. */
function resumeWatches(waited: readonly ObjectType[]): ObjectWatch[] {
    return [
        // the installations waiting for a row of a type once a row meeting what one waits for changes
        ...waited
            .filter((object) => !object.same(installation))
            .map((object): ObjectWatch => ({
                table: object.table,
                isObserving: true,
                matches: {
                    column: "waitCondition",
                    where: { waitType: ObjectTypeReference.key(object.typeReference) },
                },
            })),
        // the installations waiting for another once that one targets a revision or the space serves it
        {
            table: installation.table,
            fields: ["targetRevisionId", "servedRevisionId"],
            isObserving: true,
            matches: {
                column: "waitCondition",
                where: { waitType: ObjectTypeReference.key(installation.typeReference) },
            },
        },
        // the installation once its change request is decided
        {
            table: changeRequest.table,
            where: { parentType: installation.name },
            fields: ["state"],
            on: { id: "parentId" },
        },
        // the installation once one of its deployments retires, as a drained or uninstalled one waits for it
        {
            table: deployment.table,
            where: { status: "retired" },
            fields: ["status"],
            on: { id: "installationId" },
        },
        // the installations pinned to a build once it succeeds
        {
            table: build.table,
            where: { state: "succeeded" },
            path: [FOLLOWERS],
            on: { id: "id" },
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
            on: { id: "id" },
        },
        // the space's waiting installations once a build succeeds, as one may wait for it
        {
            table: build.table,
            where: { state: "succeeded" },
            path: [WAITING],
            on: { id: "id" },
        },
        // the space's installations once its row, its placement or a setting value changes
        { table: space.table, on: { scope: "id" } },
        {
            table: placement.table,
            path: [{ ...WAITING, on: { kind: "key", column: "scope", parent: "spaceId" } }],
            on: { id: "id" },
        },
        { table: setting.table, on: { scope: "scope" } },
        // the account's installations once an entitlement changes
        { table: entitlement.table, path: [ENTITLED_SPACES], on: { scope: "id" } },
    ];
}

/** Describe a permission the installation's package lets it ask its opener for. */
async function request(
    context: MethodContextOf<typeof installation, "request">,
    target: Installation,
    options: Pick<InstallationOptions, "accounts" | "space" | "known">,
): Promise<ResultOf<typeof installation, "request">> {
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
            `permission '${key}' is forbidden: '${target.name}' applies no revision declaring it`,
        );
    }
    const built = await Build.require(context, Installation.space(target), revision.buildId);
    const { packageId } = target;
    const declared = await PackageDescription.of(await options.space.root(packageId, built.image));

    // keep the permission where the package asks it on use or on first use, refusing one it never asks
    const sets = (["user", "optional"] as const).map((delegator) =>
        Installation.permissions(target, declared, delegator),
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
            `permission '${key}' is forbidden: '${target.name}' never asks a person for it`,
        );
    }

    return (await describeAsked(context, target, asked, options)).permissions;
}

/** Describe what an installation asks its opener for, in the space, their home and the account. */
async function describeAsked(
    context: MethodContextOf<typeof installation, "open" | "request">,
    target: Installation,
    asked: PermissionSet,
    options: Pick<InstallationOptions, "accounts" | "known">,
) {
    // read the opening person's home
    const opener = present(context.caller.subject, `the person opening ${target.name}`);
    if (!principal.user.is(opener)) {
        throw StackError.of("HOME_REQUIRED", `home of '${Subject.key(opener)}' is required`);
    }
    const { homeId: home } = await options.accounts.user.get({
        id: schema.identifier("user").parse(opener.id),
    });

    // grant them in the space, the opener's home and the space's account, each as the opener reads it
    const placed = await Space.read(context, target.scope);
    const permissions = Installation.describe(
        asked,
        { space: target.scope, home: [opener.id, home], account: placed.scope },
        [...context.objects, ...options.known],
    );

    return { home, permissions };
}

/** Open a view the installation's applied revision declares at a location, in the opener's home. */
async function open(
    context: MethodContextOf<typeof installation, "open">,
    target: Installation,
    options: Pick<InstallationOptions, "accounts" | "space" | "known">,
): Promise<ResultOf<typeof installation, "open">> {
    // require the revision the installation applies, whose build serves its views
    const { view, path } = context.input;
    const revision =
        target.appliedRevisionId === null
            ? undefined
            : await context
                  .read(installationRevision)
                  .findFirst({ where: { id: target.appliedRevisionId } });
    if (revision === undefined) {
        throw StackError.of("VIEW_NOT_FOUND", `view '${view}' of '${target.name}' not found`);
    }

    // read the revision's build and the view it mounts
    const built = await Build.require(context, Installation.space(target), revision.buildId);
    const reader = await options.space.root(target.packageId, built.image);
    const mounted = ViewDescription.list(reader.manifest)[view];
    if (mounted === undefined) {
        throw StackError.of("VIEW_NOT_FOUND", `view '${view}' of '${target.name}' not found`);
    }

    // describe what the opening person lends the installation, as its package and enabled capabilities ask
    const declared = await PackageDescription.of(reader);
    const asked = Installation.permissions(target, declared, "user");
    const { home, permissions } = await describeAsked(context, target, asked, options);

    return {
        installationId: target.id,
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
