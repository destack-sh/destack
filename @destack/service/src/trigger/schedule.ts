import { defineSchema, Instant, schema, TimeZone } from "@destack/schema";
import { Call } from "@destack/sync";

/** How a schedule trigger's runs may overlap. */
export const CONCURRENCIES = ["allow", "forbid", "replace"] as const;

/** The shortest interval between a schedule's occurrences: a minute, as cron's own resolution. */
export const MINIMUM_INTERVAL_MILLISECONDS = 60_000;

/** A calendar timing. */
const CRON = schema.object({
    /** A calendar schedule. */
    timing: schema.literal("cron"),
    /** A five-field cron expression. */
    cron: schema.string().regex(/^\S+\s+\S+\s+\S+\s+\S+\s+\S+$/u),
    /** The IANA time zone. */
    timezone: TimeZone,
    /** The earliest occurrence time, in UTC epoch milliseconds. */
    startsAt: Instant.exactOptional(),
    /** The exclusive end time, in UTC epoch milliseconds. */
    endsAt: Instant.exactOptional(),
});

/** A repeating interval timing. */
const INTERVAL = schema.object({
    /** A repeating interval schedule. */
    timing: schema.literal("interval"),
    /** The interval, in milliseconds, a minute at least. */
    interval: schema.number().int().min(MINIMUM_INTERVAL_MILLISECONDS),
    /** The first occurrence time, in UTC epoch milliseconds. */
    startsAt: Instant,
    /** The exclusive end time, in UTC epoch milliseconds. */
    endsAt: Instant.exactOptional(),
});

/** A one-off timing. */
const ONCE = schema.object({
    /** A one-off schedule. */
    timing: schema.literal("once"),
    /** The occurrence time, in UTC epoch milliseconds. */
    startsAt: Instant,
});

/** When a schedule's occurrences fall. */
export const ScheduleTiming = defineSchema(schema.union([CRON, INTERVAL, ONCE]));
/** When a schedule's occurrences fall. */
export type ScheduleTiming = schema.Infer<typeof ScheduleTiming>;

/** A schedule a trigger fires on: when its occurrences fall, how they overlap, and how late they may start. */
export const ScheduleOn = defineSchema(
    schema.object({
        /** When the occurrences fall. */
        timing: ScheduleTiming,
        /** Whether occurrences may overlap. */
        concurrency: schema.enum(CONCURRENCIES),
        /** How late an occurrence may start, in milliseconds. */
        deadline: schema.number().int().nonnegative(),
        /** The call each occurrence runs, in the installation's space. */
        call: Call,
    }),
);
/** A schedule a trigger fires on. */
export type ScheduleOn = schema.Infer<typeof ScheduleOn>;
