import { ObjectReference, Subject } from "@destack/sync";
import { nth, canonicalize, Digest, type Identifier, present, schema } from "@destack/schema";
import { principal } from "@destack/access";
import { and, eq, inArray } from "@destack/db";
import {
    type MethodContext,
    type MethodContextOf,
    ObjectWatch,
    type ResourceType,
    type ResultOf,
} from "@destack/object";
import { PackageDescription } from "@destack/object";
import type { PackageId } from "@destack/package";
import { CallError } from "@destack/object";
import { entitlement, Space, space } from "@destack/account/object";
import {
    binding,
    build,
    deployment,
    Installation,
    installation,
    installationRevision,
    InstallationRevision,
    Build,
    type Submission,
} from "../../object/index.ts";
import { PackageManifest, ViewDescription } from "@destack/package/manifest";
import {
    Application,
    type ApplicationOptions,
    finalizeApplication,
    reconcileApplication,
} from "./application.ts";
import { Approval } from "./approval.ts";
import type { InstallationSelection } from "../../declare/installation.ts";
import { SelectedRepository } from "./selection.ts";
import { followRepository, type Forge } from "./follow.ts";
import type { SpaceAccounts } from "../server.ts";
import { reconcileStack, type StackOptions } from "./stack.ts";
import { StackError } from "../../error/index.ts";
import type { TokenIssuer } from "@destack/access/token";
import { InstallationAssertion } from "./token.ts";

/** Installations a stack declares. */
const declaredInstallation = installation.implement({
    declaration: {
        // keep the name other installations address the installation by
        immutable: ["name"],
        values: (_name, desired) => ({
            packageId: desired.package.id,
            selection: { kind: "release" as const, version: desired.package.version },
            name: desired.name,
            status: desired.status,
            compute: desired.compute,
            capabilities: desired.capabilities,
            directories: desired.directories,
            tags: desired.tags,
        }),
        changed: async (transaction, row, desired) => {
            // report a release with a revision the installation does not follow yet
            if (row.revisionId === null) {
                return true;
            }
            const followed = await transaction.read(installationRevision.table).findFirst({
                where: {
                    id: row.revisionId,
                },
            });

            return (
                followed?.digest !==
                (await InstallationRevision.digest(declaredRelease(desired).manifest))
            );
        },
        written: async (context, row, desired) => {
            // keep the release as the space's build, at the manifest the stack pinned
            const release = declaredRelease(desired);
            const pulled = await Build.release(
                context.transaction.view(),
                (calls) => context.mutate(calls),
                row.scope,
                release,
            );

            // retain its revision once and follow it
            const [{ id }] = await context.mutate([await InstallationRevision.create(row, pulled)]);
            if (row.revisionId !== id) {
                await context.mutate([
                    declaredInstallation.call("follow", { id: row.id, revisionId: id }),
                ]);
            }
        },
    },
});

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

/** A create or update call selecting a release, repository or checkout. */
interface SelectingCall extends Pick<MethodContext, "read"> {
    /** The space the installation lives in. */
    readonly scope: string;
    /** The selection the call writes, absent for one leaving it. */
    readonly input: { readonly selection?: InstallationSelection | undefined };
    /** The account owning a selected repository, null for another selection. */
    readonly prepared: Identifier<"account"> | null;
}

