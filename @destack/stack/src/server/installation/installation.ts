import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { ObjectReference, Subject } from "@destack/sync";
import { canonicalize, type Digest, type Identifier, present, schema } from "@destack/schema";
import { InstallationId, principal } from "@destack/access";
import { and, type Change, eq, inArray, isNotNull, isNull, Snapshot } from "@destack/db";
import {
    installation,
    type MethodContext,
    type MethodContextOf,
    ObjectController,
    ObjectError,
    type ObjectReconciliation,
    ObjectWatch,
    PackageDescription,
    type ResourceType,
    type ResultOf,
} from "@destack/object";
import { entitlement, Space, space } from "@destack/account/object";
import {
    binding,
    build,
    deployment,
    installationRevision,
    InstallationRevision,
    Rollout,
    rollout,
    type SubmissionPreparation,
    Build,
    type Submission,
    type InstallationRollout,
} from "../../object/index.ts";
import { PackageManifest, ViewDescription } from "@destack/package/manifest";
import { type ApplicationOptions, ApplicationInstallation } from "./application.ts";
import { Approval } from "./approval.ts";
import { SelectedRepository } from "./selection.ts";
import { CommitBuild, type Forge } from "./follow.ts";
import type { SpaceAccounts } from "../server.ts";
import { StackInstallation, type StackOptions } from "./stack.ts";
import type { TokenIssuer } from "@destack/access/token";
import { InstallationAssertion } from "./token.ts";
import { StackError } from "../../error/index.ts";

/** The options a machine serves installations with. */
export interface InstallationOptions extends StackOptions, ApplicationOptions {
    /** The objects of the resource kinds the machine serves. */
    readonly resources: readonly ResourceType<typeof space>[];
    /** The account package, reading the home a person opens views in. */
    readonly accounts: Pick<SpaceAccounts, "user">;
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
            resolve: async (_name, declared, plan) => {
                const { bindings } = declared;

                return bindings === undefined
                    ? declared
                    : { ...declared, bindings: await options.binder.resolve(bindings, plan) };
            },
        },
        methods: {
            create: async (call, next) => {
                // refuse an installation without a name, which only a scope's base installations lack
                const { selection, name } = call.input;
                if (name === undefined || name === null) {
                    throw ObjectError.of("FIELD_MISSING", "an installation needs a name");
                }

                // refuse another account's repository
                const owner = await SelectedRepository.owner(options.forge, selection);
                await SelectedRepository.require(call, call.scope, selection, owner);

                // identify it by its scope and name, as readers derive it, then roll it out as its installer
                const id = await InstallationId.of(
                    { id: call.scope },
                    name,
                    call.environment.crypto,
                );
                const created = await next(call.identify(id));
                await call.mutate(call.caller, [
                    rollout.call("create", { ...rollout.in(call.scope), id: created.id }),
                ]);

                return created;
            },
            update: async (call, next) => {
                // refuse another account's repository
                const { selection } = call.input;
                if (selection !== undefined) {
                    const owner = await SelectedRepository.owner(options.forge, selection);
                    await SelectedRepository.require(call, call.scope, selection, owner);
                }

                return next(call);
            },
            delete: async (call, next) => {
                // refuse uninstalling one another installation needs, then mark it uninstalling for its controller to drain
                const [rolled] = await Rollout.installed(call, [call.target]);
                if (rolled !== undefined) {
                    await requireUnneeded(call, rolled, options);
                }

                return next(call);
            },
            token: (call) => InstallationAssertion.token(call, options.issuer),
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

/** Roll out each installation: apply the space's stack, or deploy an application. */
function rolloutController(options: InstallationOptions) {
    return {
        pending: {},
        key: rolloutKey,
        // reconcile a new rollout and a moved target or served revision, past its own applied and consumed records
        wakes: (change: Change<typeof rollout.table>) =>
            change.operation !== "update" ||
            change.before.revisionId !== change.after.revisionId ||
            change.before.servedRevisionId !== change.after.servedRevisionId,
        watches: [
            ...rolloutWatches(options.resources),
            ...(options.forge === null ? [] : forgeWatches(options.forge)),
        ],
        reconcile: async (reconciliation: ObjectReconciliation<typeof rollout>) => {
            // read the rollout with its installation, none once uninstalled
            const rows = await Rollout.read(
                Snapshot.live(reconciliation.database),
                reconciliation.rows,
            );
            const [target] = rows;
            if (target === undefined) {
                return undefined;
            }

            // apply the space's stack, or deploy the application at the release it selects
            const applied =
                target.installation.export !== null
                    ? await new StackInstallation(target, options).reconcile(reconciliation)
                    : await new ApplicationInstallation(target, options).reconcile(reconciliation);

            // then build the commit a repository selection's reference resolves to, submitting an unmanaged installation's
            const { forge } = options;
            const { selection } = target.installation;
            if (forge !== null && selection.kind === "repository") {
                await new CommitBuild(target, selection, { ...options, forge }).reconcile(
                    reconciliation,
                );
            }

            return applied;
        },
    };
}

/** Refuse uninstalling an installation another live installation of its space needs. */
async function requireUnneeded(
    call: MethodContext,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "builds">,
): Promise<void> {
    // list the live rollouts outside the target's stack targeting a revision
    const rows = await Rollout.read(
        { read: (table) => call.read(table) },
        await call.read(rollout.table).findMany({ where: { scope: target.scope } }),
    );
    const others = rows.filter(
        (row) =>
            row.id !== target.id &&
            row.installation.deletedAt === null &&
            row.revisionId !== null &&
            (row.installation.managerInstallationId === null ||
                row.installation.managerInstallationId !==
                    target.installation.managerInstallationId),
    );

    // read their revisions and builds, two reads for the space
    const revisionIds = others.map((row) =>
        present(row.revisionId, `the revision of ${row.installation.name}`),
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
            revisions.find((each) => each.id === row.revisionId),
            `the revision of ${row.installation.name}`,
        );
        const manifest = Build.manifest(
            present(
                built.find((each) => each.id === revision.buildId),
                `the build of ${row.installation.name}`,
            ),
        );
        const reader = await options.builds.contents(manifest);
        const description = await PackageDescription.of(reader);
        const needed = Rollout.needed(row.installation, revision, description);
        if (needed.some((entry) => entry.package.id === target.installation.packageId)) {
            throw StackError.of(
                "NEEDED_BY_DEPENDENT",
                `cannot uninstall ${target.installation.name}: ${row.installation.name} needs it`,
            );
        }
    }
}

