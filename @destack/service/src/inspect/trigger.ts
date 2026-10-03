import { TimeZone } from "@destack/schema";
import { CronExpressionParser } from "cron-parser";
import { DeclarationReference } from "@destack/package/declare";
import { type ScheduleTiming, type Trigger, TriggerDescription } from "../trigger/index.ts";

/** Describe a trigger, checking a schedule's timing. */
export function describeTrigger(trigger: Trigger): TriggerDescription {
    // describe a schedule with its call, checking its timing
    const on = trigger.on;
    if ("schedule" in on) {
        describeTiming(on.schedule.timing);

        return TriggerDescription.parse({ name: trigger.name, on });
    }
    // describe the changed object type by reference
    else if ("change" in on) {
        const { object, ...change } = on.change;

        return TriggerDescription.parse({
            name: trigger.name,
            on: { change: { ...change, object: DeclarationReference.of(object) } },
        });
    }

    // describe a webhook without its secret
    const { secret: _secret, ...webhook } = on.webhook;

    return TriggerDescription.parse({ name: trigger.name, on: { webhook } });
}

/** Check a timing, refusing an unknown calendar or time zone and a range that ends before it starts. */
export function describeTiming(described: ScheduleTiming): ScheduleTiming {
    // check the calendar
    if (described.timing === "cron") {
        TimeZone.require(described.timezone);
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
