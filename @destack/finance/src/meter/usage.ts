import type { Event, EventKind } from "@destack/event";
import { defineEventKind } from "@destack/event/declare";
import { schema } from "@destack/schema";

/** How long an hour is, in milliseconds. */
const HOUR = 60 * 60 * 1000;

/** How long usage stays as billing evidence: ten years, as invoices are kept. */
const RETENTION = 10 * 365 * 24 * HOUR;

/** A metered use of a resource, measured where it ran and copied to the account paying for it, which rates it. */
export const usage = defineEventKind({
    name: "usage",
    description:
        "A metered use of a resource, measured where it ran and rated for the account paying for it.",
    keys: schema.object({
        /** The meter measuring the use, by its package and name. */
        meter: schema.string(),
        /** The SKU rating the use on the host it ran on, null for a seller's own meter. */
        sku: schema.string().nullable(),
        /** The installation that used it, null for the scope itself. */
        installation: schema.string().nullable(),
        /** The amount used in the meter's unit, or the share of a month a level was kept. */
        quantity: schema.number(),
        /** The level kept at the end of the measure, for meters averaged over time, else null. */
        level: schema.number().nullable(),
    }),
    data: schema.object({
        /** The meter's unit. */
        unit: schema.string(),
        /** When the measure starts, in UTC epoch milliseconds. */
        from: schema.number().int(),
        /** When the measure ends, in UTC epoch milliseconds. */
        to: schema.number().int(),
    }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: HOUR, maxRows: 100_000 }, retention: RETENTION },
    route: "payer",
    isLocked: true,
});

/** A use of a resource as finance rates it: a usage event of a paying account. */
export type Usage =
    typeof usage extends EventKind<infer Shape, infer Data> ? Event<Shape, Data> : never;

/** A billing period, in UTC epoch milliseconds from its start up to its end. */
export interface Period {
    /** The first instant of the period. */
    readonly start: number;
    /** The first instant after the period. */
    readonly end: number;
}

/** The billing periods usage falls in. */
export const Period = {
    /** Read the calendar month in UTC a time falls in, the period of usage no subscription bills. */
    month(time: number): Period {
        const date = new Date(time);

        return {
            start: Date.UTC(date.getUTCFullYear(), date.getUTCMonth()),
            end: Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1),
        };
    },
};
