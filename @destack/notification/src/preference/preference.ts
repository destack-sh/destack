import { defineSchema, schema } from "@destack/schema";

/** The channels that alert beside the inbox. */
export const CHANNELS = ["desktop", "push", "email"] as const;

/** A channel that alerts. */
export type Channel = (typeof CHANNELS)[number];

/** How strongly a notification interrupts, as Apple's interruption levels. */
export const INTERRUPTION_LEVELS = ["passive", "active", "timeSensitive", "critical"] as const;

/** How strongly a notification interrupts. */
export type InterruptionLevel = (typeof INTERRUPTION_LEVELS)[number];

/** How a recipient receives one notification. */
export const Preference = defineSchema(
    schema.object({
        /** The channels that alert. */
        channels: schema.array(schema.enum(CHANNELS)),
        /** Alert at once, or in the scheduled summary. */
        delivery: schema.enum(["immediate", "summary"]),
    }),
);
/** How a recipient receives one declared notification. */
export type Preference = schema.Infer<typeof Preference>;
