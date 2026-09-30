import { defineSchema, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";

/** How a trigger's runs may overlap. */
export const CONCURRENCIES = ["allow", "forbid", "replace"] as const;

/** The shortest interval between a schedule's occurrences: a minute, as cron's own resolution. */
export const MINIMUM_INTERVAL_MILLISECONDS = 60_000;

/** The common schedule fields. */
const SCHEDULE = schema.object({
    /** The package-local schedule name. */
    name: DeclarationName,
    /** Whether occurrences may overlap. */
    concurrency: schema.enum(CONCURRENCIES),
    /** How late an occurrence may start, in milliseconds. */
    deadline: schema.number().int().nonnegative(),
    /** The object method call each occurrence runs, in the installation's space. */
    call: Call,
});

/** A calendar timing. */
const CRON = schema.object({
    /** A calendar schedule. */
    timing: schema.literal("cron"),
    /** A five-field cron expression. */
    cron: schema.string().regex(/^\S+\s+\S+\s+\S+\s+\S+\s+\S+$/),
    /** The IANA time zone. */
    timezone: schema.string().min(1),
    /** The earliest occurrence time, in UTC epoch milliseconds. */
    startsAt: schema.number().int().nonnegative().optional(),
    /** The exclusive end time, in UTC epoch milliseconds. */
    endsAt: schema.number().int().nonnegative().optional(),
});

/** A repeating interval timing. */
const INTERVAL = schema.object({
    /** A repeating interval schedule. */
    timing: schema.literal("interval"),
    /** The interval, in milliseconds, a minute at least. */
    interval: schema.number().int().min(MINIMUM_INTERVAL_MILLISECONDS),
    /** The first occurrence time, in UTC epoch milliseconds. */
    startsAt: schema.number().int().nonnegative(),
    /** The exclusive end time, in UTC epoch milliseconds. */
    endsAt: schema.number().int().nonnegative().optional(),
});

/** A one-off timing. */
const ONCE = schema.object({
    /** A one-off schedule. */
    timing: schema.literal("once"),
    /** The occurrence time, in UTC epoch milliseconds. */
    startsAt: schema.number().int().nonnegative(),
});

/** When a schedule's occurrences fall. */
export const ScheduleTiming = defineSchema(schema.union([CRON, INTERVAL, ONCE]));
/** When a schedule's occurrences fall. */
export type ScheduleTiming = schema.Infer<typeof ScheduleTiming>;

/** A schedule declaration. */
export const ScheduleDescription = defineSchema(
    schema.union([
        SCHEDULE.extend(CRON.shape),
        SCHEDULE.extend(INTERVAL.shape),
        SCHEDULE.extend(ONCE.shape),
    ]),
);
/** A schedule declaration. */
export type ScheduleDescription = schema.Infer<typeof ScheduleDescription>;

/** A declared schedule. */
export type Schedule = Declaration & ScheduleDescription & { readonly kind: "schedule" };

/** Declare a schedule. */
export function defineSchedule(definition: ScheduleDescription, module?: ModuleMetadata): Schedule {
    // stamp the declaring package
    const owner = declaringModule(module, "defineSchedule").package;
    const description = ScheduleDescription.parse(definition);

    return Object.freeze({
        ...description,
        kind: "schedule",
        package: owner,
    });
}
