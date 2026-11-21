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
