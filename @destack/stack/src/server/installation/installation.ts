import { ObjectReference, Subject } from "@destack/sync";
import { aligned, canonicalize, Digest, type Identifier, present, schema } from "@destack/schema";
import { principal } from "@destack/access";
import { and, eq } from "@destack/db";
import {
    type Call,
    type CallOf,
    ObjectWatch,
    type ResourceType,
    type ResultOf,
} from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { Space, space } from "@destack/account/object";
import {
    binding,
    deployment,
    type Installation,
    installation,
    installationRevision,
    InstallationRevision,
    Build,
    type Submission,
} from "../../object/index.ts";
import {
    Application,
    type ApplicationOptions,
    finalizeApplication,
    reconcileApplication,
} from "./application.ts";
import { Approval } from "./approval.ts";
import type { InstallationBuild, InstallationSelection } from "../../declare/installation.ts";
import { type RepositoryOwner, SelectedRepository } from "./selection.ts";
import type { SpaceAccounts } from "../server.ts";
import { reconcileStack, type StackOptions } from "./stack.ts";
import { StackError } from "../../error/index.ts";
import type { TokenIssuer } from "@destack/runtime/authentication";
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

            return followed?.digest !== (await InstallationRevision.digest(declaredBuild(desired)));
        },
        written: async (context, row, desired) => {
            // retain the release's revision once and follow it
            const [{ id }] = await context.mutate([
                await InstallationRevision.create(row, declaredBuild(desired)),
            ]);
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
    /** Read the account owning a selection's repository. */
    readonly repositories: RepositoryOwner | null;
    /** The account package, reading the home a person opens views in. */
    readonly accounts: Pick<SpaceAccounts, "user">;
    /** The space's token issuer, signing the tokens installations are granted. */
    readonly issuer: TokenIssuer;
}

/** A create or update call selecting a release, repository or checkout. */
interface SelectingCall extends Pick<Call, "read"> {
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
            watches: installationWatches(options.resources),
            reconcile: (reconciliation) => {
                const target = aligned(reconciliation.rows, 0);

                return target.role === "stack"
                    ? reconcileStack(target.scope, reconciliation, options)
                    : reconcileApplication(target, reconciliation);
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
                prepare: (call) =>
                    SelectedRepository.owner(
                        options.repositories,
                        canonicalize(call.input.selection) === canonicalize(call.target.selection)
                            ? undefined
                            : call.input.selection,
                    ),
                handle: (call) => submit(call, call.prepared, options),
            },
            deploy: async (call) => ({ requested: await new Application(call, options).deploy() }),
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
function handleSelection(options: Pick<InstallationOptions, "repositories">) {
    return {
        prepare: (call: {
            readonly input: { readonly selection?: InstallationSelection | undefined };
        }) => SelectedRepository.owner(options.repositories, call.input.selection),
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
    call: CallOf<typeof installation, "submit">,
    owner: Identifier<"account"> | null,
    options: InstallationOptions,
): Promise<Installation> {
    // refuse another account's repository
    const { target, input: submission } = call;
    await SelectedRepository.require(call, call.scope, submission.selection, owner);

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
    const build = await pin(call, target, submission.build, options);
    const reader = await options.builds.packages.contents(build.manifest);
    const built = reader.manifest.package.id;
    if (built !== target.packageId) {
        throw new CallError("BAD_REQUEST", {
            message: `${target.name} installs ${target.packageId}, submitted a build of ${built}`,
        });
    }

    // retain the evaluation once
    const [revision] = await call.mutate(call.self, [
        await InstallationRevision.create(target, build, evaluation),
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
    const submittedBy = ObjectReference.columns("submittedBy", call.caller.subject);

    return call.update({ ...followed, ...submittedBy });
}

/** Pin a submitted build by its manifest's digest. */
async function pin(
    call: Call,
    target: Installation,
    reference: Submission["build"],
    options: Pick<InstallationOptions, "builds" | "registry">,
): Promise<InstallationBuild> {
    // take a succeeded build of the space
    if (reference.kind === "build") {
        return Build.resolve(call, target.scope, reference.build);
    }

    // keep the release a version or tag points at in the build store
    const { builds, registry } = options;
    const manifest =
        reference.kind === "tag"
            ? await registry.find(target.packageId, reference.tag)
            : (reference.manifest ?? (await registry.find(target.packageId, reference.version)));
    const { location, fetch } = await registry.locate(target.packageId, manifest);
    await builds.packages.copy(location, fetch);
    const reader = await builds.packages.contents(manifest);

    // record the version the release has, refusing a manifest of another version
    const version = reader.manifest.package.version;
    if (reference.kind === "release" && reference.version !== version) {
        throw new CallError("BAD_REQUEST", {
            message: `${target.packageId} ${reference.version} resolved to ${version}`,
        });
    }

    return { kind: "release", version, manifest };
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

/** Read the release build a declared installation follows, pinned by the stack's evaluation. */
function declaredBuild(desired: {
    readonly package: { readonly id: string; readonly version: string; readonly manifest?: Digest };
}): InstallationBuild {
    const { id, version, manifest } = desired.package;
    if (manifest === undefined) {
        throw new StackError("INVALID_DEFINITION", `the stack installs ${id} ${version} unpinned`);
    }

    return { kind: "release", version, manifest };
}

/** Open a view the installation's revision declares at a location, in the opening person's home. */
async function open(
    call: CallOf<typeof declaredInstallation, "open">,
    options: Pick<InstallationOptions, "accounts">,
): Promise<ResultOf<typeof declaredInstallation, "open">> {
    // require a revision declaring the view
    const { target } = call;
    const { view, path } = call.input;
    const revision =
        target.revisionId === null
            ? undefined
            : await call.read(installationRevision).findFirst({ where: { id: target.revisionId } });
    const definition = revision?.views[view];
    if (revision === undefined || definition === undefined) {
        throw new CallError("NOT_FOUND", {
            message: `${target.name} declares no view ${view}`,
        });
    }

    // read the opening person's home
    const opener = present(call.caller.subject, `the person opening ${target.name}`);
    if (!principal.user.is(opener)) {
        throw new CallError("BAD_REQUEST", { message: `${Subject.key(opener)} has no home` });
    }
    const home = await options.accounts.user.home({
        id: schema.identifier("user").parse(opener.id),
    });

    return {
        installationId: target.id,
        revisionId: revision.id,
        build: revision.build,
        view,
        definition,
        path,
        home,
    };
}
