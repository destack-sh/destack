import { schema } from "@destack/schema";

/** Milliseconds per second, as CSS times convert. */
const MILLISECONDS = 1000;

/** A unit a duration takes, as CSS writes times. */
export const DurationUnit = schema.enum(["ms", "s"]);
/** A unit a duration takes. */
export type DurationUnit = schema.Infer<typeof DurationUnit>;

/** A length of time, as the design token format's duration: a number and its unit. */
const durationJson = schema
    .object({
        /** The number, never below zero. */
        value: schema.number().nonnegative(),
        /** The unit. */
        unit: DurationUnit,
    })
    .meta({ id: "style.duration", title: "Duration" });
/** A length of time. */
export type Duration = schema.Infer<typeof durationJson>;

/** A length of time, as the design token format's duration: a number and its unit. */
export const Duration = Object.assign(durationJson, {
    /** Write a duration as CSS writes a time, such as `200ms`. */
    css(duration: Duration): string {
        return `${String(duration.value)}${duration.unit}`;
    },

    /** Read a duration in milliseconds. */
    milliseconds(duration: Duration): number {
        return duration.unit === "ms" ? duration.value : duration.value * MILLISECONDS;
    },
});
