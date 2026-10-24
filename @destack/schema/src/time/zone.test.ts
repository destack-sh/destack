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
    expect(TimeZone.instant("UTC", { year: 50, month: 3, day: 1 }, "08:00")).toBe(
        -60_584_169_600_000,
    );
});

test("read the date and time of day an instant falls on in a time zone, on either side of midnight", () => {
    // read 23:30 UTC on 24 October 2026 in UTC, and as 01:30 the next day in summer-time Vienna
    const at = Date.UTC(2026, 9, 24, 23, 30);
    expect([
        [TimeZone.date("UTC", at), TimeZone.time("UTC", at)],
        [TimeZone.date("Europe/Vienna", at), TimeZone.time("Europe/Vienna", at)],
    ]).toEqual([
        [{ year: 2026, month: 10, day: 24 }, "23:30"],
        [{ year: 2026, month: 10, day: 25 }, "01:30"],
    ]);
});
