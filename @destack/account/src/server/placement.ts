import { Caller, principal, Relationship } from "@destack/access";
import { type MethodContext, type MethodContextOf, type ResultOf } from "@destack/object";
import { present, schema } from "@destack/schema";
import { account } from "../object/account.ts";
import { Entitlement, entitlement } from "../object/entitlement.ts";
import { Machine, machine } from "../object/machine.ts";
import {
    assignment,
    Placement,
    placement,
    type Schedule,
    type Tenancy,
} from "../object/placement.ts";
import { ObjectReference, Scope } from "@destack/sync";
import { space } from "../object/space.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { StoreRelationship } from "./relationship.ts";

/** Implement placements. */
export function implementPlacement(): typeof placement {
    return placement.implement({ methods: { place, move, schedule, withdraw, assign, unassign } });
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

    // serve the space at the epoch, ending any move and its receiver's access
    await releaseReceivers(context, current);

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

    // relate the target as the space's receiver alone, which lets it replicate the space's chain
    await releaseReceivers(context, target);
    await StoreRelationship.touch(context, {
        object: space.reference(target.scope, target.id),
        relation: "receiver",
        subject: principal.machine.reference(Scope.universe.id, context.input.target),
    });

    return context.update({ targetId: context.input.target, transferId: context.input.transferId });
}

/** Revoke the receivers of a space's moves, as the store. */
async function releaseReceivers(
    context: MethodContext,
    row: Pick<Placement, "id" | "scope">,
): Promise<void> {
    const object = space.reference(row.scope, row.id);
    const receivers = await Relationship.readOn(context, object, { relation: "receiver" });
    for (const each of receivers) {
        const subject = ObjectReference.at("subject", each);
        await StoreRelationship.delete(context, { object, relation: "receiver", subject });
    }
}

/** Decide where a space's tenancy places it, as the space or its machine at its epoch. */
async function schedule(
    context: MethodContextOf<typeof placement, "schedule">,
): Promise<ResultOf<typeof placement, "schedule">> {
    // read the account's entitlement of the tenancy and the tenancy of the machine serving the space
    const { target } = context;
    const { tenancy: asked, source } = context.input;
    Placement.requireServed(target, context.input.epoch);
    const entitled = await context.read(entitlement).findMany({ where: { scope: target.scope } });
    const allowed = Entitlement.decide(entitled, { tenancy: asked });
    const serving = await context.read(machine).findFirst({ where: { id: target.machineId } });
    const local = Machine.tenancy(present(serving, `machine ${target.machineId}`));

    // refuse a tenancy the account's plan does not unlock
    if (allowed.status !== "true") {
        return { kind: "stay", condition: allowed };
    }
    // leave a space on its machine when its tenancy matches
    else if (asked === local) {
        const message = `placed ${local} on ${target.machineId}`;

        return { kind: "stay", condition: { status: "true", reason: "Placed", message } };
    }
    // leave a space on its own machine until a person sets its tenancy
    else if (source === "default" && local === "local") {
        const message = `the space keeps the default tenancy ${asked} on its machine until tenancy is set`;

        return { kind: "stay", condition: { status: "false", reason: "Unset", message } };
    }
    // move it to its tenancy's machine, or leave it where no machine runs the tenancy
    else {
        return await tenancyMachine(context, target, asked);
    }
}

/** Find the machine running a tenancy for a space, a residency's or the account's own. */
async function tenancyMachine(
    context: MethodContextOf<typeof placement, "schedule">,
    target: Pick<Placement, "id" | "scope">,
    asked: Tenancy,
): Promise<Schedule> {
    // read the residency's cloud machines for a shared space, else the account's machines of the tenancy's kind
    const kind = asked === "dedicated" ? "cloud" : "personal";
    const owner = await context.read(account).findFirst({ where: { id: target.scope } });
    const residencyId = present(owner, `account ${target.scope}`).residencyId;
    const standing = { status: "enabled", revokedAt: { isNull: true } } as const;
    const candidates = await context.read(machine).findMany({
        where:
            asked === "shared"
                ? { scope: Scope.universe.id, residencyId, ...standing }
                : { scope: target.scope, kind, ...standing },
        orderBy: { id: "asc" },
    });
    const ids = candidates.map((row) => row.id);
    const [only] = ids;

    // pick a cloud machine of the residency for a shared space, staying where none serves it
    if (asked === "shared") {
        const picked = await Placement.pick(target.id, ids, context.environment.crypto);
        const message = `no cloud machine serves ${residencyId}`;

        return picked === undefined
            ? { kind: "stay", condition: { status: "false", reason: "NoCloudMachine", message } }
            : { kind: "move", machine: picked };
    }
    // stay without a machine of the kind
    else if (only === undefined) {
        const message = `the account has no ${kind} machine`;

        return { kind: "stay", condition: { status: "false", reason: "NoMachine", message } };
    }
    // stay rather than choose among several
    else if (ids.length > 1) {
        const message = `transfer the space to one of the account's machines: ${ids.join(", ")}`;

        return { kind: "stay", condition: { status: "false", reason: "Ambiguous", message } };
    }
    // move it to the one machine
    else {
        return { kind: "move", machine: only };
    }
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
