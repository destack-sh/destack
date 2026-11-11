import { through } from "@destack/access";
import { account, space } from "@destack/account/object";
import { defineObject, field, installation } from "@destack/object";
import { schema } from "@destack/schema";
import { argument, plural, t } from "@destack/locale";

/** When readings flush into a segment: hourly, or at 100,000 readings. */
const READING_FLUSH = { maxAge: { hours: 1 }, maxRows: 100_000 } as const;

/** How long a space keeps its readings: a yearly period plus Stripe's 35 days for late meters. */
const READING_RETENTION = { days: 365 + 35 } as const;

/** What a reading keeps beside its keys: the level of an averaged meter when read. */
export const ReadingData = schema.object({
    /** The level, absent for a counter. */
    level: schema.number().exactOptional(),
});
/** What a reading keeps beside its keys. */
export type ReadingData = schema.Infer<typeof ReadingData>;

/** One increment of a meter a space used, with its SKU and the account whose payer it bills. */
export const reading = defineObject(
    {
        name: "reading",
        plural: "readings",
        title: t`${plural(argument("count"), { one: "Reading", other: "Readings" })}`,
        scope: space,
        storage: {
            kind: "events",
            flush: READING_FLUSH,
            retention: READING_RETENTION,
            delivery: "exactly-once",
        },
        fields: {
            /** The meter the reading increments, by its declaration's key. */
            meter: field.string(),
            /** The installation that used it, absent for the space itself. */
            installation: field.reference(installation).optional(),
            /** The SKU rating the use where it was read, by its declaration's key. */
            sku: field.string(),
            /** The account the space belongs to, whose payer the use is billed to. */
            account: field.reference(account),
            /** The increment in the meter's unit: a delta, or a level times milliseconds. */
            value: field.number(),
            /** The level of an averaged meter when read. */
            data: field.json(ReadingData),
        },
        access: {
            permissions: {
                // let the space's observers read its readings, and the machine serving it append them
                read: through("scope", "observe"),
                append: through("scope", "serve"),
            },
        },
    },
    (method) => ({
        /** Append the readings a machine measured. */
        append: method.createMany("append", {
            visibility: "package",
            fields: ["meter", "installationId", "sku", "accountId", "value", "data"],
        }),
    }),
);
