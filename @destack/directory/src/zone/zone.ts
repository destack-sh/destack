import { check, defineTable, index, integer, sql, text } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";

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

/** A zone and the URL its cell answers at. */
export interface Location extends Zone {
    /** The URL the cell's services answer at. */
    readonly endpoint: string;
}

/** The zones placed in the cells that serve their databases. */
export const zoneTable = defineTable(
    "zone",
    {
        /** The scope the databases belong to. */
        id: text("id").primaryKey(),
        /** The scope containing it, such as its account. */
        scope: text("scope").notNull(),
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
            index("zone_incoming").on(zone.target),
            index("zone_scope").on(zone.scope),
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
