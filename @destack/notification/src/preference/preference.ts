import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { DELIVERY_CHANNELS } from "../object/delivery.ts";
import { LocalTime, Window } from "./zone.ts";

/** The most quiet windows one focus schedules: two per day for a week. */
const WINDOW_COUNT = 14;

/** The most summaries a day, as Apple allows twelve. */
const SUMMARY_COUNT = 12;

/** How strongly a notification interrupts, as Apple's interruption levels. */
export const INTERRUPTION_LEVELS = ["passive", "active", "timeSensitive", "critical"] as const;

/** How strongly a notification interrupts. */
export type InterruptionLevel = (typeof INTERRUPTION_LEVELS)[number];

/** The channels that alert beside the inbox. */
export const CHANNELS = ["desktop", ...DELIVERY_CHANNELS] as const;

/** A channel that alerts. */
export type Channel = (typeof CHANNELS)[number];

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

/** When notifications stay quiet, as Apple's Focus. */
export const focus = defineSetting({
    name: "focus",
    title: "Focus",
    description: "When notifications wait in the inbox instead of alerting you.",
    schema: schema.object({
        /** The quiet hours. */
        schedules: schema.array(Window).max(WINDOW_COUNT),
        /** When a manual focus ends, absent while off. */
        until: schema.number().int().nonnegative().optional(),
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
        times: schema.array(LocalTime).min(1).max(SUMMARY_COUNT),
        /** The channels. */
        channels: schema.array(schema.enum(DELIVERY_CHANNELS)),
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
