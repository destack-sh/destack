import { TimeZone } from "@destack/schema";
import { CronExpressionParser } from "cron-parser";
import { WorkflowError } from "../error/index.ts";
import type { ScheduleTiming } from "../trigger/schedule.ts";

/** A schedule's checked timing: its occurrences listed back from a time, and found after one. */
export class Schedule {
    /** The timing. */
    readonly timing: ScheduleTiming;

    /** Check a timing: its calendar and time zone exist, and its range ends after it starts. */
    constructor(timing: ScheduleTiming) {
        Schedule.require(timing);
        this.timing = timing;
    }

    /** Require a timing's calendar and time zone to exist, and its range to end after it starts. */
    static require(timing: ScheduleTiming): void {
        // refuse a calendar or time zone the cron parser refuses
        if (timing.timing === "cron") {
            try {
                TimeZone.require(timing.timezone);
                CronExpressionParser.parse(timing.cron, { tz: timing.timezone });
            } catch (error) {
                throw WorkflowError.of(
                    "TIMING_INVALID",
                    "schedule timing is invalid: its calendar and time zone must be known",
                    { cause: error },
                );
            }
        }

        // refuse an empty range
        if (
            "endsAt" in timing &&
            timing.endsAt !== undefined &&
            timing.startsAt !== undefined &&
            timing.endsAt <= timing.startsAt
        ) {
            throw WorkflowError.of(
                "TIMING_INVALID",
                "schedule timing is invalid: the schedule must end after its first occurrence",
            );
        }
    }

    /** List the latest occurrences after one time and up to another, at most a limit. */
    recent(
        after: number,
        until: number,
        limit: number,
    ): { readonly due: number[]; readonly earlier: number | "many" } {
        // count fixed intervals within the end
        const { timing } = this;
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
    }

    /** Find the next occurrence after a time, absent once none follows. */
    following(after: number): number | undefined {
        // step to the next interval within the end
        const { timing } = this;
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
    }
}
