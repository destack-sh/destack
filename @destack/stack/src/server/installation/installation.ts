import type { ObjectReconciliation } from "@destack/object";
import { type InstallationSelection } from "../../declare/index.ts";
import { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { ObjectReference, Subject } from "@destack/sync";
import { canonicalize, type Digest, type Identifier, nth, present, schema } from "@destack/schema";
import { principal } from "@destack/access";
import { and, eq, inArray, Snapshot } from "@destack/db";
import {
    type MethodContext,
    type MethodContextOf,
    ObjectWatch,
    type ResourceType,
    type ResultOf,
} from "@destack/object";
import { Installation, installation, PackageDescription } from "@destack/object";
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
    type NamedInstallation,
} from "../../object/index.ts";
import { PackageManifest, ViewDescription } from "@destack/package/manifest";
import {
    Application,
    type ApplicationOptions,
    finalizeApplication,
    reconcileApplication,
} from "./application.ts";
import { Approval } from "./approval.ts";
import { SelectedRepository } from "./selection.ts";
import { followRepository, type Forge } from "./follow.ts";
import type { SpaceAccounts } from "../server.ts";
import { reconcileStack, type StackOptions } from "./stack.ts";
import type { TokenIssuer } from "@destack/access/token";
import { InstallationAssertion } from "./token.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** The options a machine serves installations with. */
export interface InstallationOptions extends StackOptions, ApplicationOptions {
    /** The objects of the resource kinds the machine serves. */
    readonly resources: readonly ResourceType<typeof space>[];
    /** The repositories installations select and follow, null on a machine following none. */
    readonly forge: Forge | null;
    /** The account package, reading the home a person opens views in. */
    readonly accounts: Pick<SpaceAccounts, "user">;
    /** The space's token issuer, signing the tokens installations are granted. */
    readonly issuer: TokenIssuer;
}

/** Implement rollouts: apply stacks and deploy applications. */
export function implementInstallation(options: InstallationOptions) {
    return handleRollouts(options).implement({
        controller: {
            pending: {},
            // apply a space's one stack under its space and deploy each application under its own key
            key: rolloutKey,
            watches: [
                ...rolloutWatches(options.resources),
                ...(options.forge === null ? [] : repositoryWatches(options.forge)),
            ],
            reconcile: async (reconciliation) => {
                // apply the space's stack, or deploy the application at the release it selects
                const rows = await Rollout.read(
                    Snapshot.live(reconciliation.database),
                    reconciliation.rows,
                );
                const target = nth(rows, 0);
                const { selection } = target;
                const applied =
                    target.kind === "stack"
                        ? await reconcileStack(target.scope, reconciliation, options)
                        : await reconcileApplication(
                              selection.kind === "release"
                                  ? await followRelease(
                                        { ...target, selection },
                                        reconciliation,
                                        options,
                                    )
                                  : target,
                              reconciliation,
                              options.builds,
                          );

                // then build and submit the commit each repository selection's reference resolves to
                const { forge } = options;
                for (const row of rows) {
                    const followed = row.selection;
                    if (forge !== null && followed.kind === "repository") {
                        await followRepository({ ...row, selection: followed }, reconciliation, {
                            ...options,
                            forge,
                        });
                    }
                }

                return applied;
            },
        },
    });
}

