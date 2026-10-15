import { contained, none, principal, relation, through, union } from "@destack/access";
import type { Snapshot } from "@destack/db";
import { defineObject, defineShape, field } from "@destack/object";
import { defineSchema, Digest, Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Scope } from "@destack/sync";
import { account } from "./account.ts";
import { endpoint } from "./endpoint.ts";
import { machine } from "./machine.ts";

/** Where a space runs: the machine serving its databases at an epoch. */
const PlacementSchema = defineSchema(
    schema.object({
        /** The placed space. */
        id: schema.identifier("space"),
        /** The account the space belongs to. */
        scope: schema.identifier("account"),
        /** The machine serving the space's databases. */
        machine: schema.identifier("machine"),
        /** The epoch the machine serves the space at. */
        epoch: schema.number().int().min(1),
    }),
);
/** Where a space runs, and the machine a new space goes on. */
export const Placement = Object.assign(PlacementSchema, {
    /** Pick the machine for a space by rendezvous hashing, or none from no machines. */
    async pick(spaceId: string, machines: readonly string[]): Promise<string | undefined> {
        // keep the machine whose digest with the space scores highest
        let best: { readonly machine: string; readonly score: string } | undefined;
        for (const candidate of machines) {
            const score = await Digest.of(`${spaceId}:${candidate}`);
            if (best === undefined || score > best.score) {
                best = { machine: candidate, score };
            }
        }

        return best?.machine;
    },

    /** Read the placements of some spaces in one read, skipping unplaced spaces. */
    async find(
        reader: Pick<Snapshot, "read">,
        spaces: readonly Identifier<"space">[],
    ): Promise<Placement[]> {
        const rows = await reader
            .read(placement.table)
            .findMany({ where: { id: { in: [...spaces] } } });

        return rows.map((row) => ({
            id: row.id,
            scope: row.scope,
            machine: schema.identifier("machine").parse(row.machineId),
            epoch: row.epoch,
        }));
    },

    /** Require a standing machine of the space's account, or a shared machine of its residency. */
    async requireAdmitted(
        reader: Pick<Snapshot, "read">,
        row: Pick<Placement, "scope">,
        target: Identifier<"machine">,
    ): Promise<Identifier<"machine">> {
        // read the target, admitting only an enabled or draining unrevoked machine
        const found = await reader.read(machine.table).findFirst({ where: { id: target } });
        const isStanding =
            found !== undefined && found.revokedAt === null && found.status !== "disabled";

        // admit a machine of the account, or a shared machine of the account's residency
        const isOwn = isStanding && found.scope === row.scope;
        const isShared =
            isStanding &&
            found.scope === Scope.universe.id &&
            found.residencyId ===
                present(
                    await reader.read(account.table).findFirst({ where: { id: row.scope } }),
                    `the account ${row.scope}`,
                ).residencyId;
        if (!isOwn && !isShared) {
            throw new CallError("BAD_REQUEST", {
                message: `${target} is no standing machine of the space's account nor of its residency`,
            });
        }

        return target;
    },

    /** Refuse a machine neither serving nor receiving a space, nor working in it. */
    async requireWorker(
        reader: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        machineId: Identifier<"machine">,
    ): Promise<void> {
        // read the space's placement and the machine's work in it
        const placed = await reader.read(placement.table).findFirst({ where: { id: spaceId } });
        const assigned = await reader.read(assignment.table).findFirst({
            where: {
                placementId: spaceId,
                machineId,
            },
        });

        // refuse a machine neither serving nor receiving the space, nor working in it
        const isPlaced = machineId === placed?.machineId || machineId === placed?.targetId;
        if (!isPlaced && assigned === undefined) {
            throw new CallError("FORBIDDEN", {
                message: `${spaceId} is served and received elsewhere`,
            });
        }
    },

    /** Refuse a call at an epoch other than the space's current epoch. */
    requireServed(row: Pick<Placement, "id" | "epoch">, epoch: number): void {
        if (row.epoch !== epoch) {
            throw new CallError("CONFLICT", {
                message: `${row.id} is no longer placed at epoch ${epoch}`,
            });
        }
    },
});
/** Where a space runs. */
export type Placement = schema.Infer<typeof PlacementSchema>;

/** The epoch the calling machine serves a space at, which fences a previous machine. */
const Serving = schema.object({
    /** The epoch the calling machine serves the space at. */
    epoch: schema.number().int().min(1),
});

