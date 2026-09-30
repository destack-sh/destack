import { CronExpressionParser } from "cron-parser";
import { ScheduleDescription, type Schedule, type ScheduleTiming } from "../schedule/index.ts";

/** Describe a schedule. */
export function describeSchedule(schedule: Schedule): ScheduleDescription {
    // read the declared fields and check their timing
    const { package: _owner, kind: _kind, ...fields } = schedule;
    const description = ScheduleDescription.parse(fields);
    describeTiming(description);

    return description;
}

/** Check a timing, refusing an unknown calendar or time zone and a range that ends before it starts. */
export function describeTiming(described: ScheduleTiming): ScheduleTiming {
    // check the calendar
    if (described.timing === "cron") {
        new Intl.DateTimeFormat("en", { timeZone: described.timezone });
        CronExpressionParser.parse(described.cron, { tz: described.timezone });
    }

    // require a nonempty range
    if (
        "endsAt" in described &&
        described.endsAt !== undefined &&
        described.startsAt !== undefined &&
        described.endsAt <= described.startsAt
    ) {
        throw new RangeError("the schedule ends before its first occurrence");
    }

    return described;
}
