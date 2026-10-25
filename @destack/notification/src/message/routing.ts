import type { Destination } from "@destack/message";
import type { MethodContext } from "@destack/object";
import { Instant, schema, TimeZone } from "@destack/schema";
import { setting } from "@destack/setting/object";
import type { Subject } from "@destack/sync";
import type { InboxEntry } from "../notification/inbox.ts";
import type { Activity } from "../object/activity.ts";
import { Preference } from "../preference/preference.ts";
import { Window } from "../preference/window.ts";
import {
    type Focus,
    focus,
    preferences,
    type SummarySchedule,
    summarySchedule,
} from "../setting/setting.ts";
import type { Recipient } from "./recipient.ts";

/** The reasons a destination skips a notification's occurrence. */
export const SKIP_REASONS = ["read", "unaddressed", "preference"] as const;

/** Why a destination skips a notification's occurrence. */
export type SkipReason = (typeof SKIP_REASONS)[number];

/** What routing reads of an inbox entry. */
export type RoutingEntry = Pick<InboxEntry, "isUnread"> & {
    /** The person's snooze of it. */
    readonly notification: Pick<InboxEntry["notification"], "snoozedUntil">;
    /** Who declared it, when it last occurred, and how it interrupts. */
    readonly activity: Pick<Activity, "packageId" | "occurredAt" | "interruption">;
};

/** What a destination does with a notification's occurrence now. */
export const Decision = schema.discriminatedUnion("action", [
    schema.object({
        /** Send it now. */
        action: schema.literal("send"),
    }),
    schema.object({
        /** Decide again later. */
        action: schema.literal("defer"),
        /** When to decide again. */
        until: Instant,
    }),
    schema.object({
        /** Send it in a scheduled summary. */
        action: schema.literal("summarize"),
        /** When the summary goes out. */
        at: Instant,
    }),
    schema.object({
        /** Send nothing. */
        action: schema.literal("skip"),
        /** Why. */
        reason: schema.enum(SKIP_REASONS),
    }),
]);

/** What a destination does with a notification's occurrence now. */
export type Decision = schema.Infer<typeof Decision>;

/** The settings a person placed that a decision follows, resolved for one notification. */
export class Settings {
    /** How the person receives the notification. */
    readonly preference: Preference;
    /** When the person's notifications stay quiet. */
    readonly focus: Focus;
    /** When the person's scheduled summary goes out. */
    readonly summarySchedule: SummarySchedule;

    /** Create a person's resolved settings for one notification. */
    constructor(fields: Pick<Settings, "preference" | "focus" | "summarySchedule">) {
        this.preference = fields.preference;
        this.focus = fields.focus;
        this.summarySchedule = fields.summarySchedule;
    }

    /** Resolve a person's settings for each activity's package and space, in one read. */
    static async read(
        context: MethodContext,
        person: Subject,
        occurrences: readonly Activity[],
    ): Promise<readonly Settings[]> {
        // read the person's placed values of the settings routing resolves
        const references = [preferences, focus, summarySchedule].map(({ reference }) => ({
            packageId: reference.packageId,
            name: reference.name,
        }));
        const placed = await context.read(setting).findMany({
            where: { AND: [{ scope: person.id }, { OR: references }] },
        });

        // resolve each for its activity's space over only the person's own values, its preference else the declared one
        return occurrences.map((occurrence) => {
            const selection = { scope: person.id, space: occurrence.scope };
            const chosen = preferences.resolve(selection, placed, []).value;

            return new Settings({
                preference: chosen[Preference.key(occurrence)] ?? occurrence.preference,
                focus: focus.resolve(selection, placed, []).value,
                summarySchedule: summarySchedule.resolve(selection, placed, []).value,
            });
        });
    }
}

/** The routing of a person's notification occurrence to their destinations at one moment. */
export class Routing {
    /** The inbox entry routed. */
    readonly entry: RoutingEntry;
    /** Where the person receives notifications. */
    readonly recipient: Recipient;
    /** The person's settings for the notification. */
    readonly settings: Settings;
    /** How long an email waits for the notification to be read elsewhere, in milliseconds. */
    readonly emailDelay: number;
    /** The scheduled summary going out now, absent outside one. */
    readonly summaryAt: number | undefined;
    /** The moment of the routing. */
    readonly now: number;

