import { Caller, principal, Relationship } from "@destack/access";
import {
    type KeyFields,
    type MethodContext,
    type MethodContextOf,
    type ObjectReconciliation,
    ObjectWatch,
    type ResultOf,
} from "@destack/object";
import { Change, type DatabaseConnection, type Select, Snapshot } from "@destack/db";
import { canonicalize, type DigestCrypto, type Identifier, present, schema } from "@destack/schema";
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
    return placement.implement({
        methods: { place, move, decide, withdraw, assign, unassign },
        controller: {
            // decide every placement again once a machine, plan or residency it depends on changes
            pending: {},
            watches: [
                // decide again once a machine's standing changes, for its account or its residency's
                ObjectWatch.of(machine.table, async (_row, database, change) => {
                    // leave a change no decision reads, such as a heartbeat
                    const before = Change.before(change);
                    const after = Change.after(change);
                    if (!isMachineChanged(before, after)) {
                        return [];
                    }

                    // decide again for the accounts before and after the change
                    const sides = [before, after].filter((side) => side !== null);
                    const keys = await Promise.all(
                        sides.map((side) => machinePlacements(database, side)),
                    );

                    return keys.flat();
                }),
                // decide again for an account whose tenancy entitlement changed
                ObjectWatch.of(entitlement.table, (row, database) =>
                    row.resource === "tenancy"
                        ? accountPlacements(database, [
                              schema.identifier("account").parse(row.scope),
                          ])
                        : [],
                ),
                // decide again for an account whose residency changed
                ObjectWatch.of(account.table, (row, database, change) =>
                    row.residencyId === Change.before(change)?.residencyId
                        ? []
                        : accountPlacements(database, [row.id]),
                ),
            ],
            reconcile,
        },
    });
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

    // serve the space at the epoch, ending any move and its receiver's access, decided again for its machine
    await releaseReceivers(context, current, null);
    const placed = { ...current, machineId: caller };
    const decided = await decideSchedule(context, placed, context.environment.crypto);

    return context.update({
        machineId: caller,
        epoch,
        targetId: null,
        transferId: null,
        schedule: decided,
    });
}

/** Mark a space as moving to a target machine, as the machine serving it at its epoch. */
async function move(
    context: MethodContextOf<typeof placement, "move">,
): Promise<ResultOf<typeof placement, "move">> {
    // mark the move at the machine's epoch
    const { target } = context;
    Placement.requireServed(target, context.input.epoch);
    await Placement.requireMachine(context, target, context.input.target);

    // relate the target as the space's one receiver, keeping its relationship when it already is one
    await releaseReceivers(context, target, context.input.target);
    await StoreRelationship.touch(context, {
        object: space.reference(target.scope, target.id),
        relation: "receiver",
        subject: principal.machine.reference(Scope.universe.id, context.input.target),
    });

    return context.update({ targetId: context.input.target, transferId: context.input.transferId });
}

/** Revoke the receivers of a space's moves but a kept machine, as the store. */
async function releaseReceivers(
    context: MethodContext,
    row: Pick<Placement, "id" | "scope">,
    kept: Identifier<"machine"> | null,
): Promise<void> {
    // read the receivers other than the kept machine
    const object = space.reference(row.scope, row.id);
    const receivers = await Relationship.readOn(context, object, { relation: "receiver" });
    const released = receivers.filter((each) => each.subjectId !== kept);

    // revoke each, which ends its replication of the space's chain
    for (const each of released) {
        const subject = ObjectReference.at("subject", each);
        await StoreRelationship.delete(context, { object, relation: "receiver", subject });
    }
}

/** Decide where a space's tenancy places it, recording a changed decision. */
async function decide(
    context: MethodContextOf<typeof placement, "decide">,
): Promise<ResultOf<typeof placement, "decide">> {
    const { target } = context;
    const decided = await decideSchedule(context, target, context.environment.crypto);

    return canonicalize(target.schedule) === canonicalize(decided)
        ? target
        : context.update({ schedule: decided });
}

/** Decide where each pending placement's tenancy places its space, recording changed decisions. */
async function reconcile(
    reconciliation: ObjectReconciliation<typeof placement>,
): Promise<undefined> {
    // find the placements whose decision changed, as the latest commits read
    const reader = Snapshot.live(reconciliation.database);
    const crypto = reconciliation.store.environment.crypto;
    const decided = await Promise.all(
        reconciliation.rows.map(async (row) => ({
            row,
            isChanged:
                canonicalize(row.schedule) !==
                canonicalize(await decideSchedule(reader, row, crypto)),
        })),
    );
    const changed = decided.filter(({ isChanged }) => isChanged);

    // decide them again in one transaction, which reads what it decides on
    if (changed.length > 0) {
        await reconciliation.mutate(
            changed.map(({ row }) => placement.call("decide", placement.target(row))),
        );
    }

    return undefined;
}

