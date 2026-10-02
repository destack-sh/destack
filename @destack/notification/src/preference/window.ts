import { defineSchema, PlainDate, PlainTime, schema, TimeZone, Weekday } from "@destack/schema";

/** The schema of a recurring span of local time on some weekdays. */
const windowSchema = defineSchema(
    schema.object({
        /** The weekdays it starts on. */
        days: schema.array(Weekday).min(1).max(7),
        /** The time it starts. */
        from: PlainTime,
        /** The time it ends, the next day when not after the start. */
        to: PlainTime,
    }),
);
/** A recurring span of local time on some weekdays. */
export type Window = schema.Infer<typeof windowSchema>;

/** A recurring span of local time on some weekdays. */
export const Window = Object.assign(windowSchema, {
    /** Read when the windows containing now end in a time zone, absent outside them. */
    end(zone: TimeZone, windows: readonly Window[], now: number): number | undefined {
        // check windows starting yesterday and today
        const today = TimeZone.date(zone, now);
        const ends: number[] = [];
        for (const days of [-1, 0]) {
            const date = PlainDate.add(today, days);
            for (const window of windows) {
                // skip windows not starting on the date's weekday
                if (!window.days.includes(PlainDate.weekday(date))) {
                    continue;
                }

                // check that now lies between start and end, which is the next day when not after the start
                const isOvernight = PlainTime.minutes(window.to) <= PlainTime.minutes(window.from);
                const start = TimeZone.instant(zone, date, window.from);
                const stop = TimeZone.instant(
                    zone,
                    PlainDate.add(date, isOvernight ? 1 : 0),
                    window.to,
                );
                if (start <= now && now < stop) {
                    ends.push(stop);
                }
            }
        }

        return ends.length === 0 ? undefined : Math.max(...ends);
    },
});
