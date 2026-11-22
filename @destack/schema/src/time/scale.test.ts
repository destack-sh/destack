import { type Week } from "./week.ts";
import { TimeZone } from "./zone.ts";
import { expect, test } from "@destack/test";
import { TimeScale } from "./scale.ts";

/** The week of the United States: Sunday first. */
const US: Week = { firstDay: 7, weekend: [6, 7], minimalDays: 1 };

/** The week of Germany and Austria: Monday first, numbered as ISO 8601. */
const GERMANY: Week = { firstDay: 1, weekend: [6, 7], minimalDays: 4 };

/** The week of India: Sunday first, with Sunday alone as its weekend. */
const INDIA: Week = { firstDay: 7, weekend: [7], minimalDays: 1 };

/** Write a tick's time as local date and time in a time zone. */
function local(timeZone: string, at: number): string {
    const date = TimeZone.date(timeZone, at);

    return `${date.slice(5)} ${TimeZone.time(timeZone, at)}`;
}

test("place times along the axis, zoom around a place keeping its time, and pan by a share", () => {
    // a ten-second window
    const scale = { start: 0, end: 10_000 };
    const zoomed = TimeScale.zoom(scale, 2, 0.25);

    expect({
        at: TimeScale.at(scale, 2500),
        invert: TimeScale.invert(scale, 0.75),
        zoomed,
        kept: TimeScale.invert(zoomed, 0.25),
        panned: TimeScale.pan(scale, -0.5),
    }).toEqual({
        at: 0.25,
        invert: 7500,
        zoomed: { start: 1250, end: 6250 },
        kept: 2500,
        panned: { start: -5000, end: 5000 },
    });
});

test("mark a trace in milliseconds, a day on local hours, weeks across a DST change and years", () => {
    // a 1.2 second trace, a day in India's half-hour zone, autumn in Vienna, and a decade
    const trace = TimeScale.ticks({ start: 1000, end: 2200 }, 6, "UTC", US);
    const india = TimeScale.ticks(
        { start: Date.parse("2026-11-19T00:00:00Z"), end: Date.parse("2026-11-20T00:00:00Z") },
        8,
        "Asia/Kolkata",
        INDIA,
    );
    const vienna = TimeScale.ticks(
        { start: Date.parse("2026-10-14T00:00:00Z"), end: Date.parse("2026-11-08T00:00:00Z") },
        4,
        "Europe/Vienna",
        GERMANY,
    );
    const decade = TimeScale.ticks(
        { start: Date.parse("2020-06-01T00:00:00Z"), end: Date.parse("2030-06-01T00:00:00Z") },
        5,
        "UTC",
        US,
    );

    expect({
        trace: trace.map((tick) => [tick.at, tick.unit]),
        india: india.map((tick) => local("Asia/Kolkata", tick.at)),
        vienna: vienna.map((tick) => [local("Europe/Vienna", tick.at), tick.unit]),
        decade: decade.map((tick) => new Date(tick.at).getUTCFullYear()),
    }).toEqual({
        trace: [
            [1000, "millisecond"],
            [1200, "millisecond"],
            [1400, "millisecond"],
            [1600, "millisecond"],
            [1800, "millisecond"],
            [2000, "millisecond"],
            [2200, "millisecond"],
        ],
        india: [
            "11-19 06:00",
            "11-19 09:00",
            "11-19 12:00",
            "11-19 15:00",
            "11-19 18:00",
            "11-19 21:00",
            "11-20 00:00",
            "11-20 03:00",
        ],
        vienna: [
            ["10-19 00:00", "week"],
            ["10-26 00:00", "week"],
            ["11-02 00:00", "week"],
        ],
        decade: [2022, 2024, 2026, 2028, 2030],
    });
});

test("snap an instant to the nearest mark at the window's step: a millisecond, a local midnight", () => {
    // a 100 ms trace at 10 marks, and a fortnight in Vienna at 14 marks
    const trace = { start: 0, end: 100 };
    const fortnight = {
        start: Date.parse("2026-11-09T00:00:00Z"),
        end: Date.parse("2026-11-23T00:00:00Z"),
    };

    expect({
        trace: TimeScale.snap(trace, 10, 23.4, "UTC", US),
        fortnight: local(
            "Europe/Vienna",
            TimeScale.snap(
                fortnight,
                14,
                Date.parse("2026-11-12T16:30:00Z"),
                "Europe/Vienna",
                GERMANY,
            ),
        ),
    }).toEqual({ trace: 20, fortnight: "11-13 00:00" });
});

