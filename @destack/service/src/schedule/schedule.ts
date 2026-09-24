import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";

/** Fields shared by calendar, interval, and one-off schedules. */
const SCHEDULE = schema.object({
    /** The package-local schedule name. */
    name: DeclarationName,
    /** The declaration format version. */
    version: schema.literal(1),
    /** Whether occurrences may overlap. */
    concurrency: schema.enum(["allow", "forbid", "replace"]),
    /** How late an occurrence may start, in milliseconds. */
    deadline: schema.number().int().nonnegative(),
});

/** A controller-managed schedule, as the manifest describes it. */
export const ScheduleDescription = defineSchema(
    schema.union([
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
            /** Run the schedule once at the selected time. */
            timing: schema.literal("once"),
            /** The occurrence time in UTC epoch milliseconds. */
            startsAt: schema.number().int().nonnegative(),
        }),
    ]),
);
/** A controller-managed schedule, as the manifest describes it. */
export type ScheduleDescription = schema.Infer<typeof ScheduleDescription>;

/** A schedule as authored, before the declaration format version is added. */
export type ScheduleDefinition = WithoutVersion<ScheduleDescription>;

/** Remove the version from each timing variant. */
type WithoutVersion<Description> = Description extends unknown
    ? Omit<Description, "version">
    : never;

/** A declared schedule, run by the workload that implements it. */
export type Schedule = Declaration & ScheduleDescription;

/** A workload's handler for one declared schedule. */
export interface ScheduleImplementation {
    /** The declared schedule this handler runs. */
    readonly schedule: Schedule;
    /** Run one occurrence, observing cancellation. */
    run(signal: AbortSignal): Promise<void>;
}

/** Declare a controller-managed schedule. */
export function defineSchedule(definition: ScheduleDefinition, module?: ModuleMetadata): Schedule {
    // stamp the declaring package supplied by the module transform
    const owner = declaringModule(module, "defineSchedule").package;
    const description = ScheduleDescription.parse({ ...definition, version: 1 });

    return Object.freeze({ ...description, package: owner });
}
