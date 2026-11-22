import { installation, installationRevision, Installation } from "@destack/stack/object";
import { Entitlement, Space, space } from "@destack/account/object";
import { eq } from "@destack/db";
import type { ObjectController, ObjectReconciliation, Observation } from "@destack/object";
import { present, type UuidEnvironment, type JsonObject } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { ObjectRemote } from "@destack/object/client";
import { AccessContext, Caller, principal, Authorization } from "@destack/access";
import { Failure } from "@destack/error";
import { Scheduler } from "../machine/scheduler.ts";

/** The condition a space records its placement under, next to its entitlements' conditions. */
const PLACEMENT_CONDITION = "placement";

/** A space's role on a machine: placed here, moving here, or leaving. */
export type SpaceRole = "primary" | "standby" | "leaving";

/** Control the replicas of the spaces this machine serves: place, limit and delete each. */
export function implementSpace(options: {
    /** The remote toward the universe, receiving the space's status as the space. */
    readonly remote: ObjectRemote;
    /** Whether the machine checks its spaces' use against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
}) {
    return space.implement({ methods: {}, controllers: [limit(options)] });
}

/** Limit each served space: its deletion, suspension, entitlements and schedule. */
function limit(options: Parameters<typeof implementSpace>[0]): ObjectController<typeof space> {
    const { isEnforcingEntitlements } = options;

    return {
        name: "limit",
        reads: [
            ...Space.serving,
            ...Scheduler.watches,
            ...(isEnforcingEntitlements ? Entitlement.watches() : []),
            // look again at a deleting space as each of its uninstalling installations changes
            {
                table: installation.table,
                where: { deletedAt: { isNotNull: true } },
                fields: ["deletedAt"],
                on: { id: "scope" },
            },
        ],
        reconcile: async ({ rows, database, store, now }) => {
            // leave a space gone or another machine serves
            const [row] = rows;
            if (row === undefined || !(await Space.isServing(database.snapshot(), row.id))) {
                return;
            }

            // delete a space whose deletion is requested
            if (row.deletedAt !== null) {
                await remove(row, { database, store, now }, options);

                return;
            }

            // suspend or resume the space's scope here as its row asks
            await mirrorSuspension(row, { database, store, now });

            // decide the entitlements' conditions and schedule the space
            const allowed = isEnforcingEntitlements
                ? await Entitlement.enforce(row, database, now)
                : {};
            const placed = await Scheduler.schedule(row, store);
            const conditions = { ...allowed, [PLACEMENT_CONDITION]: placed };

            // report the conditions the space lacks to the universe, as the space itself
            if (!isObserved(row, conditions)) {
                await report(
                    options.remote,
                    row,
                    "observe",
                    { observedGeneration: row.generation, conditions },
                    store.environment,
                );
            }
        },
    };
}

/** Suspend or resume the scope of a space at its home here, as the space's row asks. */
async function mirrorSuspension(
    row: Space,
    reconciliation: Pick<ObjectReconciliation<typeof space>, "database" | "store" | "now">,
): Promise<void> {
    // leave a scope already as the row asks
    const { database, store, now } = reconciliation;
    const [scope] = await Scope.read(database.snapshot(), [row.id]);
    const isSuspended = present(scope, `the scope of ${row.id}`).suspendedAt !== null;
    if (isSuspended === (row.suspendedAt !== null)) {
        return;
    }

    // mark the scope as the store's principal
    await database.commit(async (transaction) => {
        const authorization = new Authorization(
            store.authorizer,
            transaction,
            () => AccessContext.of(Caller.of(store.principal), now),
            store.environment,
        );
        const reference = space.reference(row.scope, row.id);
        await (row.suspendedAt === null
            ? authorization.resume(reference)
            : authorization.suspend(reference));
    });
}

/** Delete a space. */
async function remove(
    row: Space,
    reconciliation: Pick<ObjectReconciliation<typeof space>, "database" | "store" | "now">,
    options: Pick<Parameters<typeof implementSpace>[0], "remote">,
): Promise<void> {
    // request the uninstall of the unmanaged installations not deleting yet as the space, the stack deleting what it manages
    const { database, store, now } = reconciliation;
    const itself = principal.space.reference(Scope.universe.id, row.id);
    const installed = await database
        .select({
            id: installation.table.id,
            scope: installation.table.scope,
            packageId: installation.table.packageId,
            deletedAt: installation.table.deletedAt,
            managerInstallationId: installation.table.managerInstallationId,
            dependencies: installationRevision.table.dependencies,
        })
        .from(installation.table)
        .leftJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.targetRevisionId),
        )
        .where(eq(installation.table.scope, row.id));
    const live = installed.filter(
        (entry) => entry.deletedAt === null && entry.managerInstallationId === null,
    );
    if (live.length > 0) {
        await store.mutate(
            AccessContext.of(Caller.of(itself), now),
            {
                id: RequestId.create(store.environment),
                calls: Installation.dependentsFirst(live).map((entry) =>
                    installation.call("delete", installation.target(entry)),
                ),
            },
            { component: space.name },
        );
    }
    if (installed.length > 0) {
        return;
    }

    // remove its row at the universe as the space
    await report(options.remote, row, "finalize", {}, store.environment).catch(refuseUnlessRemoved);
}

/** Rethrow a call's failure unless the universe already removed the row the call targets. */
function refuseUnlessRemoved(error: unknown): void {
    const { domain, reason } = Failure.of(error);
    if (domain !== "http" || reason !== "NOT_FOUND") {
        throw error;
    }
}

/** Run a method of a space's row at its home, the universe, as the space itself. */
async function report(
    remote: ObjectRemote,
    row: Pick<Space, "id" | "scope">,
    name: "observe" | "finalize",
    input: JsonObject,
    environment: UuidEnvironment,
): Promise<void> {
    const call = space.call(name, { accountId: row.scope, id: row.id, ...input });
    await ObjectRemote.mutate(
        remote,
        row.scope,
        { id: RequestId.create(environment), calls: [call] },
        principal.space.reference(Scope.universe.id, row.id),
    );
}

/** Report whether a space records some conditions already. */
function isObserved(
    row: Pick<Space, "conditions">,
    conditions: Observation["conditions"],
): boolean {
    return Object.entries(conditions).every(([name, wanted]) => {
        const current = row.conditions[name];

        return (
            current?.status === wanted.status &&
            current.reason === wanted.reason &&
            current.message === wanted.message
        );
    });
}
