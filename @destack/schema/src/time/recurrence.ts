import * as schema from "../validate/index.ts";
import { PlainDate, WEEK_DAYS, Weekday } from "./plain.ts";
import { Week } from "./week.ts";

/** The months of a year. */
const YEAR_MONTHS = 12;

/** The longest month, which a month day counts back from at -31. */
const MONTH_DAYS = 31;

/** The most weeks of one weekday a year has, which an nth weekday of a year counts within. */
const YEAR_WEEKS = 53;

/** How often a rule repeats, after JSCalendar's frequencies of whole days and longer. */
export const RecurrenceFrequency = schema.enum(["daily", "weekly", "monthly", "yearly"]);

/** How often a rule repeats. */
export type RecurrenceFrequency = schema.Infer<typeof RecurrenceFrequency>;

/** A weekday a rule repeats on, after JSCalendar's NDay: every one, or the nth of its period. */
export const RecurrenceDay = schema.object({
    /** The weekday. */
    day: Weekday,
    /** Which of the period's weekdays: 1 the first, -1 the last, every one when absent. */
    nthOfPeriod: nonZero(YEAR_WEEKS).exactOptional(),
});

/** A weekday a rule repeats on. */
export type RecurrenceDay = schema.Infer<typeof RecurrenceDay>;

/** A rule an event repeats by, after JSCalendar's RecurrenceRule and iCalendar's RRULE. */
export const RecurrenceRule = schema
    .object({
        /** How often it repeats. */
        frequency: RecurrenceFrequency,
        /** The periods between repeats, 1 by default. */
        interval: schema.number().int().positive().exactOptional(),
        /** The weekdays it repeats on, the first date's weekday in a weekly rule by default. */
        byDay: schema.array(RecurrenceDay).min(1).exactOptional(),
        /** The days of the month it repeats on, -1 the last. */
        byMonthDay: schema.array(nonZero(MONTH_DAYS)).min(1).exactOptional(),
        /** The months it repeats in, January 1. */
        byMonth: schema.array(schema.number().int().min(1).max(YEAR_MONTHS)).min(1).exactOptional(),
        /** How many times it happens, the first date included. */
        count: schema.number().int().positive().exactOptional(),
        /** The last date it may happen on. */
        until: PlainDate.exactOptional(),
    })
    .meta({ id: "schema.recurrence", title: "Recurrence" });

/** A rule an event repeats by. */
export type RecurrenceRule = schema.Infer<typeof RecurrenceRule>;

/** The dates of repeating rules, after JSCalendar's recurrence expansion. */
export const Recurrence = {
    /** List the dates rules repeat a first date on within a range, the first date always one. */
    dates(
        rules: readonly RecurrenceRule[],
        first: PlainDate,
        from: PlainDate,
        to: PlainDate,
        week: Week,
    ): PlainDate[] {
        // join every rule's dates, the first date once, in order
        const dates = new Set<PlainDate>();
        for (const rule of rules) {
            for (const date of datesOf(rule, first, from, to, week)) {
                dates.add(date);
            }
        }
        if (PlainDate.compare(first, from) >= 0 && PlainDate.compare(first, to) <= 0) {
            dates.add(first);
        }

        return [...dates].toSorted((left, right) => PlainDate.compare(left, right));
    },

    /** Split rules at a date: the rules ending the day before, and the rules going on from it. */
    split(
        rules: readonly RecurrenceRule[],
        first: PlainDate,
        date: PlainDate,
        week: Week,
    ): { readonly before: RecurrenceRule[]; readonly after: RecurrenceRule[] } {
        // end each rule the day before, its count giving way to the date
        const eve = PlainDate.add(date, { days: -1 });
        const before = rules.map((rule) => {
            const { count: _count, ...rest } = rule;

            return { ...rest, until: eve };
        });

        // go on from the date with each rule's repeats left, leaving out a rule with none left
        const after = rules.flatMap((rule) => {
            // keep a rule without a count as it is
            const count = rule.count;
            if (count === undefined) {
                return [rule];
            }

            // count the repeats before the date, the first one among them
            const used = Recurrence.dates([rule], first, first, eve, week).length;

            return used < count ? [{ ...rule, count: count - used }] : [];
        });

        return { before, after };
    },

    /** Move rules with a date they repeat on, taking its new weekday, month day and month. */
    move(rules: readonly RecurrenceRule[], from: PlainDate, to: PlainDate): RecurrenceRule[] {
        // read the date's values before and after, its month day also counted from the month's end
        const [fromWeekday, toWeekday] = [PlainDate.dayOfWeek(from), PlainDate.dayOfWeek(to)];
        const [fromDay, toDay] = [PlainDate.day(from), PlainDate.day(to)];
        const fromLast = fromDay - PlainDate.daysInMonth(from) - 1;
        const toLast = toDay - PlainDate.daysInMonth(to) - 1;
        const [fromMonth, toMonth] = [PlainDate.month(from), PlainDate.month(to)];

        // replace the old values with the new ones in each rule, the others left as they are
        return rules.map((rule) => ({
            ...rule,
            ...(rule.byDay === undefined
                ? {}
                : {
                      byDay: rule.byDay.map((each) => {
                          // move the date's weekday, its place in the period counted the same way
                          const period = periodOf(rule);
                          const nth = each.nthOfPeriod;
                          if (each.day !== fromWeekday) {
                              return each;
                          } else if (nth === undefined) {
                              return { ...each, day: toWeekday };
                          } else if (nth === nthOf(from, period, Math.sign(nth))) {
                              return {
                                  day: toWeekday,
                                  nthOfPeriod: nthOf(to, period, Math.sign(nth)),
                              };
                          } else {
                              return each;
                          }
                      }),
                  }),
            ...(rule.byMonthDay === undefined
                ? {}
                : {
                      byMonthDay: rule.byMonthDay.map((day) =>
                          day === fromDay ? toDay : day === fromLast ? toLast : day,
                      ),
                  }),
            ...(rule.byMonth === undefined
                ? {}
                : {
                      byMonth: rule.byMonth.map((month) => (month === fromMonth ? toMonth : month)),
                  }),
        }));
    },
};