/** Implement rollouts: open views, plans and approvals, submissions, roll back and deploy. */
export function implementRollouts(options: InstallationOptions) {
    return rollout.implement({
        // apply a space's stack and deploy each application
        controller: rolloutController(options),
        methods: {
            open: async (call) => open(call, await Rollout.of(call), options),
            plan: async (call) => {
                // join the plans the installation and its managed, owned or bound resources wait on
                const target = await Rollout.of(call);

                return (await Approval.of(call, options.resources, target)).plan();
            },
            submit: {
                // refuse a foreign repository, then pull the selected release and describe its revision
                prepare: async (context) => {
                    // refuse another account's repository the submission selects anew
                    const target = await Rollout.of(context);
                    const { build: submitted, selection } = context.input;
                    const isReselected =
                        canonicalize(selection) !== canonicalize(target.installation.selection);
                    if (isReselected) {
                        const owner = await SelectedRepository.owner(options.forge, selection);
                        await SelectedRepository.require(context, context.scope, selection, owner);
                    }

                    // pull a selected release, and describe the submitted build's revision
                    const release = await pull(target, submitted, options);
                    const manifest =
                        submitted.kind === "build"
                            ? (await Build.require(context, target.scope, submitted.build)).manifest
                            : present(release, `the pulled release of ${target.installation.name}`)
                                  .manifest;
                    const reader = await options.builds.contents(manifest);

                    return {
                        release,
                        ...(await InstallationRevision.describe(
                            target.installation,
                            reader,
                            context.objects,
                            options,
                        )),
                    };
                },
                handle: async (context) =>
                    submit(context, await Rollout.of(context), context.prepared, options),
            },
            deploy: async (call) => {
                await new ApplicationInstallation(await Rollout.of(call), options).deploy(call);

                return {};
            },
            approve: async (call) => {
                // accept each plan waiting as the approver reviewed them
                const target = await Rollout.of(call);
                const { plan } = call.input;

                return (await Approval.of(call, options.resources, target)).approve(plan, call);
            },
        },
    });
}

