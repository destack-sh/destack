import { expect, test } from "@destack/test";
import { fromJsonSchema, JsonValue, LocalReference, toJsonSchema } from "../json/index.ts";
import { Duration } from "./duration.ts";
import { Recurrence, RecurrenceRule } from "./recurrence.ts";
import type { Week } from "./week.ts";

/** The ISO week: Monday first, the weekend on Saturday and Sunday, week one with four days. */
const ISO: Week = { firstDay: 1, weekend: [6, 7], minimalDays: 4 };

/** Make a monthly rule on the nth Monday of the month. */
function monthlyOn(nthOfPeriod: number) {
    return { frequency: "monthly", byDay: [{ day: 1, nthOfPeriod }] };
}

/** List a rule's dates from a first date over a wide range. */
function datesOf(rule: RecurrenceRule, first: string, to = "2100-12-31"): string[] {
    return Recurrence.dates([rule], first, first, to, ISO);
}

test("repeat by RFC 5545's examples: weekly weekdays, the nth weekday of a month and of a year, and months of a year", () => {
    expect({
        tuesdaysAndThursdays: datesOf(
            { frequency: "weekly", count: 10, byDay: [{ day: 2 }, { day: 4 }] },
            "1997-09-02",
        ),
        firstFridays: datesOf(
            { frequency: "monthly", count: 10, byDay: [{ day: 5, nthOfPeriod: 1 }] },
            "1997-09-05",
        ),
        twentiethMondays: datesOf(
            { frequency: "yearly", count: 3, byDay: [{ day: 1, nthOfPeriod: 20 }] },
            "1997-05-19",
        ),
        juneAndJuly: datesOf({ frequency: "yearly", count: 4, byMonth: [6, 7] }, "1997-06-10"),
    }).toEqual({
        tuesdaysAndThursdays: [
            "1997-09-02",
            "1997-09-04",
            "1997-09-09",
            "1997-09-11",
            "1997-09-16",
            "1997-09-18",
            "1997-09-23",
            "1997-09-25",
            "1997-09-30",
            "1997-10-02",
        ],
        firstFridays: [
            "1997-09-05",
            "1997-10-03",
            "1997-11-07",
            "1997-12-05",
            "1998-01-02",
            "1998-02-06",
            "1998-03-06",
            "1998-04-03",
            "1998-05-01",
            "1998-06-05",
        ],
        twentiethMondays: ["1997-05-19", "1998-05-18", "1999-05-17"],
        juneAndJuly: ["1997-06-10", "1997-07-10", "1998-06-10", "1998-07-10"],
    });
});

test("repeat on month ends, skip months that lack a day, and stop at the until date", () => {
    expect({
        lastDays: datesOf({ frequency: "monthly", count: 4, byMonthDay: [-1] }, "2026-01-31"),
        thirtyFirsts: datesOf({ frequency: "monthly", count: 4 }, "2026-01-31"),
        everyOtherDay: datesOf(
            { frequency: "daily", interval: 2, until: "2026-11-09" },
            "2026-11-01",
        ),
        lastFridays: datesOf(
            { frequency: "monthly", until: "2026-12-31", byDay: [{ day: 5, nthOfPeriod: -1 }] },
            "2026-10-30",
        ),
    }).toEqual({
        lastDays: ["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30"],
        thirtyFirsts: ["2026-01-31", "2026-03-31", "2026-05-31", "2026-07-31"],
        everyOtherDay: ["2026-11-01", "2026-11-03", "2026-11-05", "2026-11-07", "2026-11-09"],
        lastFridays: ["2026-10-30", "2026-11-27", "2026-12-25"],
    });
});

test("list only a range's dates, counting a counted rule from its first date and jumping an endless one there", () => {
    // a weekly standup since 2020, and ten fortnightly reviews from the same day
    const range = ["2026-11-16", "2026-11-29"] as const;
    const standup = Recurrence.dates([{ frequency: "weekly" }], "2020-01-06", ...range, ISO);
    const reviews = Recurrence.dates(
        [{ frequency: "weekly", interval: 2, count: 10 }],
        "2026-09-07",
        ...range,
        ISO,
    );

    expect({ standup, reviews }).toEqual({
        standup: ["2026-11-16", "2026-11-23"],
        reviews: ["2026-11-16"],
    });
});

