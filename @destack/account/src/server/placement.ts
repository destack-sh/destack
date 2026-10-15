import { Caller } from "@destack/access";
import type { CallOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { assignment, Placement, placement } from "../object/placement.ts";

/** Implement placements. */
export function implementPlacement(): typeof placement {
    return placement.implement({ methods: { place, move, withdraw, assign, unassign } });
}

/** Serve a space at an epoch as the calling machine, ending any move. */
async function place(
    call: CallOf<typeof placement, "place">,
): Promise<ResultOf<typeof placement, "place">> {
    // read the calling machine and its epoch
    const caller = schema.identifier("machine").parse(Caller.sender(call.caller).id);
    const { epoch } = call.input;

    // keep the placement at its epoch, advance the epoch, or take the space over
    const current = call.target;
    const isKept = current.epoch === epoch && current.machineId === caller;
    const isAdvanced =
        current.epoch + 1 === epoch &&
        (current.machineId === caller || current.targetId === caller);

    // refuse any other epoch or machine
    if (!isKept && !isAdvanced) {
        throw new CallError("CONFLICT", {
            message: `${current.id} is neither placed on ${caller} at epoch ${epoch} nor moving there`,
        });
    }

    // serve the space at the epoch, ending any move
    return call.update({ machineId: caller, epoch, targetId: null, transferId: null });
}

/** Mark a space as moving to a target machine, as the machine serving it at its epoch. */
async function move(
    call: CallOf<typeof placement, "move">,
): Promise<ResultOf<typeof placement, "move">> {
    // mark the move at the machine's epoch
    const { target } = call;
    Placement.requireServed(target, call.input.epoch);
    await Placement.requireAdmitted(call, target, call.input.target);

    return call.update({ targetId: call.input.target, transferId: call.input.transferId });
}

/** End a space's placement as the machine serving it, its work with it. */
async function withdraw(
    call: CallOf<typeof placement, "withdraw">,
): Promise<ResultOf<typeof placement, "withdraw">> {
    // remove the placement at the machine's epoch, which removes the work its assignments give
    const { target } = call;
    Placement.requireServed(target, call.input.epoch);
    await call.remove();

    return target;
}

/** Give a machine work in a space once, as the machine serving it at its epoch. */
async function assign(
    call: CallOf<typeof placement, "assign">,
): Promise<ResultOf<typeof placement, "assign">> {
    // read the machine's work in the space at the machine's epoch
    const { target } = call;
    Placement.requireServed(target, call.input.epoch);
    const work = { placementId: target.id, machineId: call.input.machine };
    const assigned = await call.read(assignment).findFirst({ where: work });

    // give the work once
    if (assigned === undefined) {
        await call.mutate(call.self, [assignment.call("create", work)]);
    }

    return target;
}

/** Withdraw a machine's work in a space, as the machine serving it at its epoch. */
async function unassign(
    call: CallOf<typeof placement, "unassign">,
): Promise<ResultOf<typeof placement, "unassign">> {
    // end the machine's work at the machine's epoch
    const { target } = call;
    Placement.requireServed(target, call.input.epoch);
    const assigned = await call
        .read(assignment)
        .findMany({ where: { placementId: target.id, machineId: call.input.machine } });
    if (assigned.length > 0) {
        await call.mutate(
            call.self,
            assigned.map((row) => assignment.call("delete", assignment.target(row))),
        );
    }

    return target;
}