/** Record a submitted build as the installation's revision and target it. */
async function submit(
    context: MethodContextOf<typeof rollout, "submit">,
    target: InstallationRollout,
    prepared: SubmissionPreparation,
    options: InstallationOptions,
): Promise<Rollout> {
    // require a stack's evaluation for a stack and none for an application
    const { selection, evaluation, build: submitted, spaceStack } = context.input;
    const isStack = target.installation.export !== null;
    if (isStack !== (evaluation !== undefined)) {
        throw StackError.of(
            "SUBMISSION_KIND_MISMATCH",
            isStack
                ? `${target.installation.name} is a stack, submitted with its evaluation`
                : `${target.installation.name} is an application, submitted without a stack's evaluation`,
        );
    }

    // resolve the submitted build and require a build of the installation's package
    const resolved = await resolveBuild(context, target, submitted, prepared.release);
    const reader = await options.builds.contents(resolved.manifest);
    const built = PackageManifest.package(reader.manifest).id;
    if (built !== target.installation.packageId) {
        throw StackError.of(
            "PACKAGE_MISMATCH",
            `${target.installation.name} installs ${target.installation.packageId}, but the submitted build is ${built}`,
        );
    }

    // refuse a build whose space settings without a default the space leaves unset, before any apply
    const placement = { scope: target.scope, installation: target.id };
    for (const setting of await Setting.list(reader)) {
        const { definition } = setting;
        if (definition.scope === "space" && "required" in definition) {
            await SettingValue.resolve(context, setting, placement);
        }
    }

    // retain the evaluation once
    const [revision] = await context.mutate(context.self, [
        await InstallationRevision.create(
            target.installation,
            resolved,
            { dependencies: prepared.dependencies, states: prepared.states },
            evaluation,
            context.environment.crypto,
        ),
    ]);

    // select the submitted build and arguments on the installation
    const selected = {
        selection,
        ...(evaluation === undefined
            ? {}
            : { export: evaluation.export, parameters: evaluation.parameters }),
    };
    const { installation: installed } = target;
    const current = {
        selection: installed.selection,
        ...(evaluation === undefined
            ? {}
            : { export: installed.export, parameters: installed.parameters }),
    };
    if (canonicalize(selected) !== canonicalize(current)) {
        await context.mutate(context.self, [
            installation.call("update", {
                ...installation.in(target.scope),
                id: target.id,
                ...selected,
            }),
        ]);
    }

    // target the revision and the space's stack submitted, recording the submitter, who consents to what it requests
    const isTargeted =
        target.revisionId === revision.id &&
        (spaceStack === undefined || target.spaceStack === spaceStack);
    if (isTargeted) {
        return context.target;
    }
    const submittedBy = ObjectReference.columns("submittedBy", context.caller.subject);

    return context.update({
        revisionId: revision.id,
        ...(spaceStack === undefined ? {} : { spaceStack }),
        ...submittedBy,
    });
}

/** Pull the release a submission selects by tag or version, none for a space's build. */
async function pull(
    target: InstallationRollout,
    reference: Submission["build"],
    options: Pick<InstallationOptions, "builds" | "registry">,
): Promise<SubmissionPreparation["release"]> {
    // take a space's build as it is
    if (reference.kind === "build") {
        return null;
    }

    // read the manifest a tag or version selects, and pull it
    const { builds, registry } = options;
    const manifest =
        reference.kind === "tag"
            ? await registry.find(target.installation.packageId, reference.tag)
            : (reference.manifest ??
              (await registry.find(target.installation.packageId, reference.version)));
    const reader = await builds.pull(registry, target.installation.packageId, manifest);

    // refuse a manifest of another version
    const version = reader.manifest.package.version;
    if (reference.kind === "release" && reference.version !== version) {
        throw StackError.of(
            "VERSION_MISMATCH",
            `expected ${target.installation.packageId} ${reference.version}, got version ${version}`,
        );
    }

    return { manifest, version };
}

/** Resolve a submitted build: a succeeded build of the space, or the pulled release. */
async function resolveBuild(
    context: MethodContext,
    target: InstallationRollout,
    reference: Submission["build"],
    release: SubmissionPreparation["release"],
): Promise<{ readonly id: Identifier<"build">; readonly manifest: Digest }> {
    // take a succeeded build of the space
    if (reference.kind === "build") {
        return Build.require(context, target.scope, reference.build);
    }

    // record the pulled release as the space's build
    const pulled = present(release, `the pulled release of ${target.installation.packageId}`);

    return Build.release(context, (calls) => context.mutate(context.self, calls), target.scope, {
        package: target.installation.packageId,
        version: pulled.version,
        manifest: pulled.manifest,
    });
}

