import type { Package } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Setting } from "@destack/setting";

/** The channels that alert beside the inbox. */
export const CHANNELS = ["desktop", "push", "email"] as const;

/** A channel that alerts. */
export type Channel = (typeof CHANNELS)[number];

/** How strongly a notification interrupts, as mobile platforms' interruption levels. */
export const INTERRUPTION_LEVELS = ["passive", "active", "timeSensitive", "critical"] as const;

/** How strongly a notification interrupts. */
export type InterruptionLevel = (typeof INTERRUPTION_LEVELS)[number];

/** How a recipient receives one notification. */
const PreferenceSchema = defineSchema(
    schema.object({
        /** The channels that alert. */
        channels: schema.array(schema.enum(CHANNELS)),
        /** Alert at once, or in the scheduled summary. */
        delivery: schema.enum(["immediate", "summary"]),
    }),
);

/** How a recipient receives one notification, and the setting keeping it per notification. */
export const Preference = Object.assign(PreferenceSchema, {
    /** Build the setting of a recipient's preference for one declared notification. */
    setting(
        owner: Package,
        notification: {
            /** The notification's name. */
            readonly name: string;
            /** The preference recipients start from. */
            readonly preference: schema.Infer<typeof PreferenceSchema>;
            /** The label settings show, the name by default. */
            readonly title?: string;
            /** What it tells, the label by default. */
            readonly description?: string;
        },
    ): Setting<typeof PreferenceSchema> {
        const title = notification.title ?? notification.name;

        return new Setting(owner, {
            name: `notification.${notification.name}`,
            title,
            description: notification.description ?? title,
            schema: PreferenceSchema,
            default: notification.preference,
            scope: "user",
            overrides: ["space", "installation", "device"],
            apply: "immediate",
        });
    },
});
/** How a recipient receives one declared notification. */
export type Preference = schema.Infer<typeof Preference>;
