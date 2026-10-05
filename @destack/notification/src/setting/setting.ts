import { type Package, PackageId } from "@destack/package";
import { PlainTime, schema } from "@destack/schema";
import { Setting } from "@destack/setting";
import { defineSetting } from "@destack/setting/declare";
import { CHANNELS, Preference } from "../preference/preference.ts";
import { Window } from "../preference/window.ts";

/** The most quiet windows one focus schedules: two per day for a week. */
const WINDOW_COUNT = 14;

/** The most summaries a day, as Apple allows twelve. */
const SUMMARY_COUNT = 12;

/** Build the setting of a recipient's preference for one declared notification, starting from its declared preference. */
export function preferenceSetting(
    owner: Package,
    notification: {
        /** The notification's name. */
        readonly name: string;
        /** The preference recipients start from. */
        readonly preference: Preference;
        /** The label settings show, the name by default. */
        readonly title?: string;
        /** What it tells, the label by default. */
        readonly description?: string;
    },
): Setting<typeof Preference> {
    const title = notification.title ?? notification.name;

    return new Setting(owner, {
        name: `notification.${notification.name}`,
        title,
        description: notification.description ?? title,
        schema: Preference,
        default: notification.preference,
        scope: "user",
        overrides: ["space", "installation", "device"],
        apply: "immediate",
    });
}

/** When notifications stay quiet, as Apple's Focus. */
export const focus = defineSetting({
    name: "focus",
    title: "Focus",
    description: "When notifications wait in the inbox instead of alerting you.",
    schema: schema.object({
        /** The quiet hours. */
        schedules: schema.array(Window).max(WINDOW_COUNT),
        /** When a manual focus ends, absent while off. */
        until: schema.number().int().nonnegative().exactOptional(),
        /** The packages that alert during a focus. */
        allowed: schema.array(PackageId),
        /** Whether time-sensitive notifications alert during a focus. */
        isTimeSensitiveAllowed: schema.boolean(),
    }),
    default: { schedules: [], allowed: [], isTimeSensitiveAllowed: true },
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
});

/** When the scheduled summary goes out, and on which channels. */
export const summary = defineSetting({
    name: "summary",
    title: "Scheduled summary",
    description: "When notifications set to arrive in a summary are sent, and how.",
    schema: schema.object({
        /** The times of day it goes out. */
        times: schema.array(PlainTime).min(1).max(SUMMARY_COUNT),
        /** The channels other than the desktop. */
        channels: schema.array(schema.enum(CHANNELS).exclude(["desktop"])),
    }),
    default: { times: ["08:00", "18:00"], channels: ["email"] },
    scope: "user",
    overrides: ["space"],
    apply: "immediate",
});

/** When notifications stay quiet. */
export type Focus = schema.Infer<typeof focus.definition.schema>;

/** When the scheduled summary goes out, and how. */
export type Summary = schema.Infer<typeof summary.definition.schema>;
