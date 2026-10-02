import { expect, test } from "@destack/test";
import { TimeZone } from "./zone.ts";

test("accept known time zones and their aliases, and refuse unknown ones", () => {
    // accept canonical names and aliases
    for (const zone of ["Europe/Vienna", "UTC", "US/Eastern"]) {
        expect(() => TimeZone.require(zone)).not.toThrow();
    }

    // refuse a well-formed but unknown zone, and a malformed name
    expect(() => TimeZone.require("Mars/Olympus")).toThrow(RangeError);
    expect(TimeZone.safeParse("Europe Vienna").success).toBe(false);
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
});
