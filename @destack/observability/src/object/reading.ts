import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { defineObject, field, installation } from "@destack/object";
import { schema } from "@destack/schema";
import { REQUEST_LIFETIME_MILLISECONDS } from "@destack/sync";
import { EVENT_FLUSH } from "../event/policy.ts";

/** How long a space keeps its readings: a yearly period plus Stripe's 35 days for late meters. */
const READING_RETENTION = { days: 365 + 35 } as const;

/** What a reading keeps beside its keys: the level of an averaged meter when read. */
export const ReadingData = schema.object({
    /** The level, absent for a counter. */
    level: schema.number().exactOptional(),
});
/** What a reading keeps beside its keys. */
export type ReadingData = schema.Infer<typeof ReadingData>;

/** One increment of a meter a space used, with the SKU and payer it is billed at. */
export const reading = defineObject(
    {
        name: "reading",
        plural: "readings",
        scope: space,
        storage: {
            kind: "events",
            flush: EVENT_FLUSH,
            retention: READING_RETENTION,
            delivery: "exactly-once",
        },
        // bill each space's readings an hour at a time, kept while a billing call may retry
        readers: {
            metering: {
                by: "scope",
                start: "earliest",
                hold: { milliseconds: REQUEST_LIFETIME_MILLISECONDS },
                batch: { hours: 1 },
            },
        },
        fields: {
            /** The meter the reading increments, by its declaration's key. */
            meter: field.string(),
            /** The installation that used it, absent for the space itself. */
            installation: field.reference(installation).optional(),
            /** The SKU rating the use where it was read, by its declaration's key. */
            sku: field.string(),
            /** The account paying for the space when it was read. */
            payer: field.reference(account),
            /** The increment in the meter's unit: a delta, or a level times milliseconds. */
            value: field.number(),
            /** The level of an averaged meter when read. */
            data: field.json(ReadingData),
        },
        permissions: {
            // let the space's observers read its readings, and the machine serving it append them
            read: through("scope", "observe"),
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append the readings a machine measured. */
        append: method.createMany("append", {
            visibility: "package",
            fields: ["meter", "installationId", "sku", "payerId", "value", "data"],
        }),
    }),
);