/** Read the period a rule counts its nth weekdays in: a month, or a year without months. */
function periodOf(rule: RecurrenceRule): "month" | "year" {
    return rule.frequency === "yearly" && rule.byMonth === undefined ? "year" : "month";
}

/** Read a date's place among its weekday's dates in its period, from the start or the end. */
function nthOf(date: PlainDate, period: "month" | "year", sign: number): number {
    // count the days before the date and after it within the period
    const start =
        period === "month"
            ? PlainDate.with(date, { day: 1 })
            : PlainDate.with(date, { month: 1, day: 1 });
    const end = PlainDate.add(
        start,
        period === "month" ? { months: 1, days: -1 } : { years: 1, days: -1 },
    );
    const before = PlainDate.daysUntil(start, date);
    const after = PlainDate.daysUntil(date, end);

    return sign < 0 ? -(Math.floor(after / WEEK_DAYS) + 1) : Math.floor(before / WEEK_DAYS) + 1;
}

/** List the dates one rule repeats a first date on within a range. */
function datesOf(
    rule: RecurrenceRule,
    first: PlainDate,
    from: PlainDate,
    to: PlainDate,
    week: Week,
): PlainDate[] {
    // start at the first period, or jump to the range when no count makes every earlier one matter
    const interval = rule.interval ?? 1;
    const last =
        rule.until === undefined || PlainDate.compare(rule.until, to) > 0 ? to : rule.until;
    let index = rule.count === undefined ? skipped(rule.frequency, interval, first, from, week) : 0;
    let counted = 1;
    const dates: PlainDate[] = [];

    // walk the periods until the range or the rule ends
    for (;;) {
        // stop past the last date the range or the rule allows
        const start = periodStart(rule.frequency, first, index * interval, week);
        if (PlainDate.compare(start, last) > 0) {
            return dates;
        }

        // take the period's dates after the first, which counts as the first repeat, up to the count
        for (const date of candidates(rule, first, start, week)) {
            const isCounted = PlainDate.compare(date, first) > 0;
            counted += isCounted ? 1 : 0;
            if (PlainDate.compare(date, last) > 0 || (rule.count ?? Infinity) < counted) {
                return dates;
            } else if (isCounted && PlainDate.compare(date, from) >= 0) {
                dates.push(date);
            }
        }
        index += 1;
    }
}

/** Count the whole periods before a range starts, which a rule without a count may skip. */
function skipped(
    frequency: RecurrenceFrequency,
    interval: number,
    first: PlainDate,
    from: PlainDate,
    week: Week,
): number {
    // count the periods from the first date's to the range's, never back before the first
    const elapsed = {
        daily: () => PlainDate.daysUntil(first, from),
        weekly: () =>
            PlainDate.daysUntil(Week.startOf(week, first), Week.startOf(week, from)) / WEEK_DAYS,
        monthly: () => monthIndex(from) - monthIndex(first),
        yearly: () => PlainDate.year(from) - PlainDate.year(first),
    }[frequency]();

    return Math.max(0, Math.floor(elapsed / interval));
}

/** Read the start of the period a number of periods after the first date's. */
function periodStart(
    frequency: RecurrenceFrequency,
    first: PlainDate,
    periods: number,
    week: Week,
): PlainDate {
    // start a day, a week, a month or a year from the first date's
    if (frequency === "daily") {
        return PlainDate.add(first, { days: periods });
    } else if (frequency === "weekly") {
        return PlainDate.add(Week.startOf(week, first), { weeks: periods });
    } else if (frequency === "monthly") {
        return PlainDate.add(PlainDate.with(first, { day: 1 }), { months: periods });
    }

    return PlainDate.of(PlainDate.year(first) + periods, 1, 1);
}

