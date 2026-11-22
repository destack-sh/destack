import { PlainDate } from "./plain.ts";
import { expect, test } from "@destack/test";
import { Segment } from "./segment.ts";

/** The days of the week of 16 November 2026, Monday first. */
const WEEK = Array.from({ length: 7 }, (_, index) => PlainDate.add("2026-11-16", { days: index }));

/** The epoch milliseconds of a time on 19 November 2026 in UTC. */
function at(time: string): number {
    return Date.parse(`2026-11-19T${time}:00Z`);
}

test("stack a week's events in levels, longer first, cut at the week's ends, and count the hidden ones", () => {
    // a conference over the week's end, a launch, two one-day events and a trip from the week before
    const spans = [
        { key: "launch", start: "2026-11-19", end: "2026-11-19" },
        {
            key: "conference",
            start: "2026-11-19",
            end: "2026-11-24",
        },
        { key: "trip", start: "2026-11-12", end: "2026-11-17" },
        { key: "review", start: "2026-11-18", end: "2026-11-19" },
        { key: "retro", start: "2026-11-20", end: "2026-11-20" },
        { key: "later", start: "2026-11-30", end: "2026-11-30" },
    ];
    const segments = Segment.stack(spans, WEEK);

    expect({
        segments: segments.map((segment) => [
            segment.key,
            segment.start,
            segment.end,
            segment.isStart,
            segment.isEnd,
            segment.position,
        ]),
        positions: segments[0]?.positions,
        hidden: WEEK.map((day) => Segment.hidden(segments, day, 2)),
    }).toEqual({
        segments: [
            ["trip", "2026-11-16", "2026-11-17", false, true, 0],
            ["review", "2026-11-18", "2026-11-19", true, true, 0],
            ["conference", "2026-11-19", "2026-11-22", true, false, 1],
            ["launch", "2026-11-19", "2026-11-19", true, true, 2],
            ["retro", "2026-11-20", "2026-11-20", true, true, 0],
        ],
        positions: 3,
        hidden: [0, 0, 0, 1, 0, 0, 0],
    });
});

test("set overlapping events of a day side by side, and keep events that only touch in one column", () => {
    // a standup, two meetings overlapping it in a chain, a lunch after, and a flight from the night before
    const spans = [
        { key: "standup", start: at("09:00"), end: at("09:30") },
        { key: "design", start: at("09:15"), end: at("10:00") },
        { key: "hiring", start: at("09:45"), end: at("10:30") },
        { key: "lunch", start: at("12:00"), end: at("13:00") },
        { key: "lunch walk", start: at("13:00"), end: at("13:30") },
        { key: "flight", start: at("00:00") - 3_600_000, end: at("02:00") },
    ];
    const segments = Segment.pack(spans, at("00:00"), at("00:00") + 86_400_000);

    expect(
        segments.map((segment) => [
            segment.key,
            segment.isStart,
            segment.position,
            segment.positions,
        ]),
    ).toEqual([
        ["flight", false, 0, 1],
        ["standup", true, 0, 2],
        ["design", true, 1, 2],
        ["hiring", true, 0, 2],
        ["lunch", true, 0, 1],
        ["lunch walk", true, 0, 1],
    ]);
});

test("place a segment in its row's columns, refusing one that ends outside the row", () => {
    // a Tuesday-to-Thursday segment in a week from Monday, and one past the week
    const days = ["2026-11-02", "2026-11-03", "2026-11-04", "2026-11-05", "2026-11-06"] as const;
    const segment = {
        key: "trip",
        start: "2026-11-03",
        end: "2026-11-05",
        isStart: true,
        isEnd: true,
        position: 0,
        positions: 1,
    };
    const refusal = (() => {
        try {
            return Segment.columns({ ...segment, end: "2026-11-09" }, days);
        } catch (error) {
            return error;
        }
    })();

    expect([Segment.columns(segment, days), refusal]).toEqual([
        { start: 1, span: 3 },
        new RangeError("segment 'trip' is invalid: it ends outside its row"),
    ]);
});
