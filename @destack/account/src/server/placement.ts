import { Caller, principal, Relationship } from "@destack/access";
import type {
    MethodContext,
    MethodContextOf,
    ObjectReconciliation,
    ResultOf,
} from "@destack/object";
import type { Select } from "@destack/db";
import type { Snapshot, Watch } from "@destack/db/database";
import { canonicalize, type DigestCrypto, type Identifier, present, schema } from "@destack/schema";
import { Account, account } from "../object/account.ts";
import type { Entitlement } from "./entitlement.ts";
import { Machine, machine } from "../object/machine.ts";
import { assignment, Placement, placement, type Schedule } from "../object/placement.ts";
import { ObjectReference, Scope } from "@destack/sync";
import { space, type Tenancy } from "../object/space.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { StoreRelationship } from "./relationship.ts";

/** Implement placements, deciding tenancies by the accounts' plans. */
export function implementPlacement(
    entitlement: (accountId: Identifier<"account">) => Entitlement,
    watches: readonly Watch[],
): typeof placement {
    return placement.implement({
        methods: {
            create: {
                // let the machine it is placed on serve the space and its account
                handle: async (context, next) => {
                    const created = await next();
                    await serve(context, created, created.machineId);

                    return created;
                },
            },
            place: (context) => place(context, entitlement),
            move,
            decide: (context) => decide(context, entitlement),
            withdraw,
            report,
            assign,
            unassign,
            payer: (context) => Account.payer(context, context.target.scope),
        },
        controllers: [
            {
                name: "decide",
                // decide a placement again after each of its writes, which converges a decision a concurrent change outdated
                reads: [
                    // decide again once a machine comes, goes or changes its standing, for its account
                    {
                        table: machine.table,
                        fields: ["status", "deletedAt", "kind", "regionId"],
                        on: { scope: "scope" },
                    },
                    // TODO #Performance: decide only the machine's own and the waiting placements, once space keeps its machine

                    // decide again for every account of a managed machine's region
                    {
                        table: machine.table,
                        fields: ["status", "deletedAt", "kind", "regionId"],
                        path: [
                            {
                                table: account.table,
                                cardinality: "many",
                                on: { kind: "key", column: "regionId", parent: "regionId" },
                            },
                        ],
                        on: { scope: "id" },
                    },
                    // decide again for an account whose plan changed
                    ...watches,
                ],
                reconcile: (reconciliation) => decideChanged(reconciliation, entitlement),
            },
        ],
    });
}

/** Serve a space at an epoch as the calling machine, ending any move. */
async function place(
    context: MethodContextOf<typeof placement, "place">,
    entitlement: (accountId: Identifier<"account">) => Entitlement,
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
            "PLACEMENT_CONFLICT",
            `cannot place this space on '${caller}': it is not placed on or moving to it at epoch ${epoch}`,
        );
    }

    // end any move and its receiver's access, and hand serving to a machine taking the space over
    await releaseReceivers(context, current, null);
    if (current.machineId !== caller) {
        await unserve(context, current, current.machineId);
        await serve(context, current, caller);
    }

    // serve the space at the epoch, decided again for the machine
    const placed = { ...current, machineId: caller };
    const decided = await decideSchedule(context, placed, context.environment.crypto, entitlement);

    return context.update({
        machineId: caller,
        epoch,
        targetId: null,
        transferId: null,
        schedule: decided,
        status: "starting",
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
        object: space.reference(target.scope, target.spaceId),
        relation: "receiver",
        subject: principal.machine.reference(Scope.universe.id, context.input.target),
    });

    return context.update({ targetId: context.input.target, transferId: context.input.transferId });
}

/** Let a machine serve a placed space and its account, which the account's sellers meter by. */
async function serve(
    context: MethodContext,
    row: Pick<Select<typeof placement.table>, "spaceId" | "scope">,
    machineId: Identifier<"machine">,
): Promise<void> {
    const subject = principal.machine.reference(Scope.universe.id, machineId);
    await StoreRelationship.touch(context, {
        object: space.reference(row.scope, row.spaceId),
        relation: "machine",
        subject,
    });
    await StoreRelationship.touch(context, {
        object: await accountOf(context, row.scope),
        relation: "server",
        subject,
    });
}

/** End a machine's serving of a placed space, and of its account once it serves none else. */
async function unserve(
    context: MethodContext,
    row: Pick<Select<typeof placement.table>, "spaceId" | "scope">,
    machineId: Identifier<"machine">,
): Promise<void> {
    // end its serving of the space
    const subject = principal.machine.reference(Scope.universe.id, machineId);
    await StoreRelationship.delete(context, {
        object: space.reference(row.scope, row.spaceId),
        relation: "machine",
        subject,
    });

    // end its serving of the account once no other space of it stays placed there
    const others = await context.read(placement).findMany({
        where: { scope: row.scope, machineId, spaceId: { ne: row.spaceId } },
    });
    if (others.length === 0) {
        await StoreRelationship.delete(context, {
            object: await accountOf(context, row.scope),
            relation: "server",
            subject,
        });
    }
}

