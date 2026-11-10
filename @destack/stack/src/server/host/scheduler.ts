import { placement, Space } from "@destack/account/object";
import { principal } from "@destack/access";
import { type Select, Snapshot } from "@destack/db";
import { ObjectWatch, type Condition } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { transfer } from "../../object/index.ts";

/** The scheduler of served spaces onto the machines the universe decides for their tenancy. */
export const Scheduler = {
    /** The changes that schedule a space again: its transfers, its placement and its row. */
    watches: [
        ObjectWatch.of(transfer.table, (row) => [{ id: row.scope }]),
        ObjectWatch.of(placement.table, (row) => [{ id: row.spaceId }]),
        Space.watch(async (spaceId) => [{ id: spaceId }]),
    ],

    /** Move a served space where the universe decides, cancelling its own dropped move. */
    async schedule(
        row: Space,
        store: Pick<ObjectStore, "database" | "as" | "mutate" | "environment">,
    ): Promise<Condition> {
        // read the space's running move and the placement the universe decided
        const view = Snapshot.live(store.database);
        const moving = await view
            .read(transfer.table)
            .findFirst({ where: { scope: row.id, completedAt: { isNull: true } } });
        const placed = await view.read(placement.table).findFirst({ where: { spaceId: row.id } });
        const asked = placed?.schedule;

        // keep a person's move, and this scheduler's move while the decision still asks for it
        const isAsked = asked?.kind === "move" && asked.machine === moving?.targetId;
        const isKept = moving !== undefined && (isAsked || !Scheduler.isOwn(moving));

        // wait for the placement to replicate here, which its watch schedules again
        if (asked === undefined) {
            const message = `the placement of ${row.id} has not replicated here yet`;

            return { status: "unknown", reason: "Unplaced", message };
        }
        // let a kept move finish
        else if (moving !== undefined && isKept) {
            return { status: "unknown", reason: "Moving", message: `moving to ${moving.targetId}` };
        }
        // cancel this scheduler's move the decision no longer asks for
        else if (moving !== undefined) {
            await store.mutate(store.as(transfer), {
                id: RequestId.create(store.environment),
                calls: [transfer.call("cancel", transfer.target(moving))],
            });
            const message = `cancelled the move to ${moving.targetId}`;

            return { status: "unknown", reason: "Cancelled", message };
        }
        // stay where the universe keeps the space
        else if (asked.kind === "stay") {
            return asked.condition;
        }
        // move the space to the machine the universe decided
        else {
            await store.mutate(store.as(transfer), {
                id: RequestId.create(store.environment),
                calls: [
                    transfer.call("create", {
                        ...transfer.in(row.id),
                        id: transfer.generateId(store.environment),
                        targetId: asked.machine,
                    }),
                ],
            });

            return { status: "unknown", reason: "Moving", message: `moving to ${asked.machine}` };
        }
    },

    /** Report whether the space itself, whose own code runs this scheduler, requested a move. */
    isOwn(moving: Select<typeof transfer.table>): boolean {
        // take the space, whose own code requests the scheduler's moves
        const own = principal.space.reference(Scope.universe.id, moving.scope);

        return (
            ObjectReference.key(ObjectReference.at("requestedBy", moving)) ===
            ObjectReference.key(own)
        );
    },
};
