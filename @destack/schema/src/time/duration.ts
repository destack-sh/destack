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
    days: schema.number().finite().nonnegative().optional(),
    /** Hours. */
    hours: schema.number().finite().nonnegative().optional(),
    /** Minutes. */
    minutes: schema.number().finite().nonnegative().optional(),
    /** Seconds. */
    seconds: schema.number().finite().nonnegative().optional(),
    /** Milliseconds. */
    milliseconds: schema.number().finite().nonnegative().optional(),
});

/** A span of time in the optional units of a Temporal duration. */
export type Duration = Readonly<schema.Infer<typeof durationSchema>>;

/** Measure, check and describe spans of time. */
export const Duration = {
    /** The schema of a duration. */
    schema: durationSchema,

    /** Measure a duration in milliseconds. */
    milliseconds(duration: Duration): number {
        return Object.entries(duration).reduce(
            (total, [unit, count]) =>
                total + (count ?? 0) * UNIT_MILLISECONDS[unit as keyof typeof UNIT_MILLISECONDS],
            0,
        );
    },

    /** Require a duration of known, finite, non-negative units, at least one of them. */
    require(duration: Duration, name: string): void {
        const isValid =
            Duration.schema.safeParse(duration).success && Object.keys(duration).length > 0;
        if (!isValid) {
            throw new TypeError(`${name} is no duration: ${JSON.stringify(duration)}`);
        }
    },
};