/** List a period's dates a rule takes, in order. */
function candidates(
    rule: RecurrenceRule,
    first: PlainDate,
    start: PlainDate,
    week: Week,
): PlainDate[] {
    // take each day of a daily period, the rule's weekdays of a weekly one, and month days of the longer
    if (rule.frequency === "daily") {
        return matches(rule, start) ? [start] : [];
    } else if (rule.frequency === "weekly") {
        const days = rule.byDay?.map((each) => each.day) ?? [PlainDate.dayOfWeek(first)];
        const weekDays = Array.from({ length: WEEK_DAYS }, (_, offset) =>
            PlainDate.add(Week.startOf(week, start), { days: offset }),
        );

        return weekDays.filter(
            (day) =>
                days.includes(PlainDate.dayOfWeek(day)) && isInMonth(rule, PlainDate.month(day)),
        );
    }

    // take the months of the period, then the days each month gives
    const year = PlainDate.year(start);
    const months =
        rule.frequency === "monthly"
            ? [PlainDate.month(start)]
            : (rule.byMonth ?? [PlainDate.month(first)]).toSorted((left, right) => left - right);
    const isYearDays =
        rule.frequency === "yearly" && rule.byMonth === undefined && rule.byDay !== undefined;

    return isYearDays
        ? daysOfYear(rule, year)
        : months
              .filter((month) => rule.frequency === "yearly" || isInMonth(rule, month))
              .flatMap((month) => daysOfMonth(rule, first, PlainDate.of(year, month, 1)));
}

/** List a month's dates a monthly or yearly rule takes, in order. */
function daysOfMonth(rule: RecurrenceRule, first: PlainDate, month: PlainDate): PlainDate[] {
    // take the rule's month days, its weekdays, both together, or the first date's day
    const length = PlainDate.daysInMonth(month);
    const all = Array.from({ length }, (_, offset) => PlainDate.add(month, { days: offset }));
    const byMonthDay = rule.byMonthDay;
    const byDay = rule.byDay;
    const isMonthDay = (date: PlainDate) =>
        byMonthDay?.some((day) => (day > 0 ? day : length + day + 1) === PlainDate.day(date)) ??
        true;
    const isDay = (date: PlainDate) => byDay === undefined || isNthDay(byDay, date, all);
    if (byMonthDay === undefined && byDay === undefined) {
        return all.filter((date) => PlainDate.day(date) === PlainDate.day(first));
    }

    return all.filter((date) => isMonthDay(date) && isDay(date));
}

/** List a year's dates a yearly rule of weekdays takes, an nth weekday counted in the year. */
function daysOfYear(rule: RecurrenceRule, year: number): PlainDate[] {
    // count each weekday through the whole year
    const january = PlainDate.of(year, 1, 1);
    const length = PlainDate.daysUntil(january, PlainDate.of(year + 1, 1, 1));
    const all = Array.from({ length }, (_, offset) => PlainDate.add(january, { days: offset }));
    const byDay = rule.byDay ?? [];

    return all.filter((date) => isNthDay(byDay, date, all));
}

/** Report whether a date is one of a rule's weekdays, nth among its period's same weekdays. */
function isNthDay(days: readonly RecurrenceDay[], date: PlainDate, period: readonly PlainDate[]) {
    // place the date among its period's same weekdays from either end
    const weekday = PlainDate.dayOfWeek(date);
    const same = period.filter((each) => PlainDate.dayOfWeek(each) === weekday);
    const place = same.indexOf(date);

    return days.some(
        (each) =>
            each.day === weekday &&
            (each.nthOfPeriod === undefined ||
                (each.nthOfPeriod > 0 ? place + 1 : place - same.length) === each.nthOfPeriod),
    );
}

/** Report whether a daily rule takes a date: in its months, on its month days and weekdays. */
function matches(rule: RecurrenceRule, date: PlainDate): boolean {
    // filter by every part the rule sets
    const length = PlainDate.daysInMonth(date);
    const isMonthDay =
        rule.byMonthDay?.some(
            (day) => (day > 0 ? day : length + day + 1) === PlainDate.day(date),
        ) ?? true;
    const isDay = rule.byDay?.some((each) => each.day === PlainDate.dayOfWeek(date)) ?? true;

    return isInMonth(rule, PlainDate.month(date)) && isMonthDay && isDay;
}

/** Number a date's month among all months, so two dates' months subtract. */
function monthIndex(date: PlainDate): number {
    return PlainDate.year(date) * YEAR_MONTHS + PlainDate.month(date);
}

/** Report whether a month, January 1, is one of a rule's months, every month without any. */
function isInMonth(rule: RecurrenceRule, month: number): boolean {
    return rule.byMonth?.includes(month) ?? true;
}

/** A whole number from minus a limit to the limit but zero, in two ranges JSON Schema describes. */
function nonZero(limit: number) {
    return schema.union([
        schema.number().int().min(-limit).max(-1),
        schema.number().int().min(1).max(limit),
    ]);
}
