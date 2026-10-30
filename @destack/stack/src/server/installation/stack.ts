import { Account, Space, space } from "@destack/account/object";
import { Identifier, type JsonObject, schema } from "@destack/schema";
import {
    and,
    type BlobStore,
    type DatabaseConnection,
    eq,
    isNotNull,
    Snapshot,
    Transaction,
} from "@destack/db";
import {
    ObjectDescription,
    ObjectError,
    type ObjectReconciliation,
    ObjectType,
    ObjectTypePool,
} from "@destack/object";
import accountPackage from "@destack/account/package";
import { Stack, ObjectStore } from "@destack/object/server";
import { StackError } from "../../error/index.ts";
import { installation } from "@destack/object";
import {
    installationRevision,
    Rollout,
    rollout,
    type StackReason,
    type InstallationRollout,
} from "../../object/index.ts";
import { Approval } from "./approval.ts";
import { implementRelationship, implementRole } from "../access/role.ts";
import { type BuildStore, type Registry } from "./build.ts";
import { Failure } from "@destack/error";
import type { Forge } from "./follow.ts";

/** What a machine applies stacks with: builds, the registry, blobs and repositories. */
export interface StackOptions {
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
    /** The space's blobs, storing the states its revisions require. */
    readonly blobs: BlobStore;
    /** The forge whose references and tags installations select, null without one. */
    readonly forge: Forge | null;
}

/** The space's stack installation: it applies its document and targets what it manages. */
export class StackInstallation {
    /** The stack's rollout, with its installation's fields. */
    readonly #target: InstallationRollout;
    /** What the machine applies stacks and describes revisions with. */
    readonly #options: StackOptions;

    /** Reconcile a space's stack installation with what the machine applies it with. */
    constructor(target: InstallationRollout, options: StackOptions) {
        this.#target = target;
        this.#options = options;
    }

    /** Apply the stack's targeted revision, then target the revisions of what it manages. */
    async reconcile(reconciliation: ObjectReconciliation): Promise<undefined> {
        // apply the stack's document, then target the revisions of the installations it manages while it lives
        const stack = await this.#apply(reconciliation);
        if (stack !== undefined && stack.installation.deletedAt === null) {
            await this.#targetRevisions(reconciliation, stack);
        }

        return undefined;
    }

