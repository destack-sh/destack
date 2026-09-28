import { schema } from "@destack/schema";

/** The milliseconds in each unit a duration names. */
const UNIT_MILLISECONDS = {
    days: 86_400_000,
    hours: 3_600_000,
    minutes: 60_000,
    seconds: 1000,
    milliseconds: 1,
} as const;

/** A span of time in the units of a Temporal duration, each one optional. */
export type Duration = { readonly [Unit in keyof typeof UNIT_MILLISECONDS]?: number };

/** Measure, check and describe spans of time. */
export const Duration = {
    /** The schema of a duration. */
    schema: schema.object({
        /** Whole or partial days. */
        days: schema.number().nonnegative().optional(),
        /** Hours. */
        hours: schema.number().nonnegative().optional(),
        /** Minutes. */
        minutes: schema.number().nonnegative().optional(),
        /** Seconds. */
        seconds: schema.number().nonnegative().optional(),
        /** Milliseconds. */
        milliseconds: schema.number().nonnegative().optional(),
    }),

    /** Measure a duration in milliseconds. */
    milliseconds(duration: Duration): number {
        return Object.entries(duration).reduce(
            (total, [unit, count]) =>
                total + count * UNIT_MILLISECONDS[unit as keyof typeof UNIT_MILLISECONDS],
            0,
        );
    },

    /** Require a duration to name at least one unit. */
    require(duration: Duration, name: string): void {
        const entries = Object.entries(duration);
        const isValid =
            entries.length > 0 &&
            entries.every(
                ([unit, count]) =>
                    Object.hasOwn(UNIT_MILLISECONDS, unit) && Number.isFinite(count) && count >= 0,
            );
        if (!isValid) {
            throw new TypeError(`${name} is no duration: ${JSON.stringify(duration)}`);
        }
    },
};
