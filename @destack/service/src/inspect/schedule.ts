import { CronExpressionParser } from "cron-parser";
import { ScheduleDescription, type Schedule } from "../schedule/index.ts";

/** Describe a declared schedule after checking its calendar and occurrence bounds. */
export function describeSchedule(schedule: Schedule): ScheduleDescription {
    // resolve calendar expressions against their declared time zone
    const { package: _owner, ...fields } = schedule;
    const description = ScheduleDescription.parse(fields);
    if (description.timing === "cron") {
        new Intl.DateTimeFormat("en", { timeZone: description.timezone });
        CronExpressionParser.parse(description.cron, { tz: description.timezone });
    }

    // require a nonempty occurrence interval
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