/** Key a rollout by its installation. */
function rolloutKey(row: { readonly id: Identifier<"installation"> }) {
    return { id: row.id };
}

/** Key an installation following a source and its manager, which targets a managed one's build. */
function followerKeys(row: {
    readonly id: Identifier<"installation">;
    readonly managerInstallationId: Identifier<"installation"> | null;
}) {
    return [
        { id: row.id },
        ...(row.managerInstallationId === null ? [] : [{ id: row.managerInstallationId }]),
    ];
}

/** Wake the installations a moved tag, a moved reference or a finished commit build selects. */
function forgeWatches(forge: Forge) {
    return [
        // wake the installations of every served space selecting a moved tag of its package
        ObjectWatch.of(forge.tags, async (row, database) => {
            const tagged = forge.tag(row);
            const selecting = await Rollout.selecting(
                database,
                undefined,
                (selection) => selection.kind === "tag" && selection.tag === tagged.name,
            );

            return selecting
                .filter((each) => each.packageId === tagged.package)
                .flatMap(followerKeys);
        }),
        // wake the installations of every served space selecting a moved reference
        ObjectWatch.of(forge.references, async (row, database) => {
            const { repository, name } = forge.reference(row);
            // key the installations of every space the database serves selecting the reference
            const selecting = await Rollout.selecting(
                database,
                undefined,
                (selection) =>
                    selection.kind === "repository" &&
                    selection.repository === repository &&
                    selection.reference === name,
            );

            return selecting.flatMap(followerKeys);
        }),
        // wake the installations of the space selecting the directory a finished commit build compiled
        ObjectWatch.of(build.table, async (row, database) => {
            // leave unfinished builds and builds of other sources
            const { source } = row;
            if (source.kind !== "commit" || !Build.isFinished(row)) {
                return [];
            }

            // key the space's installations selecting its directory
            const selecting = await Rollout.selecting(
                database,
                row.scope,
                (selection) =>
                    selection.kind === "repository" &&
                    selection.repository === source.repository &&
                    selection.directory === source.directory,
            );

            return selecting.flatMap(followerKeys);
        }),
        checkoutWatch(),
    ];
}

/** Wake the managers of the installations selecting a checkout a finished build compiled. */
function checkoutWatch() {
    return ObjectWatch.of(build.table, async (row, database) => {
        // leave unfinished builds and builds of other sources
        const { source } = row;
        if (source.kind !== "checkout" || !Build.isFinished(row)) {
            return [];
        }

        // key the managers of the space's installations selecting its directory
        const selecting = await Rollout.selecting(
            database,
            row.scope,
            (selection) =>
                selection.kind === "checkout" &&
                selection.machine === source.machine &&
                selection.checkout === source.checkout &&
                selection.directory === source.directory,
        );

        return selecting.flatMap((entry) =>
            entry.managerInstallationId === null ? [] : [{ id: entry.managerInstallationId }],
        );
    });
}

