import { TimeZone } from "@destack/schema";
import { CronExpressionParser } from "cron-parser";
import type { ScheduleTiming } from "../trigger/index.ts";

/** The occurrences of schedule timings: checked, listed back from a time, and found after one. */
export const Schedule = {
    /** Require a timing's calendar and time zone to exist, and its range to end after it starts. */
    require(timing: ScheduleTiming): ScheduleTiming {
        // check the calendar
        if (timing.timing === "cron") {
            TimeZone.require(timing.timezone);
            CronExpressionParser.parse(timing.cron, { tz: timing.timezone });
        }

        // require a nonempty range
        if (
            "endsAt" in timing &&
            timing.endsAt !== undefined &&
            timing.startsAt !== undefined &&
            timing.endsAt <= timing.startsAt
        ) {
            throw new RangeError("the schedule ends before its first occurrence");
        }

        return timing;
    },

    /** List the latest occurrences after one time and up to another, at most a limit, and how many earlier ones fell in between. */
    recent(
        timing: ScheduleTiming,
        after: number,
        until: number,
        limit: number,
    ): { readonly due: number[]; readonly earlier: number | "many" } {
        // count fixed intervals arithmetically, within the end
        if (timing.timing === "interval") {
            const end = timing.endsAt === undefined ? until : Math.min(until, timing.endsAt - 1);
            const first = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
            const last = Math.floor((end - timing.startsAt) / timing.interval);
            const kept = Math.max(first, last - limit + 1);
            const due = Array.from(
                { length: Math.max(0, last - kept + 1) },
                (_, index) => timing.startsAt + (kept + index) * timing.interval,
            );

            return { due, earlier: Math.max(0, kept - first) };
        }
        // occur once
        else if (timing.timing === "once") {
            const isDue = timing.startsAt > after && timing.startsAt <= until;

            return { due: isDue ? [timing.startsAt] : [], earlier: 0 };
        }

        // walk the calendar back from the end within its bounds, one past the limit to learn whether more were missed
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
        // step to the next interval, within the end
        if (timing.timing === "interval") {
            const index = Math.max(0, Math.floor((after - timing.startsAt) / timing.interval) + 1);
            const next = timing.startsAt + index * timing.interval;

            return timing.endsAt !== undefined && next >= timing.endsAt ? undefined : next;
        }
        // occur once
        else if (timing.timing === "once") {
            return timing.startsAt > after ? timing.startsAt : undefined;
        }

        // follow the calendar, within its bounds
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
