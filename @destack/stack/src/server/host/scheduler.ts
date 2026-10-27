import { placement, Space } from "@destack/account/object";
import { principal } from "@destack/access";
import { type Select, Snapshot } from "@destack/db";
import { Installation, ObjectWatch, type Condition } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { transfer } from "../../object/index.ts";
import stackPackage from "../../package.ts";

/** The scheduler of served spaces onto the machines the universe decides for their tenancy. */
export const Scheduler = {
    /** The changes that schedule a space again: its transfers, its placement and its row. */
    watches: [
        ObjectWatch.of(transfer.table, (row) => [{ id: row.scope }]),
        ObjectWatch.of(placement.table, (row) => [{ id: row.id }]),
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
        const placed = await view.read(placement.table).findFirst({ where: { id: row.id } });
        const asked = placed?.schedule;

        // keep a person's move, and this scheduler's move while the decision still asks for it
        const isAsked = asked?.kind === "move" && asked.machine === moving?.targetId;
        const isKept = moving !== undefined && (isAsked || !(await Scheduler.isOwn(moving, store)));

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

    /** Report whether the space's base stack, which runs this scheduler, requested a move. */
    async isOwn(
        moving: Select<typeof transfer.table>,
        store: Pick<ObjectStore, "environment">,
    ): Promise<boolean> {
        // derive the space's base stack installation, which requests the scheduler's moves
        const space = principal.space.reference(Scope.universe.id, moving.scope);
        const base = await Installation.base(space, stackPackage, store.environment.crypto);
        const own = principal.installation.reference(moving.scope, base.installation);

        return (
            ObjectReference.key(ObjectReference.at("requestedBy", moving)) ===
            ObjectReference.key(own)
        );
    },
};
