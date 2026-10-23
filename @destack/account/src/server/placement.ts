import { Caller } from "@destack/access";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { assignment, Placement, placement } from "../object/placement.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** Implement placements. */
export function implementPlacement(): typeof placement {
    return placement.implement({ methods: { place, move, withdraw, assign, unassign } });
}

/** Serve a space at an epoch as the calling machine, ending any move. */
async function place(
    context: MethodContextOf<typeof placement, "place">,
): Promise<ResultOf<typeof placement, "place">> {
    // read the calling machine and its epoch
    const caller = schema.identifier("machine").parse(Caller.sender(context.caller).id);
    const { epoch } = context.input;

    // leave the placement at its epoch, advance the epoch, or take the space over
    const current = context.target;
    const isRetained = current.epoch === epoch && current.machineId === caller;
    const isAdvanced =
        current.epoch + 1 === epoch &&
        (current.machineId === caller || current.targetId === caller);

    // refuse any other epoch or machine
    if (!isRetained && !isAdvanced) {
        throw AccountError.of(
            "PLACEMENT_MISMATCH",
            `${current.id} is not placed on or moving to ${caller} at epoch ${epoch}`,
        );
    }

    // serve the space at the epoch, ending any move
    return context.update({ machineId: caller, epoch, targetId: null, transferId: null });
}

/** Mark a space as moving to a target machine, as the machine serving it at its epoch. */
async function move(
    context: MethodContextOf<typeof placement, "move">,
): Promise<ResultOf<typeof placement, "move">> {
    // mark the move at the machine's epoch
    const { target } = context;
    Placement.requireServed(target, context.input.epoch);
    await Placement.requireMachine(context, target, context.input.target);

    return context.update({ targetId: context.input.target, transferId: context.input.transferId });
}

/** End a space's placement as the machine serving it, its work with it. */
async function withdraw(
    context: MethodContextOf<typeof placement, "withdraw">,
): Promise<ResultOf<typeof placement, "withdraw">> {
    // remove the placement at the machine's epoch, which removes the work its assignments give
    const { target } = context;
    Placement.requireServed(target, context.input.epoch);
    await context.remove();

    return target;
}

/** Give a machine work in a space once, as the machine serving it at its epoch. */
async function assign(
    context: MethodContextOf<typeof placement, "assign">,
): Promise<ResultOf<typeof placement, "assign">> {
    // read the machine's work in the space at the machine's epoch
    const { target } = context;
    Placement.requireServed(target, context.input.epoch);
    const work = { placementId: target.id, machineId: context.input.machine };
    const assigned = await context.read(assignment).findFirst({ where: work });

    // give the work once
    if (assigned === undefined) {
        await context.mutate(context.self, [assignment.call("create", work)]);
    }

    return target;
}

/** Withdraw a machine's work in a space, as the machine serving it at its epoch. */
async function unassign(
    context: MethodContextOf<typeof placement, "unassign">,
): Promise<ResultOf<typeof placement, "unassign">> {
    // end the machine's work at the machine's epoch
    const { target } = context;
    Placement.requireServed(target, context.input.epoch);
    const assigned = await context
        .read(assignment)
        .findMany({ where: { placementId: target.id, machineId: context.input.machine } });
    if (assigned.length > 0) {
        await context.mutate(
            context.self,
            assigned.map((row) => assignment.call("delete", assignment.target(row))),
        );
    }

    return target;
}
