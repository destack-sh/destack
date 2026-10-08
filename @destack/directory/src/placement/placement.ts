import {
    check,
    type DatabaseConnection,
    defineTable,
    eq,
    index,
    integer,
    sql,
    text,
} from "@destack/db";
import { defineSchema, schema } from "@destack/schema";

/** Where a space runs: the machine serving its databases at an epoch. */
const PlacementSchema = defineSchema(
    schema.object({
        /** The placed space. */
        id: schema.string().min(1),
        /** The account the space belongs to. */
        scope: schema.string().min(1),
        /** The machine serving the space's databases. */
        machine: schema.string().min(1),
        /** The epoch the machine serves the space at, which fences the previous machine after a move. */
        epoch: schema.number().int().min(1),
    }),
);
/** Where a space runs, and the projection of its row into the space's own database. */
export const Placement = Object.assign(PlacementSchema, {
    /** Write a space's placement row into its database as its machine holds it, or remove it once the machine holds none. */
    async project(
        database: DatabaseConnection,
        space: string,
        placement: Placement | undefined,
    ): Promise<void> {
        // remove the row of a space its machine no longer holds
        if (placement === undefined) {
            await database.delete(placementTable).where(eq(placementTable.id, space));

            return;
        }

        // write the held placement in place of the previous one
        const { id: _id, ...placed } = placement;
        await database
            .insert(placementTable)
            .values(placement)
            .onConflictDoUpdate({ target: placementTable.id, set: placed });
    },
});
/** Where a space runs. */
export type Placement = schema.Infer<typeof PlacementSchema>;

/** The URL a machine serving spaces answers at. */
export const Endpoint = defineSchema(
    schema.object({
        /** The machine. */
        machine: schema.string().min(1),
        /** The scope it belongs to. */
        scope: schema.string().min(1),
        /** The URL its services answer at. */
        url: schema.url(),
    }),
);
/** The URL a machine serving spaces answers at. */
export type Endpoint = schema.Infer<typeof Endpoint>;

/** Where each space runs, in its account, copied to the machines running or receiving it across accounts. */
export const placementTable = defineTable(
    "placement",
    {
        /** The placed space. */
        id: text("id").primaryKey(),
        /** The account the space belongs to. */
        scope: text("scope").notNull(),
        /** The machine serving the space. */
        machine: text("machine").notNull(),
        /** The epoch the machine serves the space at. */
        epoch: integer("epoch").notNull(),
        /** The machine a pending move takes the space to. */
        target: text("target"),
    },
    {
        log: {},
        constraints: (placement) => [
            check("placement_epoch", sql`${placement.epoch} > 0`),
            check(
                "placement_target",
                sql`${placement.target} IS NULL OR ${placement.target} <> ${placement.machine}`,
            ),
            index("placement_target").on(placement.target),
            index("placement_scope").on(placement.scope),
        ],
    },
);

/** The machines a space gives work to beside the machine serving it, such as the machine running a build of its checkout. */
export const assignmentTable = defineTable(
    "assignment",
    {
        /** The space with the work. */
        space: text("space").primaryKey(),
        /** The machine the work is for. */
        machine: text("machine").primaryKey(),
        /** The universe, where every assignment lives. */
        scope: text("scope").notNull(),
        /** When the space's machine assigned the work, in UTC epoch milliseconds. */
        assignedAt: integer("assigned_at").notNull(),
    },
    {
        log: {},
        constraints: (assignment) => [index("assignment_machine").on(assignment.machine)],
    },
);

/** The URLs the machines serving spaces answer at, one per machine. */
export const endpointTable = defineTable(
    "endpoint",
    {
        /** The machine answering at the URL. */
        id: text("id").primaryKey(),
        /** The scope it belongs to. */
        scope: text("scope").notNull(),
        /** The URL its services answer at. */
        url: text("url").notNull(),
        /** The token of the publication, one per tunnel or serving process, which a withdrawal names. */
        publication: text("publication").notNull(),
        /** When the machine last published it, in UTC epoch milliseconds. */
        publishedAt: integer("published_at").notNull(),
    },
    { log: {} },
);
