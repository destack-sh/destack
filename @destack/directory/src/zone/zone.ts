import { check, defineTable, index, integer, primaryKey, sql, text } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";

/** The universe scope, as sync names it, where every zone row lives so a cell copies the zones moving to it across accounts. */
export const ZONE_SCOPE = "universe";

/** A scope with its own databases, placed in the cell serving them. */
export const Zone = defineSchema(
    schema.object({
        /** The scope the databases belong to. */
        id: schema.string().min(1),
        /** The scope containing it, such as its account. */
        scope: schema.string().min(1),
        /** The cell serving the databases. */
        cell: schema.string().min(1),
        /** The placement epoch the cell serves the zone at. */
        epoch: schema.number().int().min(1),
    }),
);
/** A scope with its own databases, placed in the cell serving them. */
export type Zone = schema.Infer<typeof Zone>;

/** A region or host serving the zones placed in it. */
export const Cell = defineSchema(
    schema.object({
        /** The region or host. */
        id: schema.string().min(1),
        /** The scope it belongs to. */
        scope: schema.string().min(1),
        /** The URL its services answer at. */
        endpoint: schema.url(),
    }),
);
/** A region or host serving the zones placed in it. */
export type Cell = schema.Infer<typeof Cell>;

/** The zones placed in the cells that serve their databases. */
export const zoneTable = defineTable(
    "zone",
    {
        /** The scope the databases belong to. */
        id: text("id").primaryKey(),
        /** The universe, where every zone row lives. */
        scope: text("scope").notNull(),
        /** The scope containing the zone's scope, such as its account. */
        parent: text("parent").notNull(),
        /** The cell serving the databases. */
        cell: text("cell").notNull(),
        /** The placement epoch the cell serves the zone at. */
        epoch: integer("epoch").notNull(),
        /** The cell a pending transfer moves the zone to. */
        target: text("target"),
    },
    {
        tier: "global",
        log: {},
        constraints: (zone) => [
            check("zone_epoch", sql`${zone.epoch} > 0`),
            check("zone_target", sql`${zone.target} IS NULL OR ${zone.target} <> ${zone.cell}`),
            index("zone_target").on(zone.target),
            index("zone_parent").on(zone.parent),
        ],
    },
);

/** The cells a zone gives work to beside the cell serving it, such as the host running a build of its checkout. */
export const assignmentTable = defineTable(
    "assignment",
    {
        /** The zone with the work. */
        zone: text("zone").notNull(),
        /** The cell the work is for. */
        cell: text("cell").notNull(),
        /** The universe, where every assignment lives. */
        scope: text("scope").notNull(),
        /** When the zone's cell assigned the work, in UTC epoch milliseconds. */
        assignedAt: integer("assigned_at").notNull(),
    },
    {
        tier: "global",
        log: {},
        constraints: (assignment) => [
            primaryKey({ name: "assignment_key", columns: [assignment.zone, assignment.cell] }),
            index("assignment_cell").on(assignment.cell),
        ],
    },
);

/** The regions and hosts serving zones, with the URL each answers at. */
export const cellTable = defineTable(
    "cell",
    {
        /** The region or host. */
        id: text("id").primaryKey(),
        /** The scope it belongs to. */
        scope: text("scope").notNull(),
        /** The URL its services answer at. */
        endpoint: text("endpoint").notNull(),
        /** When the cell last published its endpoint, in UTC epoch milliseconds. */
        publishedAt: integer("published_at").notNull(),
    },
    { tier: "global", log: {} },
);
