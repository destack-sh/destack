import { Account, Space, space } from "@destack/account/object";
import { type Identifier, type JsonObject, schema } from "@destack/schema";
import { and, type DatabaseConnection, eq, Snapshot, Transaction } from "@destack/db";
import {
    ObjectDescription,
    ObjectError,
    type ObjectReconciliation,
    type ObjectType,
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
    type NamedInstallation,
} from "../../object/index.ts";
import { Approval } from "./approval.ts";
import { implementRelationship, implementRole } from "../access/role.ts";
import { type BuildStore, type Registry } from "./build.ts";
import { Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** What a machine applies stacks with: its build store and the registry it pulls releases from. */
export interface StackOptions {
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
    /** The registry of published releases. */
    readonly registry: Registry;
}

/** Apply the revision a served space's stack follows and observe it applied or waiting. */
export async function reconcileStack(
    spaceId: Identifier<"space">,
    reconciliation: ObjectReconciliation,
    options: StackOptions,
): Promise<undefined> {
    // read the stack of a served space and the revision it follows
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
        const planned =
            installationId === undefined
                ? undefined
                : await snapshot.read(rollout.table).findFirst({ where: { id: installationId } });
        const pinned = planned?.revisionId === null ? Rollout.pinned(planned) : undefined;

        return pinned === undefined
            ? Rollout.reader(snapshot, spaceId, packageId, installationId, options.builds)
            : options.builds.pull(options.registry, packageId, pinned);
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
    if (stack.deletedAt !== null) {
        const retiring = stackApplication(database, stack, {}, applied, objects, {
            store,
            release,
            isHome,
        });
        await retiring.apply(store.clock.now());
        const managed = await database
            .select({ id: rollout.table.id })
            .from(rollout.table)
            .where(eq(rollout.table.managerInstallationId, stack.id));
        if (managed.length === 0) {
            await reconciliation.mutate([rollout.call("finalize", rollout.target(stack))]);
        }

        return undefined;
    }
    // leave a stack without a revision, or with its revision applied
    else if (revision === null || stack.appliedRevisionId === revision.id) {
        return undefined;
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
            await reconciliation.observe(rollout, stack, outcome.waiting);

            return undefined;
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
            rollout,
            stack,
            ready(isBlocked ? "Blocked" : "ApplyFailed", message, {}),
        );
        if (!isBlocked) {
            throw error;
        }

        return undefined;
    }

    // observe it applied and forget its plan, or observe it waiting with the approved plan the rest resumes
    if (deferred === undefined) {
        const fields = { plan: null, approvedPlan: null };
        const observed = { ...fields, ...Rollout.applied(stack, revision.id) };
        await reconciliation.observe(
            rollout,
            stack,
            ready("Applied", "every declaration applied", observed),
        );
    } else {
        await reconciliation.observe(rollout, stack, ready("WaitingForResources", deferred, {}));
    }

    return undefined;
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
            and(eq(rollout.table.scope, spaceId), eq(rollout.table.kind, "stack"), Space.served()),
        );

    return served === undefined
        ? undefined
        : { stack: Rollout.join(served.stack, served.installation), revision: served.revision };
}

/** Read the definition of the revision a stack applied last, empty before its first apply. */
async function appliedDefinition(
    database: DatabaseConnection,
    stack: Rollout & { readonly installation: NamedInstallation },
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
    stack: Rollout & { readonly installation: NamedInstallation },
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
    stack: Rollout & { readonly installation: NamedInstallation },
    document: JsonObject,
    objects: readonly ObjectType[],
    release: StackApplicationOptions["release"],
): Promise<{ readonly types: readonly ObjectType[]; readonly isPending: boolean }> {
    // read the installations the stack manages that follow a revision or a pinned release
    const managed = await Rollout.read(
        snapshot,
        await snapshot.read(rollout.table).findMany({
            where: { scope: stack.scope, managerInstallationId: stack.id },
        }),
    );
    const following = managed.filter(
        (entry) => entry.revisionId !== null || Rollout.pinned(entry) !== undefined,
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
        ObjectStore.known(objects, Object.values(accountPackage.objects)),
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
