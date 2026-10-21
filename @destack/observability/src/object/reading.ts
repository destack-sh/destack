import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { EVENT_FLUSH } from "../event/policy.ts";
import { telemetryRetention } from "../setting/index.ts";
import { REQUEST_LIFETIME_MILLISECONDS } from "@destack/sync";
import { installation } from "@destack/stack/object";

/** What a reading keeps beside its keys: the level an averaged meter held when read. */
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
            retention: { setting: telemetryRetention.definition },
            delivery: "exactly-once",
        },
        // bill each space's readings from the log's start an hour at a time, held as long as a billing call may retry
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
            /** The level an averaged meter held when read. */
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
            isInternal: true,
            fields: ["meter", "installationId", "sku", "payerId", "value", "data"],
        }),
    }),
);