/** Implement installations: apply stacks and deploy applications. */
export function implementInstallation(options: InstallationOptions) {
    return handleInstallations(options).implement({
        controller: {
            pending: {},
            // apply a space's one stack under its space and deploy each application under its own key
            key: (row) =>
                row.role === "stack" ? { scope: row.scope, role: "stack" } : { id: row.id },
            watches: [
                ...installationWatches(options.resources),
                ...(options.forge === null ? [] : repositoryWatches(options.forge)),
            ],
            reconcile: async (reconciliation) => {
                // apply the space's stack, or deploy the application
                const target = nth(reconciliation.rows, 0);
                const applied =
                    target.role === "stack"
                        ? await reconcileStack(target.scope, reconciliation, options)
                        : await reconcileApplication(target, reconciliation);

                // then build and submit the commit each repository selection's reference resolves to
                const { forge } = options;
                for (const row of reconciliation.rows) {
                    const { selection } = row;
                    if (forge !== null && selection.kind === "repository") {
                        await followRepository({ ...row, selection }, reconciliation, {
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

/** Handle installations: refuse selections of other accounts' repositories, record submissions. */
function handleInstallations(options: InstallationOptions) {
    return declaredInstallation.implement({
        methods: {
            open: (call) => open(call, options),
            token: (call) => InstallationAssertion.token(call, options.issuer),
            create: handleSelection(options),
            update: handleSelection(options),
            plan: async (call) => {
                // join the plans the installation and what it manages, owns or binds wait on
                const { target } = call;

                return (await Approval.of(call, options.resources, target)).plan();
            },
            submit: {
                // read the owner of a repository the submission selects anew
                prepare: (context) =>
                    SelectedRepository.owner(
                        options.forge,
                        canonicalize(context.input.selection) ===
                            canonicalize(context.target.selection)
                            ? undefined
                            : context.input.selection,
                    ),
                handle: (context) => submit(context, context.prepared, options),
            },
            allow: (call) =>
                call.update({
                    allowed: [...new Set([...call.target.allowed, call.input.capability])],
                }),
            disallow: (call) =>
                call.update({
                    allowed: call.target.allowed.filter((name) => name !== call.input.capability),
                }),
            deploy: async (call) => {
                await new Application(call, options).deploy();

                return {};
            },
            delete: async (call, next) => {
                // list the live installations outside the target's stack following a revision
                const { target } = call;
                const rows = await call
                    .read(installation.table)
                    .findMany({ where: { scope: target.scope } });
                const others = rows.filter(
                    (row) =>
                        row.id !== target.id &&
                        row.revisionId !== null &&
                        (row.managerInstallationId === null ||
                            row.managerInstallationId !== target.managerInstallationId),
                );

                // read their revisions and builds, two reads for the space
                const revisionIds = others.map((row) =>
                    present(row.revisionId, `the revision of ${row.name}`),
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
                        `the revision of ${row.name}`,
                    );
                    const manifest = Build.manifest(
                        present(
                            built.find((each) => each.id === revision.buildId),
                            `the build of ${row.name}`,
                        ),
                    );
                    const reader = await options.builds.packages.contents(manifest);
                    const description = await PackageDescription.of(reader);
                    const needed = Installation.needed(row, revision, description);
                    if (needed.some((entry) => entry.package.id === target.packageId)) {
                        throw new CallError("PRECONDITION_FAILED", {
                            message: `${row.name} needs ${target.name}, which stays installed while it does`,
                        });
                    }
                }

                return next(call);
            },
            finalize: async (call, next) => {
                // remove an application's deployments, resources and grants before the installation
                await finalizeApplication(call, options);

                return next();
            },
            approve: async (call) => {
                // accept each plan waiting as the approver reviewed them
                const { target } = call;
                const { plan } = call.input;

                return (await Approval.of(call, options.resources, target)).approve(plan, call);
            },
        },
    });
}

/** Handle a create or update: read the owner of the repository it selects. */
function handleSelection(options: Pick<InstallationOptions, "forge">) {
    return {
        prepare: (call: {
            readonly input: { readonly selection?: InstallationSelection | undefined };
        }) => SelectedRepository.owner(options.forge, call.input.selection),
        handle: async <Selecting extends SelectingCall, Result>(
            call: Selecting,
            next: (call: Selecting) => Promise<Result>,
        ): Promise<Result> => {
            // refuse another account's repository
            const { scope, input, prepared } = call;
            await SelectedRepository.require(call, scope, input.selection, prepared);

            return next(call);
        },
    };
}

/** Record a submitted build as the installation's revision and follow it. */
async function submit(
    context: MethodContextOf<typeof installation, "submit">,
    owner: Identifier<"account"> | null,
    options: InstallationOptions,
): Promise<Installation> {
    // refuse another account's repository
    const { target, input: submission } = context;
    await SelectedRepository.require(context, context.scope, submission.selection, owner);

    // require a stack's evaluation for a stack and none for an application
    const { selection, evaluation } = submission;
    const isStack = target.role === "stack";
    if (isStack !== (evaluation !== undefined)) {
        throw new CallError("BAD_REQUEST", {
            message: isStack
                ? `${target.name} is a stack, submitted with its evaluation`
                : `${target.name} is an application, submitted without a stack's evaluation`,
        });
    }

    // pin the submitted build and require a build of the installation's package
    const pinned = await pin(context, target, submission.build, options);
    const reader = await options.builds.packages.contents(pinned.manifest);
    const built = PackageManifest.package(reader.manifest).id;
    if (built !== target.packageId) {
        throw new CallError("BAD_REQUEST", {
            message: `${target.name} installs ${target.packageId}, submitted a build of ${built}`,
        });
    }

    // retain the evaluation once
    const [revision] = await context.mutate(context.self, [
        await InstallationRevision.create(target, pinned, evaluation),
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
        return target;
    }

    // record its submitter, who consents to what it requests
    const submittedBy = ObjectReference.columns("submittedBy", context.caller.subject);

    return context.update({ ...followed, ...submittedBy });
}

/** Pin a submitted build as the space's: a succeeded build, or a release pulled once. */
async function pin(
    context: MethodContext,
    target: Installation,
    reference: Submission["build"],
    options: Pick<InstallationOptions, "builds" | "registry">,
): Promise<{ readonly id: Identifier<"build">; readonly manifest: string }> {
    // take a succeeded build of the space
    if (reference.kind === "build") {
        return Build.require(context, target.scope, reference.build);
    }

    // read the version a tag or a pinned manifest has
    const { builds, registry } = options;
    const manifest =
        reference.kind === "tag"
            ? await registry.find(target.packageId, reference.tag)
            : (reference.manifest ?? (await registry.find(target.packageId, reference.version)));
    const reader = await builds.pull(registry, target.packageId, manifest);

    // refuse a manifest of another version, and keep the release as the space's build
    const version = reader.manifest.package.version;
    if (reference.kind === "release" && reference.version !== version) {
        throw new CallError("BAD_REQUEST", {
            message: `${target.packageId} ${reference.version} resolved to ${version}`,
        });
    }
    const release = { package: target.packageId, version, manifest };

    return Build.release(
        context,
        (calls) => context.mutate(context.self, calls),
        target.scope,
        release,
    );
}

/** Key a stack installation under its space and an application installation by itself. */
function installationKey(row: {
    readonly id: Identifier<"installation">;
    readonly scope: string;
    readonly role: Installation["role"];
}) {
    return row.role === "stack" ? { scope: row.scope, role: "stack" } : { id: row.id };
}

/** Wake the installations a moved reference or a finished commit build selects. */
function repositoryWatches(forge: Forge) {
    return [
        // wake the installations of every served space selecting a moved reference
        ObjectWatch.of(forge.references, async (row, database) => {
            const { repository, name } = forge.reference(row);
            // key the installations of every space the database serves selecting the reference
            const selecting = await Installation.selecting(
                database,
                undefined,
                (selection) => selection.repository === repository && selection.reference === name,
            );

            return selecting.map(installationKey);
        }),
        // wake the installations of the space selecting the directory a finished commit build compiled
        ObjectWatch.of(build.table, async (row, database) => {
            // leave unfinished builds and builds of other sources
            const { source } = row;
            if (source.kind !== "commit" || !Build.isFinished(row)) {
                return [];
            }

            // key the space's installations selecting its directory
            const selecting = await Installation.selecting(
                database,
                row.scope,
                (selection) =>
                    selection.repository === source.repository &&
                    selection.directory === source.directory,
            );

            return selecting.map(installationKey);
        }),
    ];
}

/** Wake installations on the changes of their resources, deployments, bindings and placement. */
function installationWatches(resources: readonly ResourceType<typeof space>[]) {
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
                    .select({ conditions: installation.table.conditions })
                    .from(installation.table)
                    .where(
                        and(
                            eq(installation.table.scope, row.scope),
                            eq(installation.table.role, "stack"),
                        ),
                    );
                const isWaiting = stack?.conditions["ready"]?.reason === "WaitingForResources";

                // find the applications binding it
                const bound = await database
                    .select({ id: binding.table.installationId })
                    .from(binding.table)
                    .where(
                        and(eq(binding.table.scope, row.scope), eq(binding.table.target, row.id)),
                    );

                return [
                    ...(isWaiting ? [{ scope: row.scope, role: "stack" }] : []),
                    ...bound.map((entry) => ({ id: entry.id })),
                ];
            }),
        ),
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
                .filter((each) => each.conditions["ready"]?.reason === "UnentitledCapability")
                .map((each) => ({ id: each.id }));
        }),
        // wake the space's applications waiting for a dependency once its package is installed or served
        ObjectWatch.of(installation.table, async (row, database) => {
            const refused = await database
                .select({
                    id: installation.table.id,
                    conditions: installation.table.conditions,
                    dependencies: installationRevision.table.dependencies,
                })
                .from(installation.table)
                .innerJoin(
                    installationRevision.table,
                    eq(installationRevision.table.id, installation.table.revisionId),
                )
                .where(eq(installation.table.scope, row.scope));

            return refused
                .filter(
                    (each) =>
                        ["UninstalledDependency", "WaitingForSpace"].includes(
                            each.conditions["ready"]?.reason ?? "",
                        ) &&
                        each.dependencies.some((release) => release.package.id === row.packageId),
                )
                .map((each) => ({ id: each.id }));
        }),
        // wake the space's stack and applications once its placement changes
        Space.watch(async (spaceId, database) => {
            const rows = await database
                .select({
                    id: installation.table.id,
                    role: installation.table.role,
                })
                .from(installation.table)
                .where(eq(installation.table.scope, spaceId));

            return rows.map((row) =>
                row.role === "stack" ? { scope: spaceId, role: "stack" } : { id: row.id },
            );
        }),
    ];
}

/** Read the release a declared installation follows, pinned by the stack's evaluation. */
function declaredRelease(desired: {
    readonly package: {
        readonly id: PackageId;
        readonly version: string;
        readonly manifest?: Digest;
    };
}): { readonly package: PackageId; readonly version: string; readonly manifest: string } {
    const { id, version, manifest } = desired.package;
    if (manifest === undefined) {
        throw new StackError("INVALID_DEFINITION", `the stack installs ${id} ${version} unpinned`);
    }

    return { package: id, version, manifest };
}

/** Open a view the installation's applied revision declares at a location, in the opener's home. */
async function open(
    context: MethodContextOf<typeof declaredInstallation, "open">,
    options: Pick<InstallationOptions, "accounts" | "builds">,
): Promise<ResultOf<typeof declaredInstallation, "open">> {
    // require the revision the installation applies, whose build serves its views
    const { target } = context;
    const { view, path } = context.input;
    const revision =
        target.appliedRevisionId === null
            ? undefined
            : await context
                  .read(installationRevision)
                  .findFirst({ where: { id: target.appliedRevisionId } });
    if (revision === undefined) {
        throw new CallError("NOT_FOUND", { message: `${target.name} declares no view ${view}` });
    }

    // read the revision's build and the view it mounts
    const built = await Build.require(context, target.scope, revision.buildId);
    const reader = await options.builds.packages.contents(built.manifest);
    const mounted = ViewDescription.list(reader.manifest)[view];
    if (mounted === undefined) {
        throw new CallError("NOT_FOUND", { message: `${target.name} declares no view ${view}` });
    }

    // read the opening person's home
    const opener = present(context.caller.subject, `the person opening ${target.name}`);
    if (!principal.user.is(opener)) {
        throw new CallError("BAD_REQUEST", { message: `${Subject.key(opener)} has no home` });
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
        permissions: Installation.permissions(target, declared, "user"),
    };
}
