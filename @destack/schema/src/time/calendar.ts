import { nth } from "../collection/index.ts";
import { PlainDate, PlainTime, type Weekday } from "./plain.ts";
import type { TimeScale } from "./scale.ts";
import type { ScheduleEdge } from "./schedule.ts";
import { TimeZone } from "./zone.ts";

/** The milliseconds of a block of days a calendar lists its non-working days in: four weeks. */
const CALENDAR_BLOCK = 28 * 86_400_000;

/** The most blocks a calendar keeps listed, about forty years in some 250 KB at 500 bytes each. */
const CALENDAR_BLOCKS = 512;

/** The minutes of a day, which "24:00" ends at. */
const DAY_MINUTES = 24 * 60;

/** The weekdays from Monday to Sunday, as ISO numbers them. */
const WEEKDAYS: readonly Weekday[] = [1, 2, 3, 4, 5, 6, 7];

/** A span of a day's working time, after FullCalendar's business hours. */
export interface WorkHours {
    /** The time it starts. */
    readonly start: PlainTime;
    /** The time it ends, after its start, 24:00 for the day's end. */
    readonly end: PlainTime;
}

/** The working hours of each day of a week, in order, a day it lacks working not at all. */
export type WorkWeek = Readonly<Partial<Record<Weekday, readonly WorkHours[]>>>;

/** The verbs of working weeks. */
export const WorkWeek = {
    /** Read the week whose days work whole but its weekend days, every day working with none. */
    of(weekend: readonly Weekday[]): WorkWeek {
        const whole: readonly WorkHours[] = [{ start: "00:00", end: "24:00" }];

        return Object.fromEntries(
            WEEKDAYS.filter((day) => !weekend.includes(day)).map((day) => [day, whole]),
        );
    },

    /** Report whether a weekday has working hours. */
    isWorkingDay(week: WorkWeek, day: Weekday): boolean {
        return (week[day] ?? []).length > 0;
    },
};

/** The working time of a zone by a working week, as lags, lengths and tracks count it. */
export class WorkCalendar {
    /** The zone the days start and end in. */
    readonly timeZone: TimeZone;
    /** The working hours of each weekday. */
    readonly week: WorkWeek;
    /** The blocks listed most recently, by their index from the epoch, the least recent first. */
    readonly #blocks = new Map<number, CalendarBlock>();

    /** Count working time in a zone by a week, refusing hours out of order or no work at all. */
    constructor(timeZone: TimeZone, week: WorkWeek) {
        // refuse each day's hours that run backward or overlap
        for (const day of WEEKDAYS) {
            const hours = week[day] ?? [];
            const isOrdered = hours.every(
                (each, index) =>
                    minutesOf(each.start) < minutesOf(each.end) &&
                    (index === 0 || minutesOf(nth(hours, index - 1).end) <= minutesOf(each.start)),
            );
            if (!isOrdered) {
                throw new RangeError(
                    `work calendar is invalid: the hours of day '${String(day)}' overlap or run backward`,
                );
            }
        }

        // refuse a week without working time
        if (!WEEKDAYS.some((day) => WorkWeek.isWorkingDay(week, day))) {
            throw new RangeError("work calendar is invalid: its week has no working day");
        }
        this.timeZone = timeZone;
        this.week = week;
    }

    /** Report whether an instant falls in working time: within a working day's hours. */
    isWorking(at: number): boolean {
        return this.#spanAt(at, "start") === undefined;
    }

    /** Move an instant by working time: forward to an end, back to a start. */
    add(at: number, duration: number): number {
        // leave an instant where it is for no duration
        if (duration === 0) {
            return at;
        }

        // spend the duration block by block, then find its instant inside the last block
        let block = this.#block(at);
        let target = block.workingAt(at) + duration;
        while (target > block.working) {
            target -= block.working;
            block = this.#block(block.end);
        }
        while (target < 0) {
            block = this.#block(block.start - 1);
            target += block.working;
        }

        return block.instant(target, duration < 0 ? "start" : "end");
    }

    /** Read the working time between two instants, negative backward. */
    between(start: number, end: number): number {
        // add the working time of each block the span crosses
        const [from, to] = start <= end ? [start, end] : [end, start];
        let block = this.#block(from);
        let total = -block.workingAt(from);
        while (block.end <= to) {
            total += block.working;
            block = this.#block(block.end);
        }
        total += block.workingAt(to);

        return start <= end ? total : -total;
    }

    /** Move an instant off non-working days: a start forward, an end back. */
    align(at: number, edge: ScheduleEdge): number {
        // step past each span of non-working days the instant falls in, across a block's edge too
        let aligned = at;
        for (
            let span = this.#spanAt(aligned, edge);
            span !== undefined;
            span = this.#spanAt(aligned, edge)
        ) {
            aligned = edge === "start" ? span.end : span.start;
        }

        return aligned;
    }

    /** List the non-working days a span touches, joined and cut to it. */
    nonWorking(range: TimeScale): TimeScale[] {
        // gather each block's spans, joining those that meet at a block's edge
        const spans: TimeScale[] = [];
        for (
            let block = this.#block(range.start);
            block.start < range.end;
            block = this.#block(block.end)
        ) {
            for (const span of block.spans) {
                const start = Math.max(span.start, range.start);
                const end = Math.min(span.end, range.end);
                const previous = spans.at(-1);
                if (previous !== undefined && previous.end === start && end > start) {
                    spans[spans.length - 1] = { start: previous.start, end };
                } else if (end > start) {
                    spans.push({ start, end });
                }
            }
        }

        return spans;
    }