/** Implement the installations' own methods: allowing capabilities and granting tokens. */
export function implementInstallations(options: Pick<InstallationOptions, "issuer">) {
    return installation.implement({
        methods: {
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

/** Handle rollouts: install, uninstall, refuse other accounts' repositories, record submissions. */
function handleRollouts(options: InstallationOptions) {
    return rollout.implement({
        // make the name other installations address a declared one by immutable, and resolve its bindings' targets
        declaration: {
            // follow the types its bindings target, retired before them
            after: options.binder.targets(),
            immutable: ["name"],
            // name the collection after what users declare: installations of packages
            collection: "installations",
            resolve: async (_name, declared, plan) => {
                const { bindings } = declared;

                return bindings === undefined
                    ? declared
                    : { ...declared, bindings: await options.binder.resolve(bindings, plan) };
            },
            // read a declared rollout with its installation's fields, as its declaration lists them
            read: async (row, reader) =>
                Rollout.declared(nth(await Rollout.read(reader, [row]), 0)),
        },
        methods: {
            create: async (call, next) => {
                // refuse another account's repository
                const { selection } = call.input;
                const owner = await SelectedRepository.owner(options.forge, selection);
                await SelectedRepository.require(call, call.scope, selection, owner);

                // install the package as declared by the space's stack or by the caller, then follow it
                const { packageId, name, status, compute, capabilities, directories, ...followed } =
                    call.input;
                const id = await Installation.install(call, call.scope, {
                    origin: call.manager === null ? "installed" : "declared",
                    packageId,
                    name,
                    ...(status === undefined ? {} : { status }),
                    ...(compute === undefined ? {} : { compute }),
                    ...(capabilities === undefined ? {} : { capabilities }),
                    ...(directories === undefined ? {} : { directories }),
                });

                return next(call.with({ ...followed, id }));
            },
            update: async (call, next) => {
                // refuse another account's repository
                const { selection } = call.input;
                const owner = await SelectedRepository.owner(options.forge, selection);
                await SelectedRepository.require(call, call.scope, selection, owner);

                // update the installation's fields through the installation, the rest on the rollout
                const { status, compute, capabilities, directories } = call.input;
                const fields = {
                    ...(status === undefined ? {} : { status }),
                    ...(compute === undefined ? {} : { compute }),
                    ...(capabilities === undefined ? {} : { capabilities }),
                    ...(directories === undefined ? {} : { directories }),
                };
                if (Object.keys(fields).length > 0) {
                    await call.mutate(call.self, [
                        installation.call("update", {
                            ...installation.in(call.target.scope),
                            id: call.target.id,
                            ...fields,
                        }),
                    ]);
                }

                return next(call);
            },
            open: async (call) => open(call, await Rollout.of(call), options),
            plan: async (call) => {
                // join the plans the installation and its managed, owned or bound resources wait on
                const target = await Rollout.of(call);

                return (await Approval.of(call, options.resources, target)).plan();
            },
            submit: {
                // read the owner of a repository the submission selects anew, and pull a release it selects
                prepare: async (context) => ({
                    owner: await SelectedRepository.owner(
                        options.forge,
                        canonicalize(context.input.selection) ===
                            canonicalize(context.target.selection)
                            ? undefined
                            : context.input.selection,
                    ),
                    release: await pull(await Rollout.of(context), context.input.build, options),
                }),
                handle: async (context) =>
                    submit(context, await Rollout.of(context), context.prepared, options),
            },
            deploy: async (call) => {
                await new Application(call, await Rollout.of(call), options).deploy();

                return {};
            },
            delete: async (call, next) => {
                // list the live rollouts outside the target's stack following a revision
                const target = await Rollout.of(call);
                const rows = await Rollout.read(
                    { read: (table) => call.read(table) },
                    await call
                        .read(rollout.table)
                        .findMany({ where: { scope: target.scope, deletedAt: { isNull: true } } }),
                );
                const others = rows.filter(
                    (row) =>
                        row.id !== target.id &&
                        row.revisionId !== null &&
                        (row.managerInstallationId === null ||
                            row.managerInstallationId !== target.managerInstallationId),
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
                    const reader = await options.builds.packages.contents(manifest);
                    const description = await PackageDescription.of(reader);
                    const needed = Rollout.needed(row.installation, revision, description);
                    if (
                        needed.some((entry) => entry.package.id === target.installation.packageId)
                    ) {
                        throw StackError.of(
                            "NEEDED_BY_DEPENDENT",
                            `cannot uninstall ${target.installation.name}: ${row.installation.name} needs it`,
                        );
                    }
                }

                return next(call);
            },
            finalize: async (call, next) => {
                // remove an application's deployments, resources and grants, then its installation
                const target = await Rollout.of(call);
                await finalizeApplication(call, target, options);
                const finished = await next();
                await call.mutate(call.self, [
                    installation.call("delete", {
                        ...installation.in(target.scope),
                        id: target.id,
                    }),
                ]);

                return finished;
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

/** Record a submitted build as the installation's revision and follow it. */
async function submit(
    context: MethodContextOf<typeof rollout, "submit">,
    target: Rollout & { readonly installation: NamedInstallation },
    prepared: SubmissionPreparation,
    options: InstallationOptions,
): Promise<Rollout> {
    // refuse another account's repository
    const { input: submission } = context;
    await SelectedRepository.require(context, context.scope, submission.selection, prepared.owner);

    // require a stack's evaluation for a stack and none for an application
    const { selection, evaluation } = submission;
    const isStack = target.kind === "stack";
    if (isStack !== (evaluation !== undefined)) {
        throw StackError.of(
            "SUBMISSION_KIND_MISMATCH",
            isStack
                ? `${target.installation.name} is a stack, submitted with its evaluation`
                : `${target.installation.name} is an application, submitted without a stack's evaluation`,
        );
    }

    // resolve the submitted build and require a build of the installation's package
    const resolved = await resolveBuild(context, target, submission.build, prepared.release);
    const reader = await options.builds.packages.contents(resolved.manifest);
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
            evaluation,
            context.environment.crypto,
        ),
    ]);

    // point the installation at the selection, arguments, revision and space's stack submitted
    const { spaceStack } = submission;
    const followed = {
        selection,
        ...(evaluation === undefined
            ? {}
            : { export: evaluation.export, parameters: evaluation.parameters }),
        revisionId: revision.id,
        ...(spaceStack === undefined ? {} : { spaceStack }),
    };
    const current = {
        selection: target.selection,
        ...(evaluation === undefined
            ? {}
            : { export: target.export, parameters: target.parameters }),
        revisionId: target.revisionId,
        ...(spaceStack === undefined ? {} : { spaceStack: target.spaceStack }),
    };
    if (canonicalize(followed) === canonicalize(current)) {
        return context.target;
    }

    // record its submitter, who consents to what it requests
    const submittedBy = ObjectReference.columns("submittedBy", context.caller.subject);

    return context.update({ ...followed, ...submittedBy });
}

/** Pull the release a submission selects by tag or version, none for a space's build. */
async function pull(
    target: Rollout & { readonly installation: NamedInstallation },
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
    target: Rollout & { readonly installation: NamedInstallation },
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

/** Key a stack's rollout under its space and an application's rollout by itself. */
function rolloutKey(row: {
    readonly id: Identifier<"installation">;
    readonly scope: string;
    readonly kind: Rollout["kind"];
}) {
    return row.kind === "stack" ? { scope: row.scope, kind: "stack" } : { id: row.id };
}

/** Wake the installations a moved reference or a finished commit build selects. */
function repositoryWatches(forge: Forge) {
    return [
        // wake the installations of every served space selecting a moved reference
        ObjectWatch.of(forge.references, async (row, database) => {
            const { repository, name } = forge.reference(row);
            // key the installations of every space the database serves selecting the reference
            const selecting = await Rollout.selecting(
                database,
                undefined,
                (selection) => selection.repository === repository && selection.reference === name,
            );

            return selecting.map(rolloutKey);
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
                    selection.repository === source.repository &&
                    selection.directory === source.directory,
            );

            return selecting.map(rolloutKey);
        }),
    ];
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
                    .select({ conditions: rollout.table.conditions })
                    .from(rollout.table)
                    .where(
                        and(eq(rollout.table.scope, row.scope), eq(rollout.table.kind, "stack")),
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
                    ...(isWaiting ? [{ scope: row.scope, kind: "stack" }] : []),
                    ...bound.map((entry) => ({ id: entry.id })),
                ];
            }),
        ),
        // wake the stack once a rollout it manages goes, since its teardown waits for that rollout
        ObjectWatch.of(rollout.table, (row) =>
            row.managerInstallationId === null || row.deletedAt === null
                ? []
                : [{ scope: row.scope, kind: "stack" }],
        ),
        // wake the applications waiting for their space to serve an installation's revision
        ObjectWatch.of(rollout.table, async (row, database) => {
            // leave a rollout whose revision the space does not serve yet
            if (row.revisionId === null || row.servedRevisionId !== row.revisionId) {
                return [];
            }

            // find the space's applications waiting for it to serve them or what they need
            const rollouts = await database
                .select({ id: rollout.table.id, conditions: rollout.table.conditions })
                .from(rollout.table)
                .where(and(eq(rollout.table.scope, row.scope), eq(rollout.table.kind, "package")));

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
                .select({ id: rollout.table.id, conditions: rollout.table.conditions })
                .from(rollout.table)
                .where(
                    inArray(
                        rollout.table.scope,
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
                    conditions: rollout.table.conditions,
                    dependencies: installationRevision.table.dependencies,
                })
                .from(rollout.table)
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
                .select({
                    id: rollout.table.id,
                    scope: rollout.table.scope,
                    kind: rollout.table.kind,
                })
                .from(rollout.table)
                .where(eq(rollout.table.scope, spaceId));

            return rows.map(rolloutKey);
        }),
    ];
}

/** Follow the release an application selects, answering the installation as it follows it. */
async function followRelease(
    row: Rollout & {
        readonly installation: NamedInstallation;
        readonly selection: Extract<InstallationSelection, { readonly kind: "release" }>;
    },
    reconciliation: ObjectReconciliation<typeof rollout>,
    options: InstallationOptions,
): Promise<Rollout & { readonly installation: NamedInstallation }> {
    // resolve the release's manifest, and leave a release whose revision the installation follows
    const { version } = row.selection;
    const manifest =
        row.selection.manifest ??
        (await options.registry.find(row.installation.packageId, version));
    const { database } = reconciliation;
    const followed =
        row.revisionId === null
            ? undefined
            : await Snapshot.live(database)
                  .read(installationRevision.table)
                  .findFirst({ where: { id: row.revisionId } });
    if (
        followed?.digest ===
        (await InstallationRevision.digest(
            manifest,
            undefined,
            reconciliation.store.environment.crypto,
        ))
    ) {
        return row;
    }

    // pull the release before the transactions, then record it as the space's build, retain its revision once, and follow it
    await options.builds.pull(options.registry, row.installation.packageId, manifest);
    const pulled = await Build.release(
        Snapshot.live(database),
        (calls) => reconciliation.mutate(calls),
        row.scope,
        { package: row.installation.packageId, version, manifest },
    );
    const [{ id }] = await reconciliation.mutate([
        await InstallationRevision.create(
            row.installation,
            pulled,
            undefined,
            reconciliation.store.environment.crypto,
        ),
    ]);
    if (row.revisionId === id) {
        return row;
    }
    const [following] = await reconciliation.mutate([
        rollout.call("follow", { ...rollout.target(row), revisionId: id }),
    ]);

    return { ...row, ...following };
}

/** Open a view the installation's applied revision declares at a location, in the opener's home. */
async function open(
    context: MethodContextOf<typeof rollout, "open">,
    target: Rollout & { readonly installation: NamedInstallation },
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
    const reader = await options.builds.packages.contents(built.manifest);
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
