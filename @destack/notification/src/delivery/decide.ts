import { TimeZone } from "@destack/schema";
import type { Delivery, SkipReason } from "../object/delivery.ts";
import type { Notification } from "../object/activity.ts";
import type { Preference } from "../preference/preference.ts";
import { Window } from "../preference/window.ts";
import type { Focus, Summary } from "../setting/setting.ts";
import { Contact } from "./contact.ts";

/** What a channel does with a notification now. */
export type Decision =
    | { readonly action: "send" }
    | { readonly action: "defer"; readonly until: number; readonly isSummarized: boolean }
    | { readonly action: "skip"; readonly reason: SkipReason };

/** The settings a recipient placed that a decision follows, resolved for one notification. */
export interface Settings {
    /** How the recipient receives the notification. */
    readonly preference: Preference;
    /** When the recipient's notifications stay quiet. */
    readonly focus: Focus;
    /** When the recipient's scheduled summary goes out. */
    readonly summary: Summary;
}

/** The fields of a delivery a decision reads. */
type Decided = Pick<Delivery, "id" | "channel" | "device" | "endpoint" | "isSummarized" | "dueAt">;

/** The fields of a notification a decision reads. */
type Notified = Pick<
    Notification,
    "packageId" | "occurredAt" | "readAt" | "snoozedUntil" | "interruption"
>;

/** Decide what a delivery's channel does with its notification now, the email waiting a delay in milliseconds. */
export function decide(
    delivery: Decided,
    notification: Notified,
    contact: Contact,
    settings: Settings,
    emailDelay: number,
    now: number,
): Decision {
    return (
        screen(delivery, notification, contact, settings, now) ??
        summarize(delivery, notification, contact, settings, now) ??
        interrupt(delivery, notification, contact, settings, emailDelay, now)
    );
}

/** Settle read, unaddressed, snoozed, critical and turned-off notifications, absent otherwise. */
function screen(
    delivery: Decided,
    notification: Notified,
    contact: Contact,
    settings: Settings,
    now: number,
): Decision | undefined {
    // skip read and unaddressed notifications
    if (notification.readAt !== null) {
        return { action: "skip", reason: "read" };
    } else if (!Contact.addresses(contact, delivery)) {
        return { action: "skip", reason: "unaddressed" };
    }
    // wait out a snooze
    else if (notification.snoozedUntil !== null && notification.snoozedUntil > now) {
        return { action: "defer", until: notification.snoozedUntil, isSummarized: false };
    }
    // send critical notifications at once
    else if (notification.interruption === "critical") {
        return { action: "send" };
    }
    // skip channels the recipient turned off
    else if (!settings.preference.channels.includes(delivery.channel)) {
        return { action: "skip", reason: "preference" };
    }

    return undefined;
}

/** Hold passive notifications and summarized preferences for the summary, absent otherwise. */
function summarize(
    delivery: Decided,
    notification: Notified,
    contact: Contact,
    settings: Settings,
    now: number,
): Decision | undefined {
    // summarize passive notifications and summarized preferences
    const { channel } = delivery;
    const { interruption } = notification;
    const isSummarized =
        interruption === "passive" ||
        (settings.preference.delivery === "summary" && interruption !== "timeSensitive");

    // skip channels the summary leaves out
    if (isSummarized && (channel === "desktop" || !settings.summary.channels.includes(channel))) {
        return { action: "skip", reason: "preference" };
    }
    // send with a due summary
    else if (isSummarized && delivery.isSummarized && delivery.dueAt <= now) {
        return { action: "send" };
    }
    // wait for the next summary
    else if (isSummarized) {
        const until = TimeZone.next(contact.timeZone, settings.summary.times, now);

        return { action: "defer", until, isSummarized: true };
    }

    return undefined;
}

/** Wait out a focus and the email delay, or send. */
function interrupt(
    delivery: Decided,
    notification: Notified,
    contact: Contact,
    settings: Settings,
    emailDelay: number,
    now: number,
): Decision {
    // wait out a focus unless allowed through
    const { focus } = settings;
    const end = focusEnd(focus, contact.timeZone, now);
    const isAllowed =
        (notification.interruption === "timeSensitive" && focus.isTimeSensitiveAllowed) ||
        focus.allowed.includes(notification.packageId);
    if (end !== undefined && !isAllowed) {
        return { action: "defer", until: end, isSummarized: false };
    }
    // email once unread for a while
    else if (delivery.channel === "email" && notification.occurredAt + emailDelay > now) {
        return {
            action: "defer",
            until: notification.occurredAt + emailDelay,
            isSummarized: false,
        };
    }

    return { action: "send" };
}

/** Read when a focus ends in a time zone, absent outside one. */
function focusEnd(focus: Focus, timeZone: string, now: number): number | undefined {
    // take the later of the manual and scheduled ends
    const manual = focus.until !== undefined && focus.until > now ? focus.until : undefined;
    const scheduled = Window.end(timeZone, focus.schedules, now);
    const ends = [manual, scheduled].filter((end) => end !== undefined);

    return ends.length === 0 ? undefined : Math.max(...ends);
}
