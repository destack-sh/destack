import { Setting } from "@destack/setting";
import { Plan } from "@destack/resource";
import { setting, SettingValue } from "@destack/setting/object";
import { ObjectReference, Subject } from "@destack/sync";
import { canonicalize, type Digest, type Identifier, present, schema } from "@destack/schema";
import {
    InstallationId,
    PermissionReference,
    PermissionScope,
    type PermissionSet,
    principal,
} from "@destack/access";
import { and, Change, eq, inArray, isNotNull, ne, Snapshot } from "@destack/db";
import {
    installation,
    type MethodContext,
    type MethodContextOf,
    type ObjectProjection,
    ObjectWatch,
    PackageDescription,
    type ResourceType,
    type ResultOf,
    SYNCED,
} from "@destack/object";
import { entitlement, Space, space } from "@destack/account/object";
import {
    Binding,
    build,
    deployment,
    installationRevision,
    Rollout,
    WaitingReason,
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
import { StackInstallation } from "./stack.ts";
import type { TokenIssuer } from "@destack/access/token";
import { StackError } from "../../error/index.ts";
import type { ObjectBindable } from "../resource/binding.ts";
import { RolloutTarget, type RevisionOptions } from "./target.ts";

/** The options a machine serves installations with. */
export interface InstallationOptions extends RevisionOptions, ApplicationOptions {
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
                // refuse another account's repository
                const { selection, name } = call.input;
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
                    rollout.call("create", {
                        ...rollout.in(call.scope),
                        installationId: created.id,
                    }),
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
            target: async (call) => {
                // refuse a revision of another installation, then target it
                const { revisionId } = call.input;
                const revision = await call
                    .read(installationRevision)
                    .findFirst({ where: { id: revisionId, installationId: call.target.id } });
                if (revision === undefined) {
                    throw StackError.of(
                        "REVISION_NOT_FOUND",
                        `installation ${call.target.id} has no revision ${revisionId}`,
                    );
                }

                return call.update({ revisionId });
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

/** Roll out each installation: apply the space's stack, or deploy an application. */
function rolloutProjection(options: InstallationOptions): ObjectProjection<typeof rollout> {
    return {
        watches: [
            // project a new rollout, a moved served revision, and a followed space stack, past its own applied and consumed records
            ObjectWatch.of(rollout.table, (row, _database, change) =>
                change.operation !== "update" ||
                change.before.servedRevisionId !== change.after.servedRevisionId ||
                change.before.spaceStack !== change.after.spaceStack
                    ? [rolloutKey(row)]
                    : [],
            ),
            ...rolloutWatches(options.resources, options.binder.objects()),
            ...(options.forge === null ? [] : forgeWatches(options.forge)),
        ],
        reports: {
            object: installation,
            identifiers: (rows) => rows.map((row) => row.installationId),
        },
        project: async (_id, reconciliation) => {
            // read the rollout with its installation, none once uninstalled
            const rows = await Rollout.read(
                Snapshot.live(reconciliation.database),
                reconciliation.rows,
            );
            const [target] = rows;
            if (target === undefined) {
                return;
            }

            // apply the space's stack, or deploy the application at the release it selects
            if (target.installation.export !== null) {
                await new StackInstallation(target, options).reconcile(reconciliation);
            } else {
                await new ApplicationInstallation(target, options).reconcile(reconciliation);
            }

            // then build the commit a repository selection's reference resolves to, submitting an unmanaged installation's
            const { forge } = options;
            const { selection } = target.installation;
            if (forge !== null && selection.kind === "repository") {
                await new CommitBuild(target, selection, { ...options, forge }).reconcile(
                    reconciliation,
                );
            }

            // keep one change request open for the plan it waits on now, none once none waits
            const snapshot = Snapshot.live(reconciliation.database);
            const rolled = await snapshot
                .read(rollout.table)
                .findMany({ where: { installationId: target.installationId } });
            const [current] = await Rollout.read(snapshot, rolled);
            const [row] = rolled;
            if (current !== undefined && row !== undefined) {
                const waiting = await Approval.of(snapshot, options.resources, current);
                await waiting.request(row, reconciliation);
            }
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
            row.installationId !== target.installationId &&
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
        projections: [rolloutProjection(options)],
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
                        ...(await RolloutTarget.describe(
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
            follow: {
                handle: (context) =>
                    context.update({
                        spaceStack: context.input.spaceStack,
                        ...ObjectReference.columns("submittedBy", undefined),
                    }),
            },
            deploy: async (call) => {
                await new ApplicationInstallation(await Rollout.of(call), options).deploy(call);

                return {};
            },
            revise: async (call) => {
                // revise the installation: its build at its bindings, and target the revision
                const target = await Rollout.of(call);
                const revision = await new ApplicationInstallation(target, options).revise(
                    call,
                    call.input,
                );
                if (target.revisionId !== revision.id) {
                    await call.mutate(call.self, [
                        installation.call("target", {
                            ...installation.target(target.installation),
                            revisionId: revision.id,
                        }),
                    ]);
                }

                return { revisionId: revision.id };
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
    // require a stack's export for a stack and none for an application
    const { selection, stack, build: submitted } = context.input;
    const isStack = target.installation.export !== null;
    if (isStack !== (stack !== undefined)) {
        throw StackError.of(
            "SUBMISSION_KIND_MISMATCH",
            isStack
                ? `${target.installation.name} is a stack, submitted with its export`
                : `${target.installation.name} is an application, submitted without a stack's export`,
        );
    }

    // resolve the submitted build and require a build of the installation's package
    const resolved = await resolveBuild(context, target, submitted, prepared.release);
    const reader = await options.builds.contents(resolved.manifest);
    const built = PackageManifest.package(reader.manifest);
    if (built.id !== target.installation.packageId) {
        throw StackError.of(
            "PACKAGE_MISMATCH",
            `the submitted build is of ${built.name}, which ${target.installation.name} does not install`,
        );
    }

    // refuse a build whose space settings without a default the space leaves unset, before any apply
    const placement = { scope: target.scope, installation: target.installationId };
    for (const declared of await Setting.list(reader)) {
        const { definition } = declared;
        if (definition.scope === "space" && "required" in definition) {
            await SettingValue.resolve(context, declared, placement);
        }
    }

    // evaluate a stack's export of the build here, and retain the revision once
    const evaluation =
        stack === undefined
            ? undefined
            : await RolloutTarget.evaluate(
                  await RolloutTarget.inputs(context, { ...target.installation, ...stack }),
                  reader,
                  options,
              );
    const revision = await new ApplicationInstallation(target, options).revise(context, {
        build: resolved.id,
        dependencies: prepared.dependencies,
        states: prepared.states,
        evaluation,
    });

    // select the submitted build and arguments on the installation
    const selected = { selection, ...stack };
    const { installation: installed } = target;
    const current = {
        selection: installed.selection,
        ...(stack === undefined
            ? {}
            : { export: installed.export, parameters: installed.parameters }),
    };
    if (canonicalize(selected) !== canonicalize(current)) {
        await context.mutate(context.self, [
            installation.call("update", {
                ...installation.in(target.scope),
                id: target.installationId,
                ...selected,
            }),
        ]);
    }

    // target the revision, recording the submitter, who consents to what it requests
    if (target.revisionId === revision.id) {
        return context.target;
    }
    await context.mutate(context.self, [
        installation.call("target", {
            ...installation.target(target.installation),
            revisionId: revision.id,
        }),
    ]);

    return context.update(ObjectReference.columns("submittedBy", context.caller.subject));
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
    const reader = await builds.pull(target.installation.packageId, manifest);

    // refuse a manifest of another version
    const version = reader.manifest.package.version;
    if (reference.kind === "release" && reference.version !== version) {
        throw StackError.of(
            "VERSION_MISMATCH",
            `expected ${reader.manifest.package.name} ${reference.version}, got version ${version}`,
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
function rolloutKey(row: { readonly installationId: Identifier<"installation"> }) {
    return { installationId: row.installationId };
}

/** Key an installation following a source and its manager, which targets a managed one's build. */
function followerKeys(row: {
    readonly id: Identifier<"installation">;
    readonly managerInstallationId: Identifier<"installation"> | null;
}) {
    return [
        { installationId: row.id },
        ...(row.managerInstallationId === null
            ? []
            : [{ installationId: row.managerInstallationId }]),
    ];
}

/** Wake the installations a moved tag, a moved reference or a finished commit build selects. */
function forgeWatches(forge: Forge) {
    return [
        // wake the installations of every served space selecting a moved tag of its package
        ObjectWatch.of(forge.tags, async (row, database) => {
            const tagged = forge.tag(row);
            const selecting = await RolloutTarget.selecting(
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
            const selecting = await RolloutTarget.selecting(
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
            const selecting = await RolloutTarget.selecting(
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
        const selecting = await RolloutTarget.selecting(
            database,
            row.scope,
            (selection) =>
                selection.kind === "checkout" &&
                selection.machine === source.machine &&
                selection.checkout === source.checkout &&
                selection.directory === source.directory,
        );

        return selecting.flatMap((entry) =>
            entry.managerInstallationId === null
                ? []
                : [{ installationId: entry.managerInstallationId }],
        );
    });
}

/** Wake rollouts on the changes of their bound targets, deployments and placement. */
function rolloutWatches(
    resources: readonly ResourceType<typeof space>[],
    objects: readonly ObjectBindable[],
) {
    return [
        // wake the space's stack while it waits for its resources and the applications binding one, as their status moves too
        ...resources.map((object) =>
            ObjectWatch.of(
                object.table,
                async (row, database) => {
                    // leave a resource of an account, which no installation binds
                    if (!space.identifies(row.scope)) {
                        return [];
                    }

                    // find the space's stack, which reads its resources however it last stood
                    const [stack] = await database
                        .select({ id: rollout.table.installationId })
                        .from(rollout.table)
                        .innerJoin(
                            installation.table,
                            eq(installation.table.id, rollout.table.installationId),
                        )
                        .where(
                            and(
                                eq(rollout.table.scope, row.scope),
                                isNotNull(installation.table.export),
                            ),
                        );

                    // find the applications whose targeted revision binds it, which revise again at its new version
                    const scope = schema.identifier("space").parse(row.scope);
                    const bound = await Binding.binders(Snapshot.live(database), scope, row.id);

                    return [
                        ...(stack === undefined ? [] : [{ installationId: stack.id }]),
                        ...bound.map((id) => ({ installationId: id })),
                    ];
                },
                { isObserving: true },
            ),
        ),
        // wake an installation's rollout and its stack once it comes, goes or changes, past its observed status
        ObjectWatch.of(installation.table, (row, _database, change) =>
            change.operation === "update" &&
            change.before["generation"] === change.after["generation"]
                ? []
                : [
                      { installationId: row.id },
                      ...(row.managerInstallationId === null
                          ? []
                          : [{ installationId: row.managerInstallationId }]),
                  ],
        ),
        // wake the space's other rollouts still waiting once one serves another revision, as what they wait for may be served now
        ObjectWatch.of(rollout.table, async (row, database, change) => {
            // leave a change that serves no other revision
            const before = Change.before(change);
            if (before !== null && before.servedRevisionId === row.servedRevisionId) {
                return [];
            }

            // find the space's other rollouts, with whether each is ready
            const rollouts = await database
                .select({
                    id: rollout.table.installationId,
                    conditions: installation.table.conditions,
                })
                .from(rollout.table)
                .innerJoin(
                    installation.table,
                    eq(installation.table.id, rollout.table.installationId),
                )
                .where(
                    and(
                        eq(rollout.table.scope, row.scope),
                        ne(rollout.table.installationId, row.installationId),
                    ),
                );

            return rollouts
                .filter((entry) => entry.conditions["Ready"]?.status !== "true")
                .map((entry) => ({ installationId: entry.id }));
        }),
        // wake a space's rollouts once its row arrives or changes, as the space serves its applications only then
        ObjectWatch.of(space.table, async (row, database) => {
            const rollouts = await database
                .select({ id: rollout.table.installationId })
                .from(rollout.table)
                .where(eq(rollout.table.scope, row.id));

            return rollouts.map((entry) => ({ installationId: entry.id }));
        }),
        ObjectWatch.of(deployment.table, (row) => [{ installationId: row.installationId }]),
        // wake the applications whose targeted revision binds an object of a served kind, which revise again at its new version
        ...objects.map((kind) =>
            ObjectWatch.of(kind.object.table, async (row, database) => {
                // read the object's space and identifier, leaving one outside spaces
                const scope = schema.identifier("space").safeParse(row["scope"]);
                const id = row["id"];
                if (!scope.success || typeof id !== "string") {
                    return [];
                }
                const bound = await Binding.binders(Snapshot.live(database), scope.data, id);

                return bound.map((installationId) => ({ installationId }));
            }),
        ),
        // wake the failed applications of the account's spaces, as an unentitled capability may be entitled now
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
                .filter((each) => each.conditions[SYNCED]?.status === "false")
                .map((each) => ({ installationId: each.id }));
        }),
        // wake the space's applications waiting for a dependency once its package is installed or served
        ObjectWatch.of(
            installation.table,
            async (row, database) => {
                // leave an installation outside a space, which no application of a space waits for
                const scope = schema.identifier("space").safeParse(row.scope);
                if (!scope.success) {
                    return [];
                }

                // read the space's rollouts with their conditions and dependencies
                const refused = await database
                    .select({
                        id: rollout.table.installationId,
                        conditions: installation.table.conditions,
                        dependencies: installationRevision.table.dependencies,
                    })
                    .from(rollout.table)
                    .innerJoin(
                        installation.table,
                        eq(installation.table.id, rollout.table.installationId),
                    )
                    .innerJoin(
                        installationRevision.table,
                        eq(installationRevision.table.id, installation.table.revisionId),
                    )
                    .where(eq(rollout.table.scope, scope.data));

                return refused
                    .filter(
                        (each) =>
                            (each.conditions[SYNCED]?.status === "false" ||
                                each.conditions["Ready"]?.reason ===
                                    WaitingReason.enum.WaitingForSpace) &&
                            each.dependencies.some(
                                (dependency) => dependency.package.id === row.packageId,
                            ),
                    )
                    .map((each) => ({ installationId: each.id }));
            },
            { isObserving: true },
        ),
        // wake the space's applications waiting for a setting of its package once a value is placed
        ObjectWatch.of(setting.table, async (row, database) => {
            // leave a value outside a space, which no application of a space waits for
            const scope = schema.identifier("space").safeParse(row.scope);
            if (!scope.success) {
                return [];
            }

            // read the space's rollouts with their packages and conditions
            const waiting = await database
                .select({
                    id: rollout.table.installationId,
                    packageId: installation.table.packageId,
                    conditions: installation.table.conditions,
                })
                .from(rollout.table)
                .innerJoin(
                    installation.table,
                    eq(installation.table.id, rollout.table.installationId),
                )
                .where(eq(rollout.table.scope, scope.data));

            return waiting
                .filter(
                    (each) =>
                        each.packageId === row.packageId &&
                        each.conditions["Ready"]?.reason === WaitingReason.enum.WaitingForSettings,
                )
                .map((each) => ({ installationId: each.id }));
        }),
        // wake the space's stack and applications once its placement changes
        Space.watch(async (spaceId, database) => {
            const rows = await database
                .select({ installationId: rollout.table.installationId })
                .from(rollout.table)
                .where(eq(rollout.table.scope, spaceId));

            return rows.map(rolloutKey);
        }),
    ];
}

/** Describe a permission the installation's package lets it ask its opener for. */
async function request(
    context: MethodContextOf<typeof rollout, "request">,
    target: InstallationRollout,
    options: Pick<InstallationOptions, "accounts" | "builds" | "known">,
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
            "PERMISSION_UNDECLARED",
            `${target.installation.name} applies no revision declaring ${key}`,
        );
    }
    const built = await Build.require(context, target.scope, revision.buildId);
    const declared = await PackageDescription.of(await options.builds.contents(built.manifest));

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
            "PERMISSION_UNDECLARED",
            `${target.installation.name} never asks a person for ${key}`,
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
        throw StackError.of("HOME_MISSING", `${Subject.key(opener)} has no home`);
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
    options: Pick<InstallationOptions, "accounts" | "builds" | "known">,
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

    // describe what the opening person lends the installation, as its package and enabled capabilities ask
    const declared = await PackageDescription.of(reader);
    const asked = Rollout.permissions(target.installation, declared, "user");
    const { home, permissions } = await describeAsked(context, target, asked, options);

    return {
        installationId: target.installationId,
        revisionId: revision.id,
        manifest: built.manifest,
        view,
        package: reader.manifest.package.name,
        description: mounted,
        path,
        home,
        permissions,
    };
}
