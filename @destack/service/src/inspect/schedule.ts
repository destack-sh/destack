import { CronExpressionParser } from "cron-parser";
import { ScheduleDescription, type Schedule } from "../schedule/index.ts";

/** Describe a schedule. */
export function describeSchedule(schedule: Schedule): ScheduleDescription {
    // check the calendar
    const { package: _owner, kind: _kind, handle: _handle, ...fields } = schedule;
    const description = ScheduleDescription.parse(fields);
    if (description.timing === "cron") {
        new Intl.DateTimeFormat("en", { timeZone: description.timezone });
        CronExpressionParser.parse(description.cron, { tz: description.timezone });
    }

    // require a nonempty interval
    if (
        "endsAt" in description &&
        description.endsAt !== undefined &&
        description.startsAt !== undefined &&
        description.endsAt <= description.startsAt
    ) {
        throw new RangeError(`schedule ends before its first occurrence: ${description.name}`);
    }

    return description;
}
