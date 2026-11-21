import { PlainDate } from "./plain.ts";

/** An event over whole days, its last day included. */
export interface DateSpan {
    /** The event's key. */
    readonly key: string;
    /** The first day. */
    readonly start: PlainDate;
    /** The last day, the first for a one-day event. */
    readonly end: PlainDate;
}

/** An event over a span of time, its end excluded. */
export interface TimeSpan {
    /** The event's key. */
    readonly key: string;
    /** The start, in epoch milliseconds. */
    readonly start: number;
    /** The end, in epoch milliseconds. */
    readonly end: number;
}

/** An event cut to one row of days or one column of time. */
export interface Segment<At> {
    /** The event's key. */
    readonly key: string;
    /** Where the segment starts within its row or column. */
    readonly start: At;
    /** Where the segment ends within its row or column. */
    readonly end: At;
    /** Whether the event starts in this segment. */
    readonly isStart: boolean;
    /** Whether the event ends in this segment. */
    readonly isEnd: boolean;
    /** The level in a row of days, or the column in a column of time. */
    readonly position: number;
    /** The levels of the row, or the columns of the overlapping events. */
    readonly positions: number;
}

/** Events cut to rows of days and columns of time. */
export const Segment = {
    /** Cut events to a row of days and stack them in levels. */
    stack(spans: readonly DateSpan[], days: readonly PlainDate[]): Segment<PlainDate>[] {
        // cut each event to the row
        const first = days[0];
        const last = days.at(-1);
        if (first === undefined || last === undefined) {
            return [];
        }
        const cut = spans
            .filter(
                (span) =>
                    PlainDate.compare(span.end, first) >= 0 &&
                    PlainDate.compare(span.start, last) <= 0,
            )
            .map((span) => ({
                key: span.key,
                start: PlainDate.compare(span.start, first) < 0 ? first : span.start,
                end: PlainDate.compare(span.end, last) > 0 ? last : span.end,
                isStart: PlainDate.compare(span.start, first) >= 0,
                isEnd: PlainDate.compare(span.end, last) <= 0,
            }))
            .toSorted(
                (left, right) =>
                    PlainDate.compare(left.start, right.start) ||
                    PlainDate.daysUntil(right.start, right.end) -
                        PlainDate.daysUntil(left.start, left.end),
            );

        // put each segment on the first free level
        const levels: PlainDate[] = [];
        const placed = cut.map((segment) => {
            // take the first free level
            const free = levels.findIndex((end) => PlainDate.compare(end, segment.start) < 0);
            const position = free === -1 ? levels.length : free;
            levels[position] = segment.end;

            return { ...segment, position };
        });

        return placed.map((segment) => ({ ...segment, positions: levels.length }));
    },

    /** Cut events to a column of time and set them side by side. */
    pack(spans: readonly TimeSpan[], from: number, to: number): Segment<number>[] {
        // cut each event to the column
        const cut = spans
            .filter((span) => span.end > from && span.start < to)
            .map((span) => ({
                key: span.key,
                start: Math.max(span.start, from),
                end: Math.min(span.end, to),
                isStart: span.start >= from,
                isEnd: span.end <= to,
            }))
            .toSorted((left, right) => left.start - right.start || right.end - left.end);

        // group events that overlap
        const groups: (typeof cut)[] = [];
        let reach = Number.NEGATIVE_INFINITY;
        for (const segment of cut) {
            // join the group while the event overlaps it
            const group = groups.at(-1);
            if (group !== undefined && segment.start < reach) {
                group.push(segment);
                reach = Math.max(reach, segment.end);
            }
            // else start a new group
            else {
                groups.push([segment]);
                reach = segment.end;
            }
        }

        // put each event in the first free column
        return groups.flatMap((group) => {
            const columns: number[] = [];
            const placed = group.map((segment) => {
                // take the first free column
                const free = columns.findIndex((end) => end <= segment.start);
                const position = free === -1 ? columns.length : free;
                columns[position] = segment.end;

                return { ...segment, position };
            });

            return placed.map((segment) => ({ ...segment, positions: columns.length }));
        });
    },

    /** Count the segments of a day beyond some levels. */
    hidden(segments: readonly Segment<PlainDate>[], day: PlainDate, levels: number): number {
        return segments.filter(
            (segment) =>
                segment.position >= levels &&
                PlainDate.compare(segment.start, day) <= 0 &&
                PlainDate.compare(segment.end, day) >= 0,
        ).length;
    },
};
