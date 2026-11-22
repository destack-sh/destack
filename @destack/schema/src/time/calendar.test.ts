import { expect, test } from "@destack/test";
import { WorkCalendar, WorkWeek } from "./calendar.ts";
import type { Weekday } from "./plain.ts";
import { TimeZone } from "./zone.ts";

/** The milliseconds of a day. */
const DAY = 86_400_000;

/** The milliseconds of an hour. */
const HOUR = 3_600_000;

/** The weekend of Germany: Saturday and Sunday. */
const WEEKEND: readonly Weekday[] = [6, 7];

/** Read the instant of a wall time on a day of November 2026 in Berlin. */
function berlin(date: number, hour: number, minute = 0): number {
    const day = `2026-11-${String(date).padStart(2, "0")}`;
    const time = `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;

    return TimeZone.instant("Europe/Berlin", day, time);
}

/** Write an instant as its date and wall time in Berlin. */
function local(instant: number): string {
    return `${TimeZone.date("Europe/Berlin", instant)} ${TimeZone.time("Europe/Berlin", instant)}`;
}

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
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));

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
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));

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
                return new WorkCalendar("UTC", WorkWeek.of([1, 2, 3, 4, 5, 6, 7]));
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
        new WorkCalendar("UTC", WorkWeek.of(WEEKEND)),
        new WorkCalendar("Asia/Kabul", WorkWeek.of([4, 5])),
        new WorkCalendar("Europe/Berlin", WorkWeek.of([5])),
        new WorkCalendar("America/New_York", WorkWeek.of([5, 7])),
        new WorkCalendar("Europe/Berlin", {
            1: [{ start: "09:00", end: "17:00" }],
            3: [
                { start: "08:00", end: "12:00" },
                { start: "13:00", end: "24:00" },
            ],
            6: [{ start: "00:00", end: "06:30" }],
        }),
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
        zero: [at(9), at(9), at(9), at(9), at(9)],
    });
});

test("align a weekend across a block's edge, and count a day short of daylight saving", () => {
    // a Thursday and Friday weekend in Kabul across the edge of a four-week block from the epoch
    const block = 28 * DAY;
    const edge = Math.floor(Date.UTC(2026, 10, 1) / block) * block;
    const kabul = new WorkCalendar("Asia/Kabul", WorkWeek.of([4, 5]));
    const summer = new WorkCalendar("Europe/Berlin", WorkWeek.of(WEEKEND));
    const [weekend] = kabul.nonWorking({ start: edge - 3 * DAY, end: edge + 3 * DAY });

    expect({
        start: new Date(kabul.align(edge, "start")).toISOString(),
        end: new Date(kabul.align(edge, "end")).toISOString(),
        weekend: [weekend?.start, weekend?.end].map((each) => new Date(each ?? 0).toISOString()),
        // from Friday noon to Monday eleven in Berlin, over the end of summer time, is 23 hours
        berlin: summer.between(Date.UTC(2026, 9, 23, 10), Date.UTC(2026, 9, 26, 10)) / 3_600_000,
    }).toEqual({
        start: "2026-10-23T19:30:00.000Z",
        end: "2026-10-21T19:30:00.000Z",
        weekend: ["2026-10-21T19:30:00.000Z", "2026-10-23T19:30:00.000Z"],
        berlin: 23,
    });
});

test("count fifty years of working days, past the blocks a calendar keeps listed", () => {
    // count the weekdays of each day from 2000 to 2050 one by one
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));
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

test("count working hours of a week, a working day's evenings and weekends left out", () => {
    // nine to five from Monday to Friday, with an hour off at noon on Fridays
    const days = { start: "09:00", end: "17:00" } as const;
    const calendar = new WorkCalendar("Europe/Berlin", {
        1: [days],
        2: [days],
        3: [days],
        4: [days],
        5: [
            { start: "09:00", end: "12:00" },
            { start: "13:00", end: "17:00" },
        ],
    });
    const refusal = (() => {
        try {
            return new WorkCalendar("UTC", { 1: [{ start: "12:00", end: "09:00" }] });
        } catch (error) {
            return error;
        }
    })();

    expect({
        // two hours from Friday four lose the weekend and the evening, landing on Monday ten
        friday: local(calendar.add(berlin(6, 16), 2 * HOUR)),
        noon: local(calendar.add(berlin(6, 11), 2 * HOUR)),
        monday: calendar.between(berlin(9, 8), berlin(9, 18)) / HOUR,
        week: calendar.between(berlin(9, 0), berlin(16, 0)) / HOUR,
        starts: local(calendar.align(berlin(9, 7), "start")),
        ends: local(calendar.align(berlin(9, 18), "end")),
        isWorking: [calendar.isWorking(berlin(9, 10)), calendar.isWorking(berlin(6, 12, 30))],
        refusal,
    }).toEqual({
        friday: "2026-11-09 10:00",
        noon: "2026-11-06 14:00",
        monday: 8,
        week: 39,
        starts: "2026-11-09 09:00",
        ends: "2026-11-09 17:00",
        isWorking: [true, false],
        refusal: new RangeError(
            "work calendar is invalid: the hours of day '1' overlap or run backward",
        ),
    });
});

/** Read the instant of a wall time in Berlin on the Sunday summer time starts. */
function spring(time: string): number {
    return TimeZone.instant("Europe/Berlin", "2026-03-29", time);
}

test("start working hours that fall in the spring gap the gap's hour later, and refuse a week of weekend", () => {
    // half past two to five on the Sunday Berlin skips from two to three
    const sunday = { start: "02:30", end: "05:00" } as const;
    const calendar = new WorkCalendar("Europe/Berlin", { 7: [sunday] });
    const refusal = (() => {
        try {
            return new WorkCalendar("UTC", WorkWeek.of([1, 2, 3, 4, 5, 6, 7]));
        } catch (error) {
            return error;
        }
    })();

    expect({
        starts: `${TimeZone.date("Europe/Berlin", calendar.align(spring("01:00"), "start"))} ${TimeZone.time("Europe/Berlin", calendar.align(spring("01:00"), "start"))}`,
        hours: calendar.between(spring("00:00"), spring("12:00")) / HOUR,
        refusal,
    }).toEqual({
        // the skipped hour leaves an hour and a half of the two and a half on the wall
        starts: "2026-03-29 03:30",
        hours: 1.5,
        refusal: new RangeError("work calendar is invalid: its week has no working day"),
    });
});
