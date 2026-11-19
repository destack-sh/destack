import { expect, test } from "@destack/test";
import { Condition } from "./condition.ts";
import { Now, now } from "./now.ts";

/** Noon of 2026-11-19 in UTC, a Thursday. */
const NOON = Date.UTC(2026, 10, 19, 12);

/** Noon of 2026-03-29 in UTC, the day Vienna moves its clocks forward. */
const SPRING = Date.UTC(2026, 2, 29, 12);

/** The moment in Vienna. */
const VIENNA = { now: NOON, timeZone: "Europe/Vienna" };

test("read now at an offset, rounded to the start of a day, ISO week or month in a time zone", () => {
    expect({
        exact: now.at(VIENNA),
        weekAgo: now.minus({ days: 7 }).at(VIENNA),
        micro: now.microseconds().at({ now: 2, timeZone: "UTC" }),
        day: now.start("day").at(VIENNA),
        tomorrow: now.start("day").plus({ days: 1 }).at(VIENNA),
        week: now.start("week").at(VIENNA),
        month: now.start("month").at(VIENNA),
        spring: now.start("day").at({ now: SPRING, timeZone: "Europe/Vienna" }),
        utc: now.start("day").at({ now: NOON, timeZone: "UTC" }),
    }).toEqual({
        exact: NOON,
        weekAgo: NOON - 7 * 86_400_000,
        micro: 2000,
        day: Date.UTC(2026, 10, 18, 23),
        tomorrow: Date.UTC(2026, 10, 19, 23),
        week: Date.UTC(2026, 10, 15, 23),
        month: Date.UTC(2026, 9, 31, 23),
        spring: Date.UTC(2026, 2, 28, 23),
        utc: Date.UTC(2026, 10, 19),
    });
});

test("read a time back from its placeholder's name, and nothing from another placeholder", () => {
    const time = now.start("week").minus({ days: 1 }).microseconds();

    expect({
        parsed: Now.parse(time.placeholder),
        other: Now.parse("owner"),
    }).toEqual({ parsed: time, other: undefined });
});

test("bind a condition's times to a moment, in memory or in JSON form, and leave other placeholders", () => {
    const condition = {
        dueAt: { lt: now.start("day") },
        OR: [{ seenAt: { gte: { placeholder: now.minus({ hours: 1 }).placeholder } } }],
        owner: { placeholder: "owner" },
    };

    expect(Condition.at(condition, VIENNA)).toEqual({
        dueAt: { lt: Date.UTC(2026, 10, 18, 23) },
        OR: [{ seenAt: { gte: NOON - 3_600_000 } }],
        owner: { placeholder: "owner" },
    });
});

test("write a condition's placeholders in JSON form, which binds alike", () => {
    const condition = { dueAt: { lt: now.start("day") }, owner: { placeholder: "owner" } };
    const written = Condition.json(condition);

    expect({ written, bound: Condition.at(written, VIENNA) }).toEqual({
        written: { dueAt: { lt: { placeholder: "now1_0_day" } }, owner: { placeholder: "owner" } },
        bound: Condition.at(condition, VIENNA),
    });
});