    /** Route an entry to a recipient under their settings at a moment. */
    constructor(
        entry: RoutingEntry,
        recipient: Recipient,
        settings: Settings,
        options: { readonly emailDelay: number; readonly summaryAt?: number },
        now: number,
    ) {
        // take the entry, the recipient, the settings and the moment
        this.entry = entry;
        this.recipient = recipient;
        this.settings = settings;
        this.emailDelay = options.emailDelay;
        this.summaryAt = options.summaryAt;
        this.now = now;
    }

    /** Decide what a destination does with the occurrence now, or in a summary. */
    decide(to: Destination): Decision {
        return this.#screen(to) ?? this.#summarize(to) ?? this.#interrupt(to);
    }

    /** Settle read, unaddressed, snoozed, critical and turned-off notifications. */
    #screen(to: Destination): Decision | undefined {
        // skip a notification read since it last occurred
        const { notification, activity } = this.entry;
        if (!this.entry.isUnread) {
            return { action: "skip", reason: "read" };
        }
        // skip destinations the recipient lost
        else if (!this.recipient.addresses(to)) {
            return { action: "skip", reason: "unaddressed" };
        }
        // wait out a snooze
        else if (notification.snoozedUntil !== null && notification.snoozedUntil > this.now) {
            return { action: "defer", until: notification.snoozedUntil };
        }
        // send critical notifications at once
        else if (activity.interruption === "critical") {
            return { action: "send" };
        }
        // skip channels the person turned off
        else if (!this.settings.preference.channels.some((channel) => channel === to.channel)) {
            return { action: "skip", reason: "preference" };
        }

        return undefined;
    }

    /** Send passive notifications and summarized preferences in the summary, absent otherwise. */
    #summarize(to: Destination): Decision | undefined {
        // summarize passive notifications and summarized preferences
        const { interruption } = this.entry.activity;
        const { preference, summarySchedule: scheduled } = this.settings;
        const isSummarized =
            interruption === "passive" ||
            (preference.delivery === "summary" && interruption !== "timeSensitive");

        // skip channels the summary leaves out
        if (
            isSummarized &&
            (to.channel === "desktop" || !scheduled.channels.some((each) => each === to.channel))
        ) {
            return { action: "skip", reason: "preference" };
        }
        // send in the summary going out now
        else if (isSummarized && this.summaryAt !== undefined) {
            return { action: "send" };
        }
        // schedule for the next summary
        else if (isSummarized) {
            return {
                action: "summarize",
                at: TimeZone.next(this.recipient.timeZone, scheduled.times, this.now),
            };
        }

        return undefined;
    }

    /** Wait out a focus and the email delay, or send. */
    #interrupt(to: Destination): Decision {
        // read the focus and whether the notification breaks through it
        const occurrence = this.entry.activity;
        const quiet = this.settings.focus;
        const end = this.#focusEnd();
        const isAllowed =
            (occurrence.interruption === "timeSensitive" && quiet.isTimeSensitiveAllowed) ||
            quiet.allowed.includes(occurrence.packageId);

        // wait out a focus
        if (end !== undefined && !isAllowed) {
            return { action: "defer", until: end };
        }
        // email once unread for a while
        else if (to.channel === "email" && occurrence.occurredAt + this.emailDelay > this.now) {
            return { action: "defer", until: occurrence.occurredAt + this.emailDelay };
        }

        return { action: "send" };
    }

    /** Read when the person's focus ends, absent outside one. */
    #focusEnd(): number | undefined {
        // take the later of the manual and scheduled ends
        const quiet = this.settings.focus;
        const manual =
            quiet.until !== undefined && quiet.until > this.now ? quiet.until : undefined;
        const scheduled = Window.end(this.recipient.timeZone, quiet.schedules, this.now);
        const ends = [manual, scheduled].filter((end) => end !== undefined);

        return ends.length === 0 ? undefined : Math.max(...ends);
    }
}
