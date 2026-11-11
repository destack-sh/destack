import { contained, none, principal, relation, through, union } from "@destack/access";
import type { Select, Snapshot } from "@destack/db";
import { Condition, defineObject, field, type ObjectOf } from "@destack/object";
import { Digest, type DigestCrypto, Identifier, present, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { account } from "./account.ts";
import { machine } from "./machine.ts";
import { space } from "./space.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { argument, plural, t } from "@destack/locale";

/** Where the universe places a space for its tenancy: stay on its machine or move. */
export const Schedule = schema.discriminatedUnion("kind", [
    schema.object({
        /** The space stays on its machine. */
        kind: schema.literal("stay"),
        /** Why it stays: placed, unset, unentitled or without a machine to move to. */
        condition: Condition,
    }),
    schema.object({
        /** The space moves. */
        kind: schema.literal("move"),
        /** The machine it moves to. */
        machine: schema.identifier("machine"),
    }),
]);
/** Where a space's tenancy places it. */
export type Schedule = schema.Infer<typeof Schedule>;

/** Whether a placed space's machine serves it yet, as the machine reports. */
export const PLACEMENT_STATUSES = ["starting", "serving"] as const;

/** Whether a placed space's machine serves it yet. */
export type PlacementStatus = (typeof PLACEMENT_STATUSES)[number];

/** Where a space runs: the machine serving its databases at an epoch. */
const PlacementSchema = schema.object({
    /** The placed space. */
    space: schema.identifier("space"),
    /** The account the space belongs to. */
    scope: schema.identifier("account"),
    /** The machine serving the space's databases. */
    machine: schema.identifier("machine"),
    /** The epoch the machine serves the space at. */
    epoch: schema.number().int().min(1),
});
/** Where a space runs, and the machine a new space goes on. */
export const Placement = Object.assign(PlacementSchema, {
    /** Pick the machine for a space by rendezvous hashing, or none from no machines. */
    async pick(
        spaceId: Identifier<"space">,
        machines: readonly Identifier<"machine">[],
        crypto: DigestCrypto,
    ): Promise<Identifier<"machine"> | undefined> {
        // choose the machine whose digest with the space scores highest
        let best: { readonly machine: Identifier<"machine">; readonly score: string } | undefined;
        for (const candidate of machines) {
            const score = await Digest.of(`${spaceId}:${candidate}`, crypto);
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
            .findMany({ where: { spaceId: { in: [...spaces] } } });

        return rows.map((row) => ({
            space: row.spaceId,
            scope: row.scope,
            machine: row.machineId,
            epoch: row.epoch,
        }));
    },

    /** Require a standing machine of the space's account, or a shared machine of its residency. */
    async requireMachine(
        reader: Pick<Snapshot, "read">,
        row: Pick<Placement, "scope">,
        target: Identifier<"machine">,
    ): Promise<Identifier<"machine">> {
        // read the target, allowing only an enabled or draining unrevoked machine
        const found = await reader.read(machine.table).findFirst({ where: { id: target } });
        const isStanding =
            found !== undefined && found.revokedAt === null && found.status !== "disabled";

        // allow a machine of the account, or a shared machine of the account's residency
        const isAccountMachine = isStanding && found.scope === row.scope;
        const isShared =
            isStanding &&
            found.scope === Scope.universe.id &&
            found.residencyId ===
                present(
                    await reader.read(account.table).findFirst({ where: { id: row.scope } }),
                    `the account ${row.scope}`,
                ).residencyId;
        if (!isAccountMachine && !isShared) {
            throw AccountError.of(
                "MACHINE_INVALID",
                `machine '${target}' is invalid: it is not a standing machine of the space's account or residency`,
            );
        }

        return target;
    },

    /** Refuse a call at an epoch other than the space's current epoch. */
    requireServed(
        row: Pick<Select<typeof placement.table>, "spaceId" | "epoch">,
        epoch: number,
    ): void {
        if (row.epoch !== epoch) {
            throw AccountError.of(
                "EPOCH_CONFLICT",
                `cannot place this space at epoch ${epoch}: it is no longer placed at that epoch`,
            );
        }
    },
});
/** Where a space runs. */
export type Placement = schema.Infer<typeof PlacementSchema>;

/** A space's placement and the URL its machine answers at now. */
export const PlacementEndpoint = schema.object({
    /** Where the space runs. */
    placement: PlacementSchema,
    /** The URL the placement's machine answers at. */
    url: schema.string(),
    /** Whether the machine serves the space yet. */
    status: schema.enum(PLACEMENT_STATUSES),
});
/** A space's placement and the URL its machine answers at now. */
export type PlacementEndpoint = schema.Infer<typeof PlacementEndpoint>;

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
export const placement = defineObject(
    {
        name: "placement",
        plural: "placements",
        title: t`${plural(argument("count"), { one: "Placement", other: "Placements" })}`,
        scope: account,
        key: "spaceId",
        fields: {
            /** The space placed. */
            space: field
                .reference((): ObjectOf<{ identity: "space" }> => space, { delete: "cascade" })
                .immutable(),
            /** The machine serving the space. */
            machine: field.reference(principal.machine),
            /** The epoch the machine serves the space at. */
            epoch: field.integer(),
            /** The machine a pending move takes the space to. */
            target: field.reference(principal.machine).optional(),
            /** The transfer moving the space to the target machine. */
            transferId: field.string(schema.identifier("transfer")).optional(),
            /** Where the universe places the space for its tenancy. */
            schedule: field.json(Schedule),
            /** Whether the machine serves the space yet, as the machine reports. */
            status: field.enum(PLACEMENT_STATUSES).default("starting"),
        },
        constraints: {
            // place each space once, whichever account it belongs to
            space: { kind: "unique", on: ["spaceId"] },
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
        access: {
            relations: {
                // the placed space itself
                space: { subjects: [principal.space], grantedBy: null },
            },
            permissions: {
                read: union(
                    relation("space"),
                    relation("machine"),
                    relation("target"),
                    through("account", "read"),
                    through("account", "serve"),
                    contained(principal.installation),
                ),
                create: none(),
                place: union(relation("machine"), relation("target")),
                move: relation("machine"),
                // the universe decides where the tenancy places the space
                decide: none(),
                // the machine serving the space alone, which owners' roles leave out
                serve: relation("machine"),
            },
            reserved: ["serve"],
        },
    },
    (method) => ({
        create: method.create("create", {
            visibility: "package",
            fields: ["spaceId", "machineId", "epoch", "schedule"],
        }),

        /** Serve a space at an epoch. */
        place: method.mutation({ title: t`Place`, permission: "place", input: Serving }),

        /** Mark a space as moving to a target machine, as the machine serving it at its epoch. */
        move: method.mutation({
            title: t`Move`,
            permission: "move",
            input: Serving.extend({
                /** The machine the space moves to. */
                target: schema.identifier("machine"),
                /** The transfer moving it, which hands the target its identifier. */
                transferId: schema.identifier("transfer"),
            }),
        }),

        /** Decide again where the universe places the space for its tenancy, as the store. */
        decide: method.mutation({
            title: t`Decide`,
            permission: "decide",
            visibility: "package",
            input: schema.object({}),
        }),

        /** Report whether the machine serves the space yet, at its epoch. */
        report: method.mutation({
            title: t`Report`,
            permission: "serve",
            input: Serving.extend({ status: schema.enum(PLACEMENT_STATUSES) }),
        }),

        /** Read the account paying for the placed space's use, as the machine serving it. */
        payer: method.query({
            title: t`Read payer`,
            permission: "serve",
            output: schema.identifier("account"),
        }),

        /** End a space's placement, as the machine serving it at its epoch. */
        withdraw: method.mutation({ title: t`Withdraw`, permission: "move", input: Serving }),

        /** Give a machine work in the space, as the machine serving it at its epoch. */
        assign: method.mutation({ title: t`Assign`, permission: "move", input: Worker }),

        /** Withdraw a machine's work in the space, as the machine serving it at its epoch. */
        unassign: method.mutation({ title: t`Unassign`, permission: "move", input: Worker }),
    }),
);

/** Work a space gives a machine beside its serving machine, such as a checkout build. */
export const assignment = defineObject(
    {
        name: "assignment",
        plural: "assignments",
        title: t`${plural(argument("count"), { one: "Assignment", other: "Assignments" })}`,
        scope: "universe",
        fields: {
            /** The placement of the space with the work, whose removal ends the work. */
            placement: field.reference(placement, { delete: "cascade" }),
            /** The machine the work is for. */
            machine: field.reference(principal.machine),
        },
        constraints: {
            work: { kind: "unique", on: ["placementId", "machineId"] },
            machine: { kind: "index", on: ["machineId"] },
        },
        access: {
            // let the machine with the work and the machine serving the space read it
            permissions: {
                read: union(relation("machine"), through("placement", "move")),
                give: none(),
            },
        },
    },
    (method) => ({
        /** Give a machine work in a space, as the machine serving it. */
        create: method.create("give", {
            fields: ["placementId", "machineId"],
            visibility: "package",
        }),
        /** End a machine's work in a space, as the machine serving it. */
        delete: method.delete("give", { visibility: "package" }),
    }),
);
