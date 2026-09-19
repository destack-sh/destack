import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";

/** Fields shared by calendar, interval, and one-off schedules. */
const SCHEDULE = schema.object({
    /** The package-local schedule name. */
    name: ResourceName,
    /** The declaration format version. */
    version: schema.literal(1),
    /** The exported handler receiving the scheduled occurrence. */
    handler: schema.string().min(1),
    /** Whether occurrences may overlap. */
    concurrency: schema.enum(["allow", "forbid", "replace"]),
    /** How late an occurrence may start, in milliseconds. */
    deadline: schema.number().int().nonnegative(),
});

/** A controller-managed schedule invoking an exported workload handler. */
export const ScheduleDeclaration = defineSchema(schema.union([
    SCHEDULE.extend({
        /** Evaluate calendar occurrences in the selected time zone. */
        timing: schema.literal("cron"),
        /** A five-field cron expression. */
        cron: schema.string().regex(/^\S+\s+\S+\s+\S+\s+\S+\s+\S+$/),
        /** The IANA time zone used to evaluate occurrences. */
        timezone: schema.string().min(1),
        /** The earliest occurrence time in UTC epoch milliseconds. */
        startsAt: schema.number().int().nonnegative().optional(),
        /** The exclusive end time in UTC epoch milliseconds. */
        endsAt: schema.number().int().nonnegative().optional(),
    }),
    SCHEDULE.extend({
        /** Repeat at a fixed interval from the first occurrence. */
        timing: schema.literal("interval"),
        /** The interval in milliseconds. */
        interval: schema.number().int().positive(),
        /** The first occurrence time in UTC epoch milliseconds. */
        startsAt: schema.number().int().nonnegative(),
        /** The exclusive end time in UTC epoch milliseconds. */
        endsAt: schema.number().int().nonnegative().optional(),
    }),
    SCHEDULE.extend({
        /** Invoke the handler once at the selected time. */
        timing: schema.literal("once"),
        /** The occurrence time in UTC epoch milliseconds. */
        startsAt: schema.number().int().nonnegative(),
    }),
]));
/** A controller-managed schedule invoking an exported workload handler. */
export type ScheduleDeclaration = schema.Infer<typeof ScheduleDeclaration>;

/** Declare a schedule for build-time validation. */
export function defineSchedule(value: ScheduleDeclaration): ScheduleDeclaration {
    return ScheduleDeclaration.parse(value);
}
