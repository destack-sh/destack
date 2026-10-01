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
