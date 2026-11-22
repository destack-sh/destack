import { expect, test } from "@destack/test";
import { TimeZone } from "./zone.ts";

test("accept known time zones and their aliases, and refuse unknown ones", () => {
    // accept canonical names and aliases
    const known = ["Europe/Vienna", "UTC", "US/Eastern"];
    for (const zone of known) {
        expect(() => TimeZone.require(zone)).not.toThrow();
    }

    // refuse a well-formed but unknown zone, and a malformed name
    expect(() => TimeZone.require("Mars/Olympus")).toThrow(RangeError);
    expect(TimeZone.safeParse("Europe Vienna").success).toBe(false);

    // report the same as known or unknown without throwing
    expect([...known, "Mars/Olympus"].map((zone) => TimeZone.isKnown(zone))).toEqual([
        true,
        true,
        true,
        false,
    ]);
});

test("place times of day in a time zone across the end and start of summer time", () => {
    // place 08:00 on Sunday in winter time, an hour later in UTC than on Saturday 24 October 2026
    const saturday = Date.UTC(2026, 9, 24, 12, 0);
    const sunday = TimeZone.next("Europe/Vienna", ["08:00"], saturday);
    const later = TimeZone.next("Europe/Vienna", ["08:00", "18:00"], saturday);

    // move 02:30 on the night the clocks go forward past the gap, to 03:30 summer time
    const gap = TimeZone.next("Europe/Vienna", ["02:30"], Date.UTC(2026, 2, 28, 12, 0));
    expect([sunday, later, gap]).toEqual([
        Date.UTC(2026, 9, 25, 7, 0),
        Date.UTC(2026, 9, 24, 16, 0),
        Date.UTC(2026, 2, 29, 1, 30),
    ]);
    expect(() => TimeZone.next("Mars/Olympus", ["08:00"], saturday)).toThrow(RangeError);

    // keep a year below 100 as it is, 0050-03-01T08:00Z, where Date.UTC would read 1950
    expect(TimeZone.instant("UTC", "0050-03-01", "08:00")).toBe(-60_584_169_600_000);
});

test("read the date and time of day an instant falls on in a time zone, on either side of midnight", () => {
    // read 23:30 UTC on 24 October 2026 in UTC, and as 01:30 the next day in summer-time Vienna
    const at = Date.UTC(2026, 9, 24, 23, 30);
    expect([
        [TimeZone.date("UTC", at), TimeZone.time("UTC", at)],
        [TimeZone.date("Europe/Vienna", at), TimeZone.time("Europe/Vienna", at)],
    ]).toEqual([
        ["2026-10-24", "23:30"],
        ["2026-10-25", "01:30"],
    ]);
});

test("read wall times before 1970 at their exact instant, the minute floored toward the past", () => {
    // the Moon landing in UTC and in New York, both before the epoch
    const date = "1969-07-20";

    expect([
        new Date(TimeZone.instant("UTC", date, "20:17")).toISOString(),
        new Date(TimeZone.instant("America/New_York", date, "16:17")).toISOString(),
    ]).toEqual(["1969-07-20T20:17:00.000Z", "1969-07-20T20:17:00.000Z"]);
});

test("read each quarter hour of the days summer time starts and ends at its wall time, minute by minute", () => {
    // every quarter hour around Berlin's two changes of 2026, written out and read back
    const walls = ["2026-03-29", "2026-10-25"].flatMap((date) => {
        const midnight = TimeZone.instant("Europe/Berlin", date, "00:00");

        return Array.from({ length: 4 * 4 }, (_, index) => {
            const at = midnight + index * 15 * 60_000;

            return `${TimeZone.date("Europe/Berlin", at)} ${TimeZone.time("Europe/Berlin", at)} ${String(TimeZone.offset("Europe/Berlin", at) / 3_600_000)}`;
        });
    });

    expect(walls).toEqual([
        // the clocks skip from two to three in March
        "2026-03-29 00:00 1",
        "2026-03-29 00:15 1",
        "2026-03-29 00:30 1",
        "2026-03-29 00:45 1",
        "2026-03-29 01:00 1",
        "2026-03-29 01:15 1",
        "2026-03-29 01:30 1",
        "2026-03-29 01:45 1",
        "2026-03-29 03:00 2",
        "2026-03-29 03:15 2",
        "2026-03-29 03:30 2",
        "2026-03-29 03:45 2",
        "2026-03-29 04:00 2",
        "2026-03-29 04:15 2",
        "2026-03-29 04:30 2",
        "2026-03-29 04:45 2",
        // and go from three back to two in October, two o'clock happening twice
        "2026-10-25 00:00 2",
        "2026-10-25 00:15 2",
        "2026-10-25 00:30 2",
        "2026-10-25 00:45 2",
        "2026-10-25 01:00 2",
        "2026-10-25 01:15 2",
        "2026-10-25 01:30 2",
        "2026-10-25 01:45 2",
        "2026-10-25 02:00 2",
        "2026-10-25 02:15 2",
        "2026-10-25 02:30 2",
        "2026-10-25 02:45 2",
        "2026-10-25 02:00 1",
        "2026-10-25 02:15 1",
        "2026-10-25 02:30 1",
        "2026-10-25 02:45 1",
    ]);
});
