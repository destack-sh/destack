import { through, union } from "@destack/access";
import { account, machine, space } from "@destack/account/object";
import { defineObject, field, installation } from "@destack/object";
import { schema } from "@destack/schema";

/** When usage flushes into a segment: hourly, or at 100,000 uses. */
const USAGE_FLUSH = { maxAge: { hours: 1 }, maxRows: 100_000 } as const;

/** How long the payer's home stores usage: ten years, as long as the invoices billing it. */
const USAGE_RETENTION = { days: 3650 } as const;

/** What a use has beside its keys. */
export const UsageData = schema.object({
    /** When the measure starts, in UTC epoch milliseconds. */
    from: schema.number().int(),
    /** When the measure ends, in UTC epoch milliseconds. */
    to: schema.number().int(),
});

/** A metered use of a resource, stored in the home of the account paying for it. */
export const usage = defineObject(
    {
        name: "usage",
        plural: "usages",
        scope: account,
        storage: {
            kind: "events",
            flush: USAGE_FLUSH,
            retention: USAGE_RETENTION,
            delivery: "exactly-once",
            isLocked: true,
        },
        fields: {
            /** The measure's unit and interval. */
            data: field.json(UsageData),
            /** The meter measuring the use, by its package and name. */
            meter: field.string(),
            /** The SKU rating the use on the machine it ran on, absent for a seller's own meter. */
            sku: field.string().optional(),
            /** The space the use ran in, absent for a dedicated machine's own hours. */
            space: field.reference(space).optional(),
            /** The dedicated machine whose hours were used, absent for a space's use. */
            machine: field.reference(machine).optional(),
            /** The installation that used it, absent for the space itself. */
            installation: field.reference(installation).optional(),
            /** The amount used in the meter's unit, or the share of a month a level was held. */
            quantity: field.number(),
            /** The level held at the end of the measure, for meters averaged over time. */
            level: field.number().optional(),
        },
        permissions: {
            // let the payer's readers and its residency's servers read every use, a space's readers its own
            read: union(
                through("scope", "read"),
                through("scope", "serve"),
                through("space", "read"),
            ),
            // let the paying account's servers append what they measured: its residency's machines and its own
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append measured uses. */
        append: method.createMany("append", {
            fields: [
                "meter",
                "sku",
                "spaceId",
                "machineId",
                "installationId",
                "quantity",
                "level",
                "data",
            ],
        }),
    }),
);