    /** Find the non-working span an edge falls in, from either side by the edge. */
    #spanAt(at: number, edge: ScheduleEdge): TimeScale | undefined {
        // look in the block of the instant, or of the millisecond before it for an end
        return edge === "start"
            ? this.#block(at).spanAt(at, edge)
            : this.#block(Math.ceil(at) - 1).spanAt(at, edge);
    }

    /** Read the block an instant falls in, listing its non-working days the first time. */
    #block(at: number): CalendarBlock {
        // reuse a listed block as the most recent, or list it
        const index = Math.floor(at / CALENDAR_BLOCK);
        const block = this.#blocks.get(index) ?? new CalendarBlock(index * CALENDAR_BLOCK, this);
        this.#blocks.delete(index);
        this.#blocks.set(index, block);

        // forget the least recent block past the limit
        const [oldest] = this.#blocks.keys();
        if (this.#blocks.size > CALENDAR_BLOCKS && oldest !== undefined) {
            this.#blocks.delete(oldest);
        }

        return block;
    }
}

/** A block of a calendar's time: its non-working spans and its working time. */
class CalendarBlock {
    /** The block's first instant. */
    readonly start: number;
    /** The instant after the block. */
    readonly end: number;
    /** The non-working days in the block, joined into spans in order. */
    readonly spans: readonly TimeScale[];
    /** The non-working time in the block before each span. */
    readonly before: readonly number[];
    /** The working time in the whole block. */
    readonly working: number;

    /** List the non-working time of a calendar from a block's first instant. */
    constructor(start: number, calendar: WorkCalendar) {
        // step through the block's days, reading the zone only for days with time off
        const { timeZone, week } = calendar;
        const end = start + CALENDAR_BLOCK;
        const last = TimeZone.date(timeZone, end - 1);
        const spans: TimeScale[] = [];
        for (
            let day = TimeZone.date(timeZone, start);
            day <= last;
            day = PlainDate.add(day, { days: 1 })
        ) {
            for (const gap of gapsOf(day, week[PlainDate.dayOfWeek(day)] ?? [], timeZone)) {
                // cut the gap to the block, then join it to the span before it or open a span
                const from = Math.max(gap.start, start);
                const to = Math.min(gap.end, end);
                const previous = spans.at(-1);
                if (previous?.end === from && to > from) {
                    spans[spans.length - 1] = { start: previous.start, end: to };
                } else if (to > from) {
                    spans.push({ start: from, end: to });
                }
            }
        }

        // sum the non-working time before each span, and the working time left
        const before: number[] = [];
        let hidden = 0;
        for (const span of spans) {
            before.push(hidden);
            hidden += span.end - span.start;
        }
        this.start = start;
        this.end = end;
        this.spans = spans;
        this.before = before;
        this.working = CALENDAR_BLOCK - hidden;
    }

    /** Read the working time from the block's start to an instant inside it. */
    workingAt(at: number): number {
        // leave out the non-working time before the instant, inside a span too
        const index = this.spans.findLastIndex((span) => span.start <= at);
        const span = this.spans[index];
        const off =
            span === undefined ? 0 : nth(this.before, index) + Math.min(at, span.end) - span.start;

        return at - this.start - off;
    }

    /** Read the instant at a working time from the block's start, as an end or a start. */
    instant(working: number, edge: ScheduleEdge): number {
        // find the last span that starts before the working time, or at it for a start
        const index = this.spans.findLastIndex((span, each) => {
            const position = span.start - this.start - nth(this.before, each);

            return edge === "start" ? position <= working : position < working;
        });
        const span = this.spans[index];

        // add back the non-working time up to the end of that span
        return span === undefined
            ? this.start + working
            : this.start + working + nth(this.before, index) + span.end - span.start;
    }

    /** Find the span an edge at an instant falls in, from either side by the edge. */
    spanAt(at: number, edge: ScheduleEdge): TimeScale | undefined {
        return this.spans.find((span) =>
            edge === "start"
                ? span.start <= at && at < span.end
                : span.start < at && at <= span.end,
        );
    }
}

/** Read the minutes after midnight of a working time, "24:00" a whole day's. */
function minutesOf(time: PlainTime): number {
    return time === "24:00" ? DAY_MINUTES : PlainTime.minutes(time);
}

/** List the time off a day has around its working hours, none for a day working whole. */
function gapsOf(day: PlainDate, hours: readonly WorkHours[], timeZone: TimeZone): TimeScale[] {
    // skip a day working from midnight to midnight
    const [first] = hours;
    if (hours.length === 1 && first?.start === "00:00" && first.end === "24:00") {
        return [];
    }

    // read each boundary in the zone, then pair the time off between the working hours
    const next = PlainDate.add(day, { days: 1 });
    const instantOf = (time: PlainTime) =>
        time === "24:00"
            ? TimeZone.instant(timeZone, next, "00:00")
            : TimeZone.instant(timeZone, day, time);
    const edges = [
        TimeZone.instant(timeZone, day, "00:00"),
        ...hours.flatMap((each) => [instantOf(each.start), instantOf(each.end)]),
        TimeZone.instant(timeZone, next, "00:00"),
    ];
    const gaps: TimeScale[] = [];
    for (let index = 0; index < edges.length; index += 2) {
        const gap = { start: nth(edges, index), end: nth(edges, index + 1) };
        if (gap.end > gap.start) {
            gaps.push(gap);
        }
    }

    return gaps;
}