    /** Apply the revision the stack targets, observing it applied or waiting. */
    async #apply(reconciliation: ObjectReconciliation): Promise<InstallationRollout | undefined> {
        // read the stack of a served space and the revision it follows
        const spaceId = space.identifier(this.#target.scope);
        const { database, store } = reconciliation;
        const served = await readServedStack(spaceId, database);
        if (served === undefined) {
            return undefined;
        }
        const { stack, revision } = served;
        const release: StackApplicationOptions["release"] = async (
            snapshot,
            packageId,
            installationId,
        ) => {
            // open the selected release of a planned installation before its first revision, else its revision's build
            const rows =
                installationId === undefined
                    ? []
                    : await snapshot
                          .read(rollout.table)
                          .findMany({ where: { id: installationId } });
            const [planned] = await Rollout.read(snapshot, rows);
            const pinned =
                planned?.revisionId === null ? Rollout.pinned(planned.installation) : undefined;

            return pinned === undefined
                ? Rollout.reader(snapshot, spaceId, packageId, installationId, this.#options.builds)
                : this.#options.builds.pull(this.#options.registry, packageId, pinned);
        };
        const objects = declaredTypes(store.objects);
        const applied = await appliedDefinition(database, stack);

        // read whether the stack's space is its account's home, whose stack declares into the account
        const [placed] = await database
            .select({ account: space.table.scope })
            .from(space.table)
            .where(eq(space.table.id, space.identifier(stack.scope)));
        const isHome = placed !== undefined && Account.home(placed.account) === stack.scope;

        // tear down a deleted stack: retire every declaration, then finalize it once nothing it manages remains
        if (stack.installation.deletedAt !== null) {
            const retiring = stackApplication(database, stack, {}, applied, objects, {
                store,
                release,
                isHome,
            });
            await retiring.apply(store.clock.now());
            const managed = await database
                .select({ id: installation.table.id })
                .from(installation.table)
                .where(eq(installation.table.managerInstallationId, stack.id));
            if (managed.length === 0) {
                await reconciliation.mutate([
                    installation.call("finalize", installation.target(stack.installation)),
                ]);
            }

            return stack;
        }
        // leave a stack without a revision, or with its revision applied
        else if (revision === null || stack.appliedRevisionId === revision.id) {
            return stack;
        }

        // require an evaluated revision of this stack
        if (revision.installationId !== stack.id || !revision.definition) {
            throw StackError.of("DEFINITION_INVALID", `not a revision of stack ${stack.id}`);
        }

        // plan the declarations once and apply the plan unchanged unless it waits for an approval, again while it loses a race
        let deferred: string | undefined;
        try {
            const { definition } = revision;
            const applying = stackApplication(database, stack, definition, applied, objects, {
                store,
                release,
                isHome,
            });
            const outcome = await Transaction.retry(database, "CONCURRENT_UPDATE", async () => {
                // plan, and apply the plan unless it waits for an approval
                const planned = await applying.plan(store.clock.now());
                const waiting = await Approval.waiting(
                    Snapshot.live(database),
                    stack,
                    planned.plan,
                    database.state.environment.crypto,
                );
                if (waiting === undefined) {
                    await planned.apply();
                }

                return { waiting, deferred: planned.plan.deferred };
            });

            // observe a plan waiting for its approval
            if (outcome.waiting !== undefined) {
                await reconciliation.observe(installation, stack.installation, outcome.waiting);

                return stack;
            }
            deferred = outcome.deferred;
        } catch (error) {
            // block on a definition, declaration or declared call the next revision must fix, and retry another failure
            const isBlocked =
                StackError.is(error, "DEFINITION_INVALID") ||
                ObjectError.is(error, "DECLARATION_INVALID") ||
                ObjectError.is(error, "CALL_INVALID");
            const { message } = Failure.of(error);
            await reconciliation.observe(
                installation,
                stack.installation,
                ready(isBlocked ? "Blocked" : "ApplyFailed", message, {}),
            );
            if (!isBlocked) {
                throw error;
            }

            return stack;
        }

        // observe it applied and forget its plan, or observe it waiting with the approved plan the rest resumes
        if (deferred === undefined) {
            await reconciliation.observe(
                installation,
                stack.installation,
                ready("Applied", "every declaration applied", { plan: null, approvedPlan: null }),
            );
            await reconciliation.mutate([
                rollout.call("update", {
                    ...rollout.target(stack),
                    ...Rollout.applied(stack, revision.id),
                }),
            ]);
        } else {
            await reconciliation.observe(
                installation,
                stack.installation,
                ready("WaitingForResources", deferred, {}),
            );
        }

        return stack;
    }

    /** Target each managed installation's selected build in one transaction, past detached ones. */
    async #targetRevisions(reconciliation: ObjectReconciliation, stack: InstallationRollout) {
        // read the live installations the stack manages, past detached ones, with their rollouts
        const snapshot = Snapshot.live(reconciliation.database);
        const managed = await snapshot.read(installation.table).findMany({
            where: {
                scope: stack.scope,
                managerInstallationId: stack.id,
                detachedAt: { isNull: true },
                deletedAt: { isNull: true },
            },
        });
        const rows = await Rollout.installed(snapshot, managed);
        await Rollout.target(reconciliation, rows, this.#options);
    }
}

/** Read the stack of a served space and the revision it follows. */
async function readServedStack(spaceId: Identifier<"space">, database: DatabaseConnection) {
    const [served] = await database
        .select({
            stack: rollout.table,
            installation: installation.table,
            revision: installationRevision.table,
        })
        .from(rollout.table)
        .innerJoin(
            installation.table,
            and(
                eq(installation.table.id, rollout.table.id),
                eq(installation.table.scope, rollout.table.scope),
            ),
        )
        .innerJoin(space.table, eq(space.table.id, rollout.table.scope))
        .leftJoin(
            installationRevision.table,
            eq(installationRevision.table.id, rollout.table.revisionId),
        )
        .where(
            and(
                eq(rollout.table.scope, spaceId),
                isNotNull(installation.table.export),
                Space.served(),
            ),
        );

    return served === undefined
        ? undefined
        : { stack: Rollout.join(served.stack, served.installation), revision: served.revision };
}

/** Read the definition of the revision a stack applied last, empty before its first apply. */
async function appliedDefinition(
    database: DatabaseConnection,
    stack: InstallationRollout,
): Promise<JsonObject> {
    // read the applied revision, which a stack's evaluation always defines
    if (stack.appliedRevisionId === null) {
        return {};
    }
    const [revision] = await database
        .select({ definition: installationRevision.table.definition })
        .from(installationRevision.table)
        .where(eq(installationRevision.table.id, stack.appliedRevisionId));
    const definition = revision?.definition;
    if (definition === undefined || definition === null) {
        throw StackError.of(
            "DEFINITION_INVALID",
            `stack ${stack.id} has no applied evaluated revision`,
        );
    }

    return definition;
}

/** Where a stack revision applies. */
export interface StackApplicationOptions {
    /** The object store running the handlers' calls as its principal. */
    readonly store: Pick<
        ObjectStore,
        | "principal"
        | "mutate"
        | "query"
        | "authorizeHere"
        | "clock"
        | "claims"
        | "isReplica"
        | "environment"
    >;
    /** Open the build of a package's release in the stack's space, through a step's database. */
    readonly release: NonNullable<Stack["options"]["release"]>;
    /** Whether the stack's space is its account's home, which may declare into the account. */
    readonly isHome: boolean;
}

/** Create the stack of a document's declarations in its space. */
function stackApplication(
    database: DatabaseConnection,
    stack: InstallationRollout,
    document: JsonObject,
    applied: JsonObject,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Stack {
    // apply every declaration the machine supports in dependency order, those of installed types once the store serves them
    return new Stack({
        installing: (snapshot) =>
            installedTypes(snapshot, stack, document, objects, options.release),
        database,
        objects,
        manager: { installationId: stack.id, packageId: stack.installation.packageId },
        as: options.store.principal,
        scope: stack.scope,
        document,
        applied,
        policies: [space.policy],
        ...(options.store.claims === undefined ? {} : { claims: options.store.claims }),
        ...options,
    });
}

/** Read the types the builds of a stack's installations declare, and which follow a build. */
async function installedTypes(
    snapshot: Snapshot,
    stack: InstallationRollout,
    document: JsonObject,
    objects: readonly ObjectType[],
    release: StackApplicationOptions["release"],
): Promise<{ readonly types: readonly ObjectType[]; readonly isPending: boolean }> {
    // read the installations the stack manages that follow a revision or a pinned release
    const installed = await snapshot.read(installation.table).findMany({
        where: { scope: stack.scope, managerInstallationId: stack.id },
    });
    const managed = await Rollout.read(
        snapshot,
        await snapshot
            .read(rollout.table)
            .findMany({ where: { id: { in: installed.map((entry) => entry.id) } } }),
    );
    const following = managed.filter(
        (entry) => entry.revisionId !== null || Rollout.pinned(entry.installation) !== undefined,
    );

    // read the object types each one's build declares
    const declared = await Promise.all(
        following.map(async (entry) => {
            const reader = await release(snapshot, entry.installation.packageId, entry.id);

            return ObjectDescription.installed(await reader.declarations());
        }),
    );

    // declare them against the space's types and account's, which they refer to
    const pool = ObjectTypePool.of(
        ObjectType.known(objects, Object.values(accountPackage.objects)),
    );

    // report an installation the document declares that follows no build yet
    const followed = new Set(following.map((entry) => entry.installation.name));
    const names = Object.keys(
        schema.record(schema.string(), schema.unknown()).parse(document["installations"] ?? {}),
    );

    return {
        types: pool.declare(declared.flat()),
        isPending: names.some((name) => !followed.has(name)),
    };
}

/** List the types stacks declare, once per table: the managed ones, roles and relationships. */
function declaredTypes(types: readonly ObjectType[]): readonly ObjectType[] {
    // take the managed types and the stack's roles and relationships, once per table
    const role = implementRole();
    const relationship = implementRelationship();
    const handled = [...types.filter((type) => type.isDeclarable()), role, relationship];

    return [...new Map(handled.map((type) => [type.table, type])).values()];
}

/** Describe a stack's ready observation: its reason, message and observed fields. */
function ready(reason: StackReason, message: string, fields: Readonly<Record<string, unknown>>) {
    const status = reason === "Applied" ? "true" : "false";

    return { conditions: { Ready: { status, reason, message } }, fields } as const;
}
