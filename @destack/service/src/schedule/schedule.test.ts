import { expect, test } from "@destack/test";
import type { ScheduleTiming } from "../trigger/index.ts";
import { Schedule } from "./schedule.ts";

/** Nine in the morning in Berlin, which moves from 08:00 to 07:00 UTC when summer time starts on 2026-03-29. */
const MORNING: ScheduleTiming = { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Berlin" };

test("follow a calendar in its time zone across a summer time change", () => {
    // step from the day before the change to the morning of the change
    const after = Date.parse("2026-03-28T12:00:00Z");
    expect(Schedule.following(MORNING, after)).toBe(Date.parse("2026-03-29T07:00:00Z"));

    // keep the two latest mornings and mark the earlier ones as more than the limit
    const recent = Schedule.recent(
        MORNING,
        Date.parse("2026-03-26T12:00:00Z"),
        Date.parse("2026-03-31T00:00:00Z"),
        2,
    );
    expect(recent).toEqual({
        due: [Date.parse("2026-03-29T07:00:00Z"), Date.parse("2026-03-30T07:00:00Z")],
        earlier: "many",
    });

    // end the calendar before an occurrence at its end
    const ended = { ...MORNING, endsAt: Date.parse("2026-03-29T07:00:00Z") };
    expect(Schedule.following(ended, after)).toBeUndefined();
});

test("count intervals and one-off occurrences arithmetically", () => {
    // keep the two latest minutes and count the three before them
    const minutely: ScheduleTiming = { timing: "interval", interval: 60_000, startsAt: 0 };
    expect([
        Schedule.recent(minutely, 0, 300_000, 2),
        Schedule.following(minutely, 90_000),
    ]).toEqual([{ due: [240_000, 300_000], earlier: 3 }, 120_000]);

    // occur once, after which nothing follows
    const once: ScheduleTiming = { timing: "once", startsAt: 60_000 };
    expect([
        Schedule.recent(once, 0, 60_000, 2),
        Schedule.following(once, 0),
        Schedule.following(once, 60_000),
    ]).toEqual([{ due: [60_000], earlier: 0 }, 60_000, undefined]);
});

test("refuse a timing that ends before it starts", () => {
    const backwards = { ...MORNING, startsAt: 60_000, endsAt: 60_000 };
    expect(() => Schedule.require(backwards)).toThrow(
        new RangeError("the schedule ends before its first occurrence"),
    );
    expect(Schedule.require(MORNING)).toBe(MORNING);
});