/** The machine to give work in a space, with the serving machine's epoch. */
const Worker = Serving.extend({
    /** The machine the work is for. */
    machine: schema.identifier("machine"),
});

/** Where a space runs. */
export const placement = defineObject({
    name: "placement",
    plural: "placements",
    scope: account,
    key: schema.identifier("space"),
    fields: {
        /** The machine serving the space. */
        machine: field.reference(principal.machine),
        /** The epoch the machine serves the space at. */
        epoch: field.integer(),
        /** The machine a pending move takes the space to. */
        target: field.reference(principal.machine).optional(),
        /** The transfer moving the space to the target machine. */
        transferId: field.string(schema.identifier("transfer")).optional(),
    },
    relations: {
        // the placed space itself, under the placement's own identifier
        self: { subjects: [principal.space], grantedBy: null },
    },
    permissions: {
        read: union(
            relation("self"),
            relation("machine"),
            relation("target"),
            through("account", "read"),
            through("account", "serve"),
            contained(principal.installation),
        ),
        create: none(),
        place: union(relation("machine"), relation("target")),
        move: relation("machine"),
        // the machine serving the space alone, which owners' roles leave out
        serve: relation("machine"),
    },
    reserved: ["serve"],
    constraints: {
        epoch: { kind: "check", condition: { epoch: { gt: 0 } } },
        target: {
            kind: "check",
            condition: {
                targetId: { OR: [{ isNull: true }, { ne: { field: "machineId" } }] },
            },
        },
        transfer: {
            kind: "check",
            condition: {
                OR: [
                    { targetId: { isNull: true }, transferId: { isNull: true } },
                    { targetId: { isNotNull: true }, transferId: { isNotNull: true } },
                ],
            },
        },
        targeted: { kind: "index", on: ["targetId"] },
    },
}).methods((method) => ({
    create: method.create("create", { isInternal: true, fields: ["machineId", "epoch"] }),

    /** Serve a space at an epoch. */
    place: method.mutation({ permission: "place", input: Serving }),

    /** Mark a space as moving to a target machine, as the machine serving it at its epoch. */
    move: method.mutation({
        permission: "move",
        input: Serving.extend({
            /** The machine the space moves to. */
            target: schema.identifier("machine"),
            /** The transfer moving it, which hands the target its identifier. */
            transferId: schema.identifier("transfer"),
        }),
    }),

    /** End a space's placement, as the machine serving it at its epoch. */
    withdraw: method.mutation({ permission: "move", input: Serving }),

    /** Give a machine work in the space, as the machine serving it at its epoch. */
    assign: method.mutation({ permission: "move", input: Worker }),

    /** Withdraw a machine's work in the space, as the machine serving it at its epoch. */
    unassign: method.mutation({ permission: "move", input: Worker }),
}));

/** Work a space gives a machine beside its serving machine, such as a checkout build. */
export const assignment = defineObject({
    name: "assignment",
    plural: "assignments",
    scope: "universe",
    fields: {
        /** The placement of the space with the work, whose removal ends the work. */
        placement: field.reference(placement, { delete: "cascade" }),
        /** The machine the work is for. */
        machine: field.reference(principal.machine),
    },
    // let the machine with the work and the machine serving the space read it
    permissions: {
        read: union(relation("machine"), through("placement", "move")),
        give: none(),
    },
    constraints: {
        work: { kind: "unique", on: ["placementId", "machineId"] },
        machine: { kind: "index", on: ["machineId"] },
    },
}).methods((method) => ({
    /** Give a machine work in a space, as the machine serving it. */
    create: method.create("give", { fields: ["placementId", "machineId"], isInternal: true }),
    /** End a machine's work in a space, as the machine serving it. */
    delete: method.delete("give", { isInternal: true }),
}));

/** One machine's own row, endpoint, placements and assignments, replicated to it. */
export const placementShape = defineShape({
    name: "placement",
    parameters: schema.object({ machine: schema.string().min(1) }),
    objects: [machine, endpoint, placement, assignment],
    queries: ({ machine: id }) => [
        machine.query({ scopes: "every", where: { id } }),
        endpoint.query({ scopes: "every", where: { id } }),
        placement.query({
            scopes: "every",
            where: { OR: [{ machineId: id }, { targetId: id }] },
        }),
        assignment.query({ where: { machineId: id } }),
    ],
});