test("count the first date as the first repeat even when the rule's weekdays leave it out", () => {
    // a Wednesday start of a rule on Mondays and Fridays, twice
    expect(
        datesOf({ frequency: "weekly", count: 3, byDay: [{ day: 1 }, { day: 5 }] }, "2026-11-18"),
    ).toEqual(["2026-11-18", "2026-11-20", "2026-11-23"]);
});

test("split rules at a date: ending each the day before, and going on from it with its count's repeats left", () => {
    // ten daily repeats from 1 November, and an endless weekly one, split on 5 November
    const { before, after } = Recurrence.split(
        [{ frequency: "daily", count: 10 }, { frequency: "weekly" }],
        "2026-11-01",
        "2026-11-05",
        ISO,
    );

    expect({ before, after }).toEqual({
        before: [
            { frequency: "daily", until: "2026-11-04" },
            { frequency: "weekly", until: "2026-11-04" },
        ],
        after: [{ frequency: "daily", count: 6 }, { frequency: "weekly" }],
    });
});

test("move rules with a date: its weekday, month day and month become the new date's, the others stay", () => {
    // a Monday and Wednesday meeting, the 15th of each month, and the last day of March each year
    const rules: RecurrenceRule[] = [
        { frequency: "weekly", byDay: [{ day: 1 }, { day: 3 }] },
        { frequency: "monthly", byMonthDay: [1, 15] },
        { frequency: "yearly", byMonth: [3], byMonthDay: [-1] },
    ];

    expect([
        Recurrence.move(rules.slice(0, 2), "2026-06-15", "2026-06-16"),
        Recurrence.move(rules.slice(2), "2026-03-31", "2026-04-30"),
    ]).toEqual([
        [
            // from Monday the 15th to Tuesday the 16th
            { frequency: "weekly", byDay: [{ day: 2 }, { day: 3 }] },
            { frequency: "monthly", byMonthDay: [1, 16] },
        ],
        // the last of March to the last of April
        [{ frequency: "yearly", byMonth: [4], byMonthDay: [-1] }],
    ]);
});

test("move numbered weekdays with a date to its place in the new month, counted from the same end", () => {
    // the second Tuesday a week later, the last Friday a day earlier, and Tuesday the 14th a day on
    const second: RecurrenceRule[] = [
        { frequency: "monthly", byDay: [{ day: 2, nthOfPeriod: 2 }] },
    ];
    const last: RecurrenceRule[] = [{ frequency: "monthly", byDay: [{ day: 5, nthOfPeriod: -1 }] }];

    expect([
        Recurrence.move(second, "2026-07-14", "2026-07-21"),
        Recurrence.move(last, "2026-07-31", "2026-07-30"),
        Recurrence.move(second, "2026-07-14", "2026-07-15"),
    ]).toEqual([
        [{ frequency: "monthly", byDay: [{ day: 2, nthOfPeriod: 3 }] }],
        [{ frequency: "monthly", byDay: [{ day: 4, nthOfPeriod: -1 }] }],
        // july starts on a Wednesday, so the 15th is its third
        [{ frequency: "monthly", byDay: [{ day: 3, nthOfPeriod: 3 }] }],
    ]);
});

test("describe a rule as JSON Schema by its name, read back refusing the zeroth weekday as before", () => {
    // describe the rule, read it back, and check an nth weekday of 0, -1 and 54
    const described = toJsonSchema(RecurrenceRule);
    const back = fromJsonSchema(described);

    expect({
        names: [Duration, RecurrenceRule].map((each) =>
            LocalReference.root(JsonValue.object(toJsonSchema(each))),
        ),
        taken: [0, -1, 54].map((nth) => [
            RecurrenceRule.safeParse(monthlyOn(nth)).success,
            back.safeParse(monthlyOn(nth)).success,
        ]),
    }).toEqual({
        names: ["schema.duration", "schema.recurrence"],
        taken: [
            [false, false],
            [true, true],
            [false, false],
        ],
    });
});
