import { expect, test } from "@destack/test";
import { Duration } from "./duration.ts";

test("measure durations in milliseconds and seconds and refuse empty, negative and unknown ones", () => {
    // add up every unit
    expect(
        Duration.milliseconds({ days: 1, hours: 2, minutes: 3, seconds: 4, milliseconds: 5 }),
    ).toBe(93_784_005);
    expect(Duration.seconds({ minutes: 1, milliseconds: 500 })).toBe(60.5);

    // accept a duration of one unit, and refuse an empty, negative, infinite or unknown one
    expect(() => Duration.require({ days: 30 }, "recovery")).not.toThrow();
    for (const duration of [{}, { hours: -1 }, { seconds: Infinity }, { weeks: 1 }]) {
        expect(() => Duration.require(duration, "recovery")).toThrow(
            new TypeError(`recovery is no duration: ${JSON.stringify(duration)}`),
        );
    }
});
