import { Entitlement, Placement, placement, Space, space } from "@destack/account/object";
import { and, eq, Snapshot, Transaction } from "@destack/db";
import { IdentityKeystore } from "@destack/account/identity";
import {
    type ObjectController,
    type ObjectReconciliation,
    ObjectWatch,
    type Observation,
} from "@destack/object";
import { nth, present, type UuidEnvironment, type JsonObject } from "@destack/schema";
import { RequestId, Scope, type Uplink } from "@destack/sync";
import { AccessContext, Authorization, Caller, principal } from "@destack/access";
import { installation } from "@destack/object";
import { Failure } from "@destack/error";
import { installationRevision, Rollout, rollout } from "../../object/index.ts";
import { Scheduler } from "./scheduler.ts";
import type { BuildStore } from "../installation/build.ts";

/** The condition a space records its placement under, next to its entitlements' conditions. */
const PLACEMENT_CONDITION = "placement";

/** A space's role on a machine: placed here, or standing by for a move while placed elsewhere. */
export type SpaceRole = "primary" | "standby";

/** Control the replicas of the spaces this machine serves: place, limit and delete each. */
export function implementSpace(options: {
    /** The keystore whose keys of a deleted space the machine forgets. */
    readonly keystore: IdentityKeystore;
    /** The build store whose root of a deleted space the machine forgets. */
    readonly builds: BuildStore;
    /** The universe's uplink, receiving the space's status as the space. */
    readonly publisher: Uplink;
    /** Whether the machine checks its spaces' use against their accounts' entitlements. */
    readonly isEnforcingEntitlements: boolean;
}) {
    return space.implement({ methods: {}, controller: control(options) });
}

/** Control each served space. */
function control(options: Parameters<typeof implementSpace>[0]): ObjectController<typeof space> {
    const { isEnforcingEntitlements } = options;

    return {
        pending: {},
        watches: [
            ...Scheduler.watches,
            ...(isEnforcingEntitlements ? Entitlement.watches() : []),
            // look again at a deleting space as each of its deleting rollouts changes
            ObjectWatch.of(rollout.table, (row) =>
                row.deletedAt === null ? [] : [{ id: row.scope }],
            ),
        ],
        reconcile: async ({ rows, database, store, now }) => {
            // leave a space another machine serves
            const row = nth(rows, 0);
            if (!(await Space.isServing(Snapshot.live(database), row.id))) {
                return undefined;
            }

            // delete a space whose deletion is requested
            if (row.deletedAt !== null) {
                await remove(row, { database, store, now }, options);

                return undefined;
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
                    options.publisher,
                    row,
                    "observe",
                    { observedGeneration: row.generation, conditions },
                    store.environment,
                );
            }

            return undefined;
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
    const [scope] = await Scope.read(Snapshot.live(database), [row.id]);
    const isSuspended = present(scope, `the scope of ${row.id}`).suspendedAt !== null;
    if (isSuspended === (row.suspendedAt !== null)) {
        return;
    }

    // mark the scope as the store's principal
    await Transaction.run(database, async (transaction) => {
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
    options: Pick<Parameters<typeof implementSpace>[0], "keystore" | "builds" | "publisher">,
): Promise<void> {
    // request the uninstall of the unmanaged rollouts not deleting yet as the space itself, the stack deleting what it manages
    const { database, store, now } = reconciliation;
    const itself = principal.space.reference(Scope.universe.id, row.id);
    const rollouts = await database
        .select({
            id: rollout.table.id,
            scope: rollout.table.scope,
            packageId: installation.table.packageId,
            deletedAt: rollout.table.deletedAt,
            managerInstallationId: rollout.table.managerInstallationId,
            dependencies: installationRevision.table.dependencies,
        })
        .from(rollout.table)
        .innerJoin(
            installation.table,
            and(
                eq(installation.table.id, rollout.table.id),
                eq(installation.table.scope, rollout.table.scope),
            ),
        )
        .leftJoin(
            installationRevision.table,
            eq(installationRevision.table.id, rollout.table.revisionId),
        )
        .where(eq(rollout.table.scope, row.id));
    const live = rollouts.filter(
        (entry) => entry.deletedAt === null && entry.managerInstallationId === null,
    );
    if (live.length > 0) {
        await store.mutate(
            AccessContext.of(Caller.of(itself), now),
            {
                id: RequestId.create(store.environment),
                calls: Rollout.dependentsFirst(live).map((entry) =>
                    rollout.call("delete", rollout.target(entry)),
                ),
            },
            { component: space.name },
        );
    }
    if (rollouts.length > 0) {
        return;
    }

    // withdraw the space's placement as its machine, which an earlier look may have withdrawn already
    const [located] = await Placement.find(Snapshot.live(database), [row.id]);
    if (located !== undefined) {
        const withdrawal = { accountId: located.scope, id: located.id, epoch: located.epoch };
        await options.publisher
            .receive({
                id: RequestId.create(store.environment),
                calls: [placement.call("withdraw", withdrawal)],
            })
            .catch(refuseUnlessRemoved);
    }

    // forget its keys and build root, then remove its row at the universe as the space itself, as an earlier look may have
    await options.keystore.forget(database, row.id);
    await options.builds.packages.forget(row.id);
    await report(options.publisher, row, "finalize", {}, store.environment).catch(
        refuseUnlessRemoved,
    );
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
    publisher: Uplink,
    row: Pick<Space, "id" | "scope">,
    name: "observe" | "finalize",
    input: JsonObject,
    environment: UuidEnvironment,
): Promise<void> {
    const call = space.call(name, { accountId: row.scope, id: row.id, ...input });
    await publisher.receive(
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
