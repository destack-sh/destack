import { expect, test } from "@destack/test";
import { WorkCalendar } from "./calendar.ts";
import type { Weekday } from "./plain.ts";

/** The milliseconds of a day. */
const DAY = 86_400_000;

/** The weekend of Germany: Saturday and Sunday. */
const WEEKEND: readonly Weekday[] = [6, 7];

/** Read the instant of a day of November 2026 at an hour in UTC, the 2nd a Monday. */
function at(date: number, hour = 0): number {
    return Date.UTC(2026, 10, date, hour);
}

/** Write an instant as its day of November and its hour in UTC. */
function when(instant: number): string {
    const date = new Date(instant);

    return `${String(date.getUTCDate())} ${String(date.getUTCHours())}h`;
}

test("count working time over a weekend, forward and back, as a calendar's lags and lengths do", () => {
    // a day of work from Friday noon runs to Monday noon, and back again
    const calendar = new WorkCalendar("UTC", WEEKEND);

    expect({
        forward: when(calendar.add(at(6, 12), DAY)),
        back: when(calendar.add(at(9, 12), -DAY)),
        between: calendar.between(at(6, 12), at(9, 12)) / DAY,
        backward: calendar.between(at(9, 12), at(6, 12)) / DAY,
        fromSaturday: when(calendar.add(at(7, 9), DAY / 2)),
        isWorking: [calendar.isWorking(at(6, 12)), calendar.isWorking(at(7, 12))],
    }).toEqual({
        forward: "9 12h",
        back: "6 12h",
        between: 1,
        backward: -1,
        fromSaturday: "9 12h",
        isWorking: [true, false],
    });
});

test("align edges off the weekend and count working time across many weeks", () => {
    // a start on Saturday moves to Monday, an end on Saturday or Monday's midnight back to Friday's end
    const calendar = new WorkCalendar("UTC", WEEKEND);

    expect({
        start: when(calendar.align(at(7, 10), "start")),
        end: when(calendar.align(at(7, 10), "end")),
        mondayEnd: when(calendar.align(at(9), "end")),
        working: when(calendar.align(at(6, 10), "start")),
        sixWeeks: when(calendar.add(at(2), 30 * DAY)),
        sixWeeksBack: when(calendar.add(Date.UTC(2026, 11, 12), -30 * DAY)),
        between: calendar.between(at(2), Date.UTC(2026, 11, 12)) / DAY,
        refused: (() => {
            try {
                return new WorkCalendar("UTC", [1, 2, 3, 4, 5, 6, 7]);
            } catch (error) {
                return error;
            }
        })(),
    }).toEqual({
        start: "9 0h",
        end: "7 0h",
        mondayEnd: "7 0h",
        working: "6 10h",
        // thirty working days from Monday the 2nd end at Friday the 11th of December's end
        sixWeeks: "12 0h",
        sixWeeksBack: "2 0h",
        between: 30,
        refused: new RangeError("work calendar is invalid: its week has no working day"),
    });
});

test("keep a duration across many calendars: add and between undo each other", () => {
    // walk a fixed sequence of instants and durations through weekends of different shapes and zones
    const calendars = [
        new WorkCalendar("UTC", WEEKEND),
        new WorkCalendar("Asia/Kabul", [4, 5]),
        new WorkCalendar("Europe/Berlin", [5]),
        new WorkCalendar("America/New_York", [5, 7]),
    ];
    const failures: string[] = [];
    let seed = 7;
    const next = () => {
        seed = (seed * 48_271) % 2_147_483_647;

        return seed / 2_147_483_647;
    };
    for (const calendar of calendars) {
        for (let run = 0; run < 500; run += 1) {
            // pick a whole millisecond in 2026 and a duration of up to sixty working days either way
            const from = Math.round(Date.UTC(2026, 0, 1) + next() * 365 * DAY);
            const duration = Math.round((next() - 0.5) * 120 * DAY);
            const moved = calendar.add(from, duration);
            const isOrdered = duration >= 0 ? moved >= from : moved <= from;
            if (calendar.between(from, moved) !== duration || !isOrdered) {
                failures.push(`${calendar.timeZone} ${String(from)} ${String(duration)}`);
            }
        }
    }

    // every case comes back to its duration, on the side of the instant it moves to
    expect({ failures, zero: calendars.map((calendar) => calendar.add(at(9), 0)) }).toEqual({
        failures: [],
        zero: [at(9), at(9), at(9), at(9)],
    });
});

test("align a weekend across a block's edge, and count a day short of daylight saving", () => {
    // a Thursday and Friday weekend in Kabul across the edge of a four-week block from the epoch
    const block = 28 * DAY;
    const edge = Math.floor(Date.UTC(2026, 10, 1) / block) * block;
    const kabul = new WorkCalendar("Asia/Kabul", [4, 5]);
    const berlin = new WorkCalendar("Europe/Berlin", WEEKEND);
    const [weekend] = kabul.nonWorking({ start: edge - 3 * DAY, end: edge + 3 * DAY });

    expect({
        start: new Date(kabul.align(edge, "start")).toISOString(),
        end: new Date(kabul.align(edge, "end")).toISOString(),
        weekend: [weekend?.start, weekend?.end].map((each) => new Date(each ?? 0).toISOString()),
        // from Friday noon to Monday eleven in Berlin, over the end of summer time, is 23 hours
        berlin: berlin.between(Date.UTC(2026, 9, 23, 10), Date.UTC(2026, 9, 26, 10)) / 3_600_000,
    }).toEqual({
        start: "2026-10-23T19:30:00.000Z",
        end: "2026-10-21T19:30:00.000Z",
        weekend: ["2026-10-21T19:30:00.000Z", "2026-10-23T19:30:00.000Z"],
        berlin: 23,
    });
});

test("count fifty years of working days, past the blocks a calendar keeps listed", () => {
    // count the weekdays of each day from 2000 to 2050 one by one
    const calendar = new WorkCalendar("UTC", WEEKEND);
    const start = Date.UTC(2000, 0, 1);
    const end = Date.UTC(2050, 0, 1);
    let weekdays = 0;
    for (let day = start; day < end; day += DAY) {
        const weekday = new Date(day).getUTCDay();
        weekdays += weekday === 0 || weekday === 6 ? 0 : 1;
    }

    // count them, then again once the first blocks have been forgotten and listed anew
    const first = calendar.between(start, end) / DAY;
    const again = calendar.between(start, end) / DAY;

    expect({ first, again }).toEqual({ first: weekdays, again: weekdays });
});