/** Decide where a space's tenancy places it, by its plan and the machines it may use. */
export async function decideSchedule(
    reader: Pick<Snapshot, "read">,
    row: Pick<Select<typeof placement.table>, "id" | "scope" | "machineId">,
    crypto: DigestCrypto,
): Promise<Schedule> {
    // read the space's tenancy, its account's entitlement and the tenancy of the machine serving it
    const placed = await reader.read(space.table).findFirst({ where: { id: row.id } });
    const { tenancy } = present(placed, `space ${row.id}`);
    const entitled = await reader.read(entitlement.table).findMany({ where: { scope: row.scope } });
    const serving = await reader.read(machine.table).findFirst({ where: { id: row.machineId } });
    const local = Machine.tenancy(present(serving, `machine ${row.machineId}`));

    // keep a space on its machine until a person sets its tenancy
    if (tenancy === null) {
        const message = `the space stays ${local} on ${row.machineId} until a person sets its tenancy`;

        return { kind: "stay", condition: { status: "true", reason: "Unset", message } };
    }

    // refuse a tenancy the account's plan does not unlock
    const allowed = Entitlement.decide(entitled, { tenancy });
    if (allowed.status !== "true") {
        return { kind: "stay", condition: allowed };
    }
    // leave a space on its machine when its tenancy matches
    else if (tenancy === local) {
        const message = `placed ${local} on ${row.machineId}`;

        return { kind: "stay", condition: { status: "true", reason: "Placed", message } };
    }
    // move it to its tenancy's machine, or leave it where no machine runs the tenancy
    else {
        return await tenancyMachine(reader, row, tenancy, crypto);
    }
}

/** Find the machine running a tenancy for a space, a residency's or the account's own. */
async function tenancyMachine(
    reader: Pick<Snapshot, "read">,
    row: Pick<Select<typeof placement.table>, "id" | "scope">,
    asked: Tenancy,
    crypto: DigestCrypto,
): Promise<Schedule> {
    // read the residency's cloud machines for a shared space, else the account's machines of the tenancy's kind
    const kind = asked === "dedicated" ? "cloud" : "personal";
    const owner = await reader.read(account.table).findFirst({ where: { id: row.scope } });
    const residencyId = present(owner, `account ${row.scope}`).residencyId;
    const standing = { status: "enabled", revokedAt: { isNull: true } } as const;
    const candidates = await reader.read(machine.table).findMany({
        where:
            asked === "shared"
                ? { scope: Scope.universe.id, residencyId, ...standing }
                : { scope: row.scope, kind, ...standing },
        orderBy: { id: "asc" },
    });
    const ids = candidates.map((candidate) => candidate.id);
    const [only] = ids;

    // pick a cloud machine of the residency for a shared space, staying where none serves it
    if (asked === "shared") {
        const picked = await Placement.pick(row.id, ids, crypto);
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

/** Decide whether a machine change can change a decision: added, removed, standing or home. */
function isMachineChanged(
    before: Select<typeof machine.table> | null,
    after: Select<typeof machine.table> | null,
): boolean {
    return (
        before === null ||
        after === null ||
        after.status !== before.status ||
        after.revokedAt !== before.revokedAt ||
        after.kind !== before.kind ||
        after.scope !== before.scope ||
        after.residencyId !== before.residencyId
    );
}

/** List the placements of a machine's account, or of its residency's accounts for a shared one. */
async function machinePlacements(
    database: DatabaseConnection,
    row: Pick<Select<typeof machine.table>, "scope" | "residencyId">,
): Promise<readonly KeyFields[]> {
    // take an account machine's account
    if (row.scope !== Scope.universe.id) {
        return await accountPlacements(database, [schema.identifier("account").parse(row.scope)]);
    }

    // take every account of a shared machine's residency
    const residencyId = present(row.residencyId, "the residency of a shared machine");
    const accounts = await Snapshot.live(database)
        .read(account.table)
        .findMany({ where: { residencyId } });

    return await accountPlacements(
        database,
        accounts.map((each) => each.id),
    );
}

/** List the placements of some accounts as controller keys. */
async function accountPlacements(
    database: DatabaseConnection,
    accounts: readonly Identifier<"account">[],
): Promise<readonly KeyFields[]> {
    const rows = await Snapshot.live(database)
        .read(placement.table)
        .findMany({ where: { scope: { in: [...accounts] } } });

    return rows.map((row) => ({ id: row.id }));
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