/** Revoke the receivers of a space's moves but a kept machine, as the store. */
async function releaseReceivers(
    context: MethodContext,
    row: Pick<Select<typeof placement.table>, "spaceId" | "scope">,
    kept: Identifier<"machine"> | null,
): Promise<void> {
    // read the receivers other than the kept machine
    const object = space.reference(row.scope, row.spaceId);
    const receivers = await Relationship.read(context, object, { relation: "receiver" });
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
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<ResultOf<typeof placement, "decide">> {
    const { target } = context;
    const decided = await decideSchedule(context, target, context.environment.crypto, entitlement);

    return canonicalize(target.schedule) === canonicalize(decided)
        ? target
        : context.update({ schedule: decided });
}

/** Decide a placement again as its inputs changed, recording a changed decision. */
async function decideChanged(
    reconciliation: ObjectReconciliation<typeof placement>,
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<void> {
    // find the placements whose decision changed, as the latest commits read
    const reader = reconciliation.database.snapshot();
    const crypto = reconciliation.store.environment.crypto;
    const decided = await Promise.all(
        reconciliation.rows.map(async (row) => ({
            row,
            isChanged:
                canonicalize(row.schedule) !==
                canonicalize(await decideSchedule(reader, row, crypto, entitlement)),
        })),
    );
    const changed = decided.filter(({ isChanged }) => isChanged);

    // decide them again in one transaction, which reads what it decides on
    if (changed.length > 0) {
        await reconciliation.mutate(
            changed.map(({ row }) => placement.call("decide", placement.target(row))),
        );
    }
}

/** Decide where a space's tenancy places it, by its plan and the machines it may use. */
export async function decideSchedule(
    reader: Pick<Snapshot, "read">,
    row: Pick<Select<typeof placement.table>, "spaceId" | "scope" | "machineId">,
    crypto: DigestCrypto,
    entitlement: (accountId: Identifier<"account">) => Entitlement,
): Promise<Schedule> {
    // read the space's tenancy and the machine serving it
    const placed = present(
        await reader.read(space.table).findFirst({ where: { id: row.spaceId } }),
        `space ${row.spaceId}`,
    );
    const serving = present(
        await reader.read(machine.table).findFirst({ where: { id: row.machineId } }),
        `machine ${row.machineId}`,
    );

    // keep the serving machine's tenancy while none is set, and the machine while it stands
    const current = Machine.tenancy(serving, row.scope);
    const asked = placed.tenancy ?? current;
    const allowed = await entitlement(row.scope).decide({
        kind: "tenancy",
        tenancy: asked,
    });
    const isStanding = serving.status === "enabled" && serving.deletedAt === null;

    // refuse a tenancy a person set that the account's plan does not unlock
    if (placed.tenancy !== null && allowed.status !== "true") {
        return { kind: "stay", condition: allowed };
    }
    // leave a space on a standing machine of its tenancy, saying whether a person set it
    else if (asked === current && isStanding) {
        const reason = placed.tenancy === null ? "Unset" : "Placed";
        const message = `placed ${current} on ${row.machineId}`;

        return { kind: "stay", condition: { status: "true", reason, message } };
    }
    // move it to its tenancy's machine, or leave it where no machine runs the tenancy
    else {
        return await tenancyMachine(reader, row, asked, crypto);
    }
}

/** Find the machine running a tenancy for a space, a region's or the account's own. */
async function tenancyMachine(
    reader: Pick<Snapshot, "read">,
    row: Pick<Select<typeof placement.table>, "spaceId" | "scope">,
    asked: Tenancy,
    crypto: DigestCrypto,
): Promise<Schedule> {
    // read the candidates for a shared space, else the account's own machines of the tenancy's kind
    const kind = asked === "self" ? "self" : "managed";
    const owner = present(
        await reader.read(account.table).findFirst({ where: { id: row.scope } }),
        `account ${row.scope}`,
    );
    const { regionId } = owner;
    const standing = { status: "enabled", deletedAt: { isNull: true } } as const;
    const ids =
        asked === "shared"
            ? await Placement.candidates(reader, owner)
            : (
                  await reader.read(machine.table).findMany({
                      where: { scope: row.scope, kind, ...standing },
                      orderBy: { id: "asc" },
                  })
              ).map((candidate) => candidate.id);
    const [only] = ids;

    // pick a candidate for a shared space, staying where none serves it
    if (asked === "shared") {
        const picked = await Placement.pick(row.spaceId, ids, crypto);
        const message = `no shared machine serves ${regionId}`;

        return picked === undefined
            ? { kind: "stay", condition: { status: "false", reason: "NoSharedMachine", message } }
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
    await unserve(context, target, target.machineId);
    await context.remove();

    return target;
}

/** Record whether the machine serves the space yet, as the machine serving it at its epoch. */
async function report(
    context: MethodContextOf<typeof placement, "report">,
): Promise<ResultOf<typeof placement, "report">> {
    Placement.requireServed(context.target, context.input.epoch);

    return context.update({ status: context.input.status });
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

/** Read an account's reference, the object its serving machines relate to. */
async function accountOf(context: MethodContext, accountId: string): Promise<ObjectReference> {
    const owner = await Account.of(context, accountId);

    return account.reference(owner.scope, owner.id);
}
