import { expect, test } from "@destack/test";
import { PlainDate } from "./plain.ts";

test("order dates by year, then month, then day", () => {
    const dates: PlainDate[] = ["2026-10-04", "2025-12-31", "2026-02-28", "2026-10-01"];
    expect(dates.toSorted((left, right) => PlainDate.compare(left, right))).toEqual([
        "2025-12-31",
        "2026-02-28",
        "2026-10-01",
        "2026-10-04",
    ]);
});

test("equal the same day, and nothing that is absent", () => {
    const day = "2026-10-04";
    expect([
        PlainDate.equals(day, "2026-10-04"),
        PlainDate.equals(day, "2026-10-05"),
        PlainDate.equals(day, undefined),
        PlainDate.equals(undefined, undefined),
    ]).toEqual([true, false, false, false]);
});

test("take dates written as ISO 8601, read their fields, and refuse days and months that do not exist", () => {
    // read a leap day and a year below 100, build a date from its fields, then refuse impossible ones
    const accepted = [
        "2028-02-29",
        "0099-12-31",
        "2026-02-29",
        "2026-13-01",
        "2026-1-1",
        "2100-02-29",
    ];

    expect({
        accepted: accepted.map((text) => PlainDate.safeParse(text).success),
        fields: [
            PlainDate.year("0099-12-31"),
            PlainDate.month("0099-12-31"),
            PlainDate.day("0099-12-31"),
        ],
        built: PlainDate.of(2028, 2, 29),
        refused: (() => {
            try {
                return PlainDate.of(2026, 2, 29);
            } catch (error) {
                return error;
            }
        })(),
    }).toEqual({
        accepted: [true, true, false, false, false, false],
        fields: [99, 12, 31],
        built: "2028-02-29",
        refused: new RangeError("not a date: '2026-02-29'"),
    });
});

test("move dates by months first, keeping the day within shorter months, then by weeks and days", () => {
    const moved = [
        PlainDate.add("2026-01-31", { months: 1 }),
        PlainDate.add("2028-01-31", { months: 1 }),
        PlainDate.add("2028-02-29", { years: 1 }),
        PlainDate.add("2026-12-30", { weeks: 1 }),
        PlainDate.add("2026-03-01", { days: -1 }),
        PlainDate.add("2026-01-31", { months: 1, days: 1 }),
    ].map((each) => each);

    expect(moved).toEqual([
        "2026-02-28",
        "2028-02-29",
        "2029-02-28",
        "2027-01-06",
        "2026-02-28",
        "2026-03-01",
    ]);
});

test("count days between dates, read weekdays, month lengths and ISO weeks across year ends", () => {
    // the first and last weeks of a year can belong to the neighbouring year
    expect({
        until: [
            PlainDate.daysUntil("2026-11-19", "2027-01-01"),
            PlainDate.daysUntil("2026-03-01", "2026-02-01"),
        ],
        dayOfWeek: PlainDate.dayOfWeek("2026-11-19"),
        daysInMonth: [
            PlainDate.daysInMonth("2026-02-01"),
            PlainDate.daysInMonth("2028-02-01"),
            PlainDate.daysInMonth("2026-12-15"),
        ],
        weekOfYear: ["2026-01-01", "2026-12-31", "2027-01-01", "2027-01-04"].map((text) =>
            PlainDate.weekOfYear(text),
        ),
        months: [
            PlainDate.add("2026-01-01", { months: -1 }),
            PlainDate.add("2026-11-01", { years: 1, months: 2 }),
        ],
    }).toEqual({
        until: [43, -28],
        dayOfWeek: 4,
        daysInMonth: [28, 29, 31],
        weekOfYear: [1, 53, 53, 1],
        months: ["2025-12-01", "2028-01-01"],
    });
});