test("snap instants before 1970 to whole hours, the time zone's offset read at minute precision", () => {
    // a day of 1969 in UTC at hourly marks
    const scale = {
        start: Date.parse("1969-07-20T00:00:00Z"),
        end: Date.parse("1969-07-21T00:00:00Z"),
    };

    expect(
        new Date(
            TimeScale.snap(scale, 24, Date.parse("1969-07-20T20:17:40Z"), "UTC", US),
        ).toISOString(),
    ).toBe("1969-07-20T20:00:00.000Z");
});

/** Write a period's ends as ISO 8601 instants. */
function isoOf(period: TimeScale): string[] {
    return [new Date(period.start).toISOString(), new Date(period.end).toISOString()];
}

test("list the months and hours that cover a window, cutting the first and last to it, none past its end", () => {
    // a window from 20 October to 10 December, and one from 9:30 to 11:15, in UTC
    const months = TimeScale.periods(
        { start: Date.UTC(2026, 9, 20), end: Date.UTC(2026, 11, 10) },
        "month",
        "UTC",
        US,
    );
    const hours = TimeScale.periods(
        { start: Date.UTC(2026, 10, 18, 9, 30), end: Date.UTC(2026, 10, 18, 11, 15) },
        "hour",
        "UTC",
        US,
    );

    expect({
        months: months.map(isoOf),
        hours: hours.map(isoOf),
        whole: TimeScale.periods(
            { start: Date.UTC(2026, 10, 1), end: Date.UTC(2026, 11, 1) },
            "month",
            "UTC",
            US,
        ).map(isoOf),
        units: (["day", "week", "hour", "year"] as const).map((unit) => TimeScale.periodOf(unit)),
    }).toEqual({
        months: [
            ["2026-10-20T00:00:00.000Z", "2026-11-01T00:00:00.000Z"],
            ["2026-11-01T00:00:00.000Z", "2026-12-01T00:00:00.000Z"],
            ["2026-12-01T00:00:00.000Z", "2026-12-10T00:00:00.000Z"],
        ],
        hours: [
            ["2026-11-18T09:30:00.000Z", "2026-11-18T10:00:00.000Z"],
            ["2026-11-18T10:00:00.000Z", "2026-11-18T11:00:00.000Z"],
            ["2026-11-18T11:00:00.000Z", "2026-11-18T11:15:00.000Z"],
        ],
        whole: [["2026-11-01T00:00:00.000Z", "2026-12-01T00:00:00.000Z"]],
        units: ["month", "month", "day", undefined],
    });
});

test("step from an instant to the axis's next and previous marks, on the axis's own step", () => {
    // a ten-day axis over 10 marks steps by days, and a day's axis over 8 marks by 3 hours
    const days = { start: Date.UTC(2026, 10, 1), end: Date.UTC(2026, 10, 11) };
    const hours = { start: Date.UTC(2026, 10, 1), end: Date.UTC(2026, 10, 2) };
    const at = Date.UTC(2026, 10, 4, 7, 30);

    expect(
        [
            TimeScale.markBeside(days, 10, at, 1, "UTC", US),
            TimeScale.markBeside(days, 10, at, -1, "UTC", US),
            TimeScale.markBeside(hours, 8, at, 1, "UTC", US),
            TimeScale.markBeside(hours, 8, at, -1, "UTC", US),
        ].map((time) => new Date(time).toISOString()),
    ).toEqual([
        "2026-11-05T00:00:00.000Z",
        "2026-11-04T00:00:00.000Z",
        "2026-11-04T09:00:00.000Z",
        "2026-11-04T06:00:00.000Z",
    ]);
});

test("round an instant to a unit's nearest boundary in a zone, and step to the next or last one", () => {
    // at 15:40 on Thursday 19 November 2026 in Vienna, an hour ahead of UTC
    const zone = "Europe/Vienna";
    const at = TimeZone.instant(zone, "2026-11-19", "15:40");

    expect({
        day: local(zone, TimeScale.round("day", at, zone, GERMANY)),
        hour: local(zone, TimeScale.round("hour", at, zone, GERMANY)),
        week: local(zone, TimeScale.round("week", at, zone, GERMANY)),
        nextDay: local(zone, TimeScale.boundaryBeside("day", at, 1, zone, GERMANY)),
        lastWeek: local(zone, TimeScale.boundaryBeside("week", at, -1, zone, US)),
    }).toEqual({
        day: "11-20 00:00",
        hour: "11-19 16:00",
        week: "11-23 00:00",
        nextDay: "11-20 00:00",
        lastWeek: "11-15 00:00",
    });
});

test("round an instant midway between two boundaries to the later one", () => {
    // noon in UTC lies midway between two midnights
    const noon = Date.UTC(2026, 10, 19, 12);

    expect(local("UTC", TimeScale.round("day", noon, "UTC", GERMANY))).toBe("11-20 00:00");
});
