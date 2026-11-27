import { expect, test } from "@destack/test";
import { Duration } from "./duration.ts";

test("measure durations in milliseconds and seconds and refuse empty, negative and calendar ones", () => {
    // add up every unit of fixed length, a week seven days
    expect(
        Duration.milliseconds({ days: 1, hours: 2, minutes: 3, seconds: 4, milliseconds: 5 }),
    ).toBe(93_784_005);
    expect(Duration.seconds({ minutes: 1, milliseconds: 500 })).toBe(60.5);
    expect(Duration.milliseconds({ weeks: 1 })).toBe(604_800_000);
    expect(() => Duration.milliseconds({ months: 1 })).toThrow(
        new RangeError("duration has 'months', whose length needs a date"),
    );

    // accept a forward duration of fixed units, and refuse an empty, negative, fractional or calendar one
    expect(() => Duration.require({ days: 30 }, "recovery")).not.toThrow();
    for (const duration of [
        {},
        { hours: -1 },
        { seconds: 0.5 },
        { months: 1 },
        { fortnights: 1 },
    ]) {
        expect(() => Duration.require(duration, "recovery")).toThrow(
            new TypeError(`'recovery' is not a duration: ${JSON.stringify(duration)}`),
        );
    }
});

test("write and read a duration in ISO 8601 as Temporal does, its sign shared by every unit", () => {
    const durations = [
        {
            years: 1,
            months: 2,
            weeks: 3,
            days: 4,
            hours: 5,
            minutes: 6,
            seconds: 7,
            milliseconds: 8,
        },
        { hours: -1, minutes: -30 },
        { milliseconds: 1500, nanoseconds: 1 },
        {},
    ];
    const written = durations.map((duration) => Duration.iso(duration));

    expect({
        written,
        read: written.map((text) => Duration.read(text)),
        signs: durations.map((duration) => Duration.sign(duration)),
        refused: ["P", "PT", "P1H", "1D", "P1.5D"].map((text) => Duration.read(text)),
        isMixedValid: Duration.safeParse({ hours: 1, minutes: -1 }).success,
    }).toEqual({
        written: ["P1Y2M3W4DT5H6M7.008S", "-PT1H30M", "PT1.500000001S", "PT0S"],
        read: [
            {
                years: 1,
                months: 2,
                weeks: 3,
                days: 4,
                hours: 5,
                minutes: 6,
                seconds: 7,
                milliseconds: 8,
            },
            { hours: -1, minutes: -30 },
            { seconds: 1, milliseconds: 500, nanoseconds: 1 },
            {},
        ],
        signs: [1, -1, 1, 0],
        refused: [undefined, undefined, undefined, undefined, undefined],
        isMixedValid: false,
    });
});

test("write an amount of a unit as whole units, carrying each fraction down", () => {
    expect({
        whole: [
            Duration.of(1.5, "hours"),
            Duration.of(1.5, "years"),
            Duration.of(-0.25, "days"),
            Duration.of(3, "weeks"),
        ],
        refused: (() => {
            try {
                return Duration.of(0.5, "months");
            } catch (error) {
                return error;
            }
        })(),
    }).toEqual({
        whole: [{ hours: 1, minutes: 30 }, { years: 1, months: 6 }, { hours: -6 }, { weeks: 3 }],
        refused: new RangeError("duration has a fraction of a month, whose length needs a date"),
    });
});
