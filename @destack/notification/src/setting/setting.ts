import { PackageId } from "@destack/package";
import { PlainTime, schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { CHANNELS } from "../preference/preference.ts";
import { Window } from "../preference/window.ts";

/** The most quiet windows one focus schedules: two per day for a week. */
const WINDOW_COUNT = 14;

/** The most summaries a day, as mobile platforms allow twelve. */
const SUMMARY_COUNT = 12;

/** When notifications stay quiet, as mobile platforms' focus modes. */
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
