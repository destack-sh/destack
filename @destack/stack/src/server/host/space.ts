import { Placement, placement, Space, space, SpaceAllowance } from "@destack/account/object";
import { eq, Snapshot } from "@destack/db";
import { IdentityKeystore } from "@destack/account/identity";
import {
    type ObjectController,
    type ObjectReconciliation,
    ObjectWatch,
    type Observation,
} from "@destack/object";
import { nth, type JsonObject } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { AccessContext, Caller, principal } from "@destack/access";
import { Scope, type Uplink } from "@destack/sync";
import { installation } from "../../object/index.ts";
import type { Scheduler } from "./scheduler.ts";

/** The condition a space records its placement under, beside its allowances' conditions. */
const PLACEMENT_CONDITION = "placement";

/** Control the replicas of the spaces this machine serves: place, limit and delete each. */
export function implementSpace(options: {
    /** The keystore whose keys of a deleted space the machine forgets. */
    readonly keystore: IdentityKeystore;
    /** The scheduler placing each space where its tenancy asks. */
    readonly scheduler: Scheduler;
    /** The universe's uplink, receiving the space's status as the space. */
    readonly publisher: Uplink;
    /** Whether the machine checks its spaces' use against their accounts' allowances. */
    readonly isEnforcingAllowances: boolean;
}) {
    return space.implement({ methods: {}, controller: control(options) });
}

/** Control each served space. */
function control(options: Parameters<typeof implementSpace>[0]): ObjectController<typeof space> {
    const { scheduler, isEnforcingAllowances } = options;

    return {
        pending: {},
        watches: [
            ...scheduler.watches,
            ...(isEnforcingAllowances ? SpaceAllowance.watches() : []),
            // look again at a deleting space as each of its deleting installations changes
            ObjectWatch.of(installation.table, (row) =>
                row.deletedAt === null ? [] : [{ id: row.scope }],
            ),
        ],
        reconcile: async ({ rows, database, store, now }) => {
            // leave a space another machine serves
            const row = nth(rows, 0);
            if (!(await Space.isServing(database, row.id))) {
                return undefined;
            }

            // delete a space whose deletion is requested
            if (row.deletedAt !== null) {
                await remove(row, { database, store, now }, options);

                return undefined;
            }

            // decide the allowances' conditions and schedule the space
            const allowed = isEnforcingAllowances
                ? await SpaceAllowance.enforce(row, database, now)
                : {};
            const placed = await scheduler.schedule(row, store, now);
            const conditions = { ...allowed, [PLACEMENT_CONDITION]: placed };

            // report the conditions the space lacks to the universe, as the space itself
            if (!isObserved(row, conditions)) {
                await report(
                    options.publisher,
                    row,
                    "observe",
                    { observedGeneration: row.generation, conditions },
                    now,
                );
            }

            return undefined;
        },
    };
}

/** Delete a space. */
async function remove(
    row: Space,
    reconciliation: Pick<ObjectReconciliation<typeof space>, "database" | "store" | "now">,
    options: Pick<Parameters<typeof implementSpace>[0], "keystore" | "publisher">,
): Promise<void> {
    // request the deletion of the installations not deleting yet as the space itself, waiting while any remains
    const { database, store, now } = reconciliation;
    const itself = principal.space.reference(Scope.universe.id, row.id);
    const installations = await database
        .select({
            id: installation.table.id,
            scope: installation.table.scope,
            deletedAt: installation.table.deletedAt,
        })
        .from(installation.table)
        .where(eq(installation.table.scope, row.id));
    const live = installations.filter((entry) => entry.deletedAt === null);
    if (live.length > 0) {
        await store.mutate(
            AccessContext.of(Caller.of(itself), now),
            {
                id: RequestId.create(now),
                calls: live.map((entry) => installation.call("delete", installation.target(entry))),
            },
            { component: space.name },
        );
    }
    if (installations.length > 0) {
        return;
    }

    // withdraw the space's placement as its machine and forget its keys, each again after a failed earlier look
    const [located] = await Placement.find(Snapshot.live(database), [row.id]);
    if (located !== undefined) {
        const withdrawal = { accountId: located.scope, id: located.id, epoch: located.epoch };
        await options.publisher.receive({
            id: RequestId.create(now),
            calls: [placement.call("withdraw", withdrawal)],
        });
    }
    await options.keystore.forget(database, row.id);

    // remove the row at the universe as the space itself
    await report(options.publisher, row, "finalize", {}, now);
}

/** Run a method of a space's row at its home, the universe, as the space itself. */
async function report(
    publisher: Uplink,
    row: Pick<Space, "id" | "scope">,
    name: "observe" | "finalize",
    input: JsonObject,
    now: number,
): Promise<void> {
    const call = space.call(name, { accountId: row.scope, id: row.id, ...input });
    await publisher.receive(
        { id: RequestId.create(now), calls: [call] },
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
