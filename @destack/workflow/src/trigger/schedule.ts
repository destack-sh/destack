import { defineSchema, Instant, schema, TimeZone } from "@destack/schema";
import { CronExpressionParser } from "cron-parser";
import { Failure } from "@destack/sync";
import { CallError } from "@destack/runtime/error";

/** How a schedule trigger's runs may overlap, as a Kubernetes CronJob's concurrency policy. */
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

/** A schedule a trigger fires on. */
export const ScheduleOn = defineSchema(
    schema.object({
        /** When the occurrences fall. */
        timing: ScheduleTiming,
        /** Whether occurrences may overlap. */
        concurrency: schema.enum(CONCURRENCIES),
        /** How late an occurrence may start, in milliseconds. */
        deadline: schema.number().int().nonnegative(),
    }),
);
/** A schedule a trigger fires on. */
export type ScheduleOn = schema.Infer<typeof ScheduleOn>;

/** Why a schedule skipped an occurrence: missed, late, superseded, or overlapping a run. */
export const SCHEDULE_SKIP_REASONS = ["missed", "late", "superseded", "overlap"] as const;

/** A skipped occurrence of a schedule and why. */
export const ScheduleSkip = defineSchema(
    schema.object({
        /** The occurrence's time, in UTC epoch milliseconds. */
        at: Instant,
        /** Why it was skipped. */
        reason: schema.enum(SCHEDULE_SKIP_REASONS),
        /** The occurrences missed before it uncounted, absent when none. */
        earlier: schema
            .union([schema.number().int().positive(), schema.literal("many")])
            .exactOptional(),
    }),
);
/** A skipped occurrence of a schedule and why. */
export type ScheduleSkip = schema.Infer<typeof ScheduleSkip>;

/** The occurrences of schedule timings: checked, listed back from a time, and found after one. */
export const Schedule = {
    /** Require a timing's calendar and time zone to exist, and its range to end after it starts. */
    require(timing: ScheduleTiming): ScheduleTiming {
        // refuse a calendar or time zone the cron parser refuses
        if (timing.timing === "cron") {
            try {
                TimeZone.require(timing.timezone);
                CronExpressionParser.parse(timing.cron, { tz: timing.timezone });
            } catch (error) {
                throw new CallError("BAD_REQUEST", {
                    message: `invalid schedule timing: ${Failure.of(error).message}`,
                });
            }
        }

        // refuse an empty range
        if (
            "endsAt" in timing &&
            timing.endsAt !== undefined &&
            timing.startsAt !== undefined &&
            timing.endsAt <= timing.startsAt
        ) {
            throw new CallError("BAD_REQUEST", {
                message: "invalid schedule timing: the schedule ends before its first occurrence",
            });
        }

        return timing;
    },

    /** List the latest occurrences after one time and up to another, at most a limit. */
    recent(
        timing: ScheduleTiming,
        after: number,
        until: number,
        limit: number,
    ): { readonly due: number[]; readonly earlier: number | "many" } {
        // count fixed intervals within the end
        if (timing.timing === "interval") {
            const end = timing.endsAt === undefined ? until : Math.min(until, timing.endsAt - 1);
            const first = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
            const last = Math.floor((end - timing.startsAt) / timing.interval);
            const retained = Math.max(first, last - limit + 1);
            const due = Array.from(
                { length: Math.max(0, last - retained + 1) },
                (_, index) => timing.startsAt + (retained + index) * timing.interval,
            );

            return { due, earlier: Math.max(0, retained - first) };
        }
        // occur once
        else if (timing.timing === "once") {
            const isDue = timing.startsAt > after && timing.startsAt <= until;

            return { due: isDue ? [timing.startsAt] : [], earlier: 0 };
        }

        // walk the calendar back from the end within its bounds
        const end = timing.endsAt === undefined ? until : Math.min(until, timing.endsAt - 1);
        const start = timing.startsAt === undefined ? after : Math.max(after, timing.startsAt - 1);
        const calendar = CronExpressionParser.parse(timing.cron, {
            currentDate: end + 1,
            tz: timing.timezone,
        });
        const walked: number[] = [];
        while (walked.length <= limit) {
            const previous = calendar.prev().getTime();
            if (previous <= start) {
                break;
            }
            walked.push(previous);
        }
        const isMore = walked.length > limit;

        return { due: walked.slice(0, limit).toReversed(), earlier: isMore ? "many" : 0 };
    },

    /** Find the next occurrence after a time, absent once none follows. */
    following(timing: ScheduleTiming, after: number): number | undefined {
        // step to the next interval within the end
        if (timing.timing === "interval") {
            const index = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
            const next = timing.startsAt + index * timing.interval;

            return timing.endsAt !== undefined && next >= timing.endsAt ? undefined : next;
        }
        // occur once
        else if (timing.timing === "once") {
            return timing.startsAt > after ? timing.startsAt : undefined;
        }

        // follow the calendar within its bounds
        const start = timing.startsAt === undefined ? after : Math.max(after, timing.startsAt - 1);
        const next = CronExpressionParser.parse(timing.cron, {
            currentDate: start,
            tz: timing.timezone,
        })
            .next()
            .getTime();

        return timing.endsAt !== undefined && next >= timing.endsAt ? undefined : next;
    },
};
