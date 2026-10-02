import * as schema from "../validate/index.ts";

/** The milliseconds in each unit a duration names. */
const UNIT_MILLISECONDS = {
    days: 86_400_000,
    hours: 3_600_000,
    minutes: 60_000,
    seconds: 1000,
    milliseconds: 1,
} as const;

/** The schema of a duration. */
const durationSchema = schema.object({
    /** Whole or partial days. */
    days: schema.number().nonnegative().exactOptional(),
    /** Hours. */
    hours: schema.number().nonnegative().exactOptional(),
    /** Minutes. */
    minutes: schema.number().nonnegative().exactOptional(),
    /** Seconds. */
    seconds: schema.number().nonnegative().exactOptional(),
    /** Milliseconds. */
    milliseconds: schema.number().nonnegative().exactOptional(),
});

/** A span of time in the optional units of a Temporal duration. */
export type Duration = Readonly<schema.Infer<typeof durationSchema>>;

/** Measure, check and describe spans of time. */
export const Duration = {
    /** The schema of a duration. */
    schema: durationSchema,

    /** Measure a duration in milliseconds. */
    milliseconds(duration: Duration): number {
        const { days = 0, hours = 0, minutes = 0, seconds = 0, milliseconds = 0 } = duration;

        return (
            days * UNIT_MILLISECONDS.days +
            hours * UNIT_MILLISECONDS.hours +
            minutes * UNIT_MILLISECONDS.minutes +
            seconds * UNIT_MILLISECONDS.seconds +
            milliseconds
        );
    },

    /** Require a duration of known, finite, non-negative units, at least one of them. */
    require(duration: unknown, name: string): Duration {
        const parsed = Duration.schema.safeParse(duration);
        if (!parsed.success || Object.keys(parsed.data).length === 0) {
            throw new TypeError(`${name} is no duration: ${JSON.stringify(duration)}`);
        }

        return parsed.data;
    },
};