/** Wake rollouts on the changes of their resources, deployments, bindings and placement. */
function rolloutWatches(resources: readonly ResourceType<typeof space>[]) {
    return [
        // wake the space's stack while it waits for its resources and the applications binding a changed resource
        ...resources.map((object) =>
            ObjectWatch.of(object.table, async (row, database) => {
                // leave a resource of an account, which no installation binds
                if (!space.identifies(row.scope)) {
                    return [];
                }

                // find the waiting stack
                const [stack] = await database
                    .select({ id: rollout.table.id, conditions: installation.table.conditions })
                    .from(rollout.table)
                    .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
                    .where(
                        and(
                            eq(rollout.table.scope, row.scope),
                            isNotNull(installation.table.export),
                        ),
                    );
                const isWaiting = stack?.conditions["Ready"]?.reason === "WaitingForResources";

                // find the applications binding it
                const bound = await database
                    .select({ id: binding.table.installationId })
                    .from(binding.table)
                    .where(
                        and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)),
                    );

                return [
                    ...(stack !== undefined && isWaiting ? [{ id: stack.id }] : []),
                    ...bound.map((entry) => ({ id: entry.id })),
                ];
            }),
        ),
        // wake an installation's rollout and its stack once it comes, goes or changes, past its observed status
        ObjectWatch.of(installation.table, (row, _database, change) =>
            ObjectController.isObserved(installation, change)
                ? []
                : [
                      { id: row.id },
                      ...(row.managerInstallationId === null
                          ? []
                          : [{ id: row.managerInstallationId }]),
                  ],
        ),
        // wake the applications waiting for their space to serve an installation's revision
        ObjectWatch.of(rollout.table, async (row, database) => {
            // leave a rollout whose revision the space does not serve yet
            if (row.revisionId === null || row.servedRevisionId !== row.revisionId) {
                return [];
            }

            // find the space's applications waiting for it to serve them or what they need
            const rollouts = await database
                .select({ id: rollout.table.id, conditions: installation.table.conditions })
                .from(rollout.table)
                .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
                .where(and(eq(rollout.table.scope, row.scope), isNull(installation.table.export)));

            return rollouts
                .filter((entry) => entry.conditions["Ready"]?.reason === "WaitingForSpace")
                .map((entry) => ({ id: entry.id }));
        }),
        ObjectWatch.of(deployment.table, (row) => [{ id: row.installationId }]),
        ObjectWatch.of(binding.table, (row) => [{ id: row.installationId }]),
        // wake the applications of the account's spaces refused for an unentitled capability
        ObjectWatch.of(entitlement.table, async (row, database) => {
            const spaces = await Space.within(database, row.scope);
            const refused = await database
                .select({ id: installation.table.id, conditions: installation.table.conditions })
                .from(installation.table)
                .where(
                    inArray(
                        installation.table.scope,
                        spaces.map((each) => schema.string().parse(each["id"])),
                    ),
                );

            return refused
                .filter((each) => each.conditions["Ready"]?.reason === "UnentitledCapability")
                .map((each) => ({ id: each.id }));
        }),
        // wake the space's applications waiting for a dependency once its package is installed or served
        ObjectWatch.of(installation.table, async (row, database) => {
            // leave an installation outside a space, which no application of a space waits for
            const scope = schema.identifier("space").safeParse(row.scope);
            if (!scope.success) {
                return [];
            }

            // read the space's rollouts with their conditions and dependencies
            const refused = await database
                .select({
                    id: rollout.table.id,
                    conditions: installation.table.conditions,
                    dependencies: installationRevision.table.dependencies,
                })
                .from(rollout.table)
                .innerJoin(installation.table, eq(installation.table.id, rollout.table.id))
                .innerJoin(
                    installationRevision.table,
                    eq(installationRevision.table.id, rollout.table.revisionId),
                )
                .where(eq(rollout.table.scope, scope.data));

            return refused
                .filter(
                    (each) =>
                        ["UninstalledDependency", "WaitingForSpace"].includes(
                            each.conditions["Ready"]?.reason ?? "",
                        ) &&
                        each.dependencies.some((release) => release.package.id === row.packageId),
                )
                .map((each) => ({ id: each.id }));
        }),
        // wake the space's stack and applications once its placement changes
        Space.watch(async (spaceId, database) => {
            const rows = await database
                .select({ id: rollout.table.id })
                .from(rollout.table)
                .where(eq(rollout.table.scope, spaceId));

            return rows.map(rolloutKey);
        }),
    ];
}

/** Open a view the installation's applied revision declares at a location, in the opener's home. */
async function open(
    context: MethodContextOf<typeof rollout, "open">,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "accounts" | "builds">,
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
            `${target.installation.name} declares no view ${view}`,
        );
    }

    // read the revision's build and the view it mounts
    const built = await Build.require(context, target.scope, revision.buildId);
    const reader = await options.builds.contents(built.manifest);
    const mounted = ViewDescription.list(reader.manifest)[view];
    if (mounted === undefined) {
        throw StackError.of(
            "VIEW_NOT_FOUND",
            `${target.installation.name} declares no view ${view}`,
        );
    }

    // read the opening person's home
    const opener = present(
        context.caller.subject,
        `the person opening ${target.installation.name}`,
    );
    if (!principal.user.is(opener)) {
        throw StackError.of("HOME_MISSING", `${Subject.key(opener)} has no home`);
    }
    const { homeId: home } = await options.accounts.user.get({
        id: schema.identifier("user").parse(opener.id),
    });

    // read what the opening person lends the installation, as its package and enabled capabilities ask
    const declared = await PackageDescription.of(reader);

    return {
        installationId: target.id,
        revisionId: revision.id,
        manifest: built.manifest,
        view,
        description: mounted,
        path,
        home,
        permissions: Rollout.permissions(target.installation, declared, "user"),
    };
}
