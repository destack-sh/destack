import type { SkipReason } from "../object/delivery.ts";
import type { NotificationRow } from "../object/notification.ts";
import type {
    Channel,
    Focus,
    InterruptionLevel,
    Preference,
    Summary,
} from "../preference/preference.ts";
import { TimeZone } from "../preference/zone.ts";

/** How long an email waits for the notification to be read elsewhere, as Slack and Linear do, in milliseconds. */
export const EMAIL_DELAY = 15 * 60_000;

/** What a channel does with a notification now. */
export type Decision =
    | { readonly action: "send" }
    | { readonly action: "defer"; readonly until: number; readonly isSummarized: boolean }
    | { readonly action: "skip"; readonly reason: SkipReason };

/** What a decision reads. */
export interface Circumstances {
    /** The channel. */
    readonly channel: Channel;
    /** The notification. */
    readonly notification: Pick<
        NotificationRow,
        "parentPackageId" | "occurredAt" | "readAt" | "snoozedUntil"
    >;
    /** The interruption level. */
    readonly interruption: InterruptionLevel;
    /** The recipient's preference. */
    readonly preference: Preference;
    /** The recipient's focus. */
    readonly focus: Focus;
    /** The recipient's scheduled summary. */
    readonly summary: Summary;
    /** The recipient's time zone. */
    readonly timeZone: string;
    /** Whether the recipient may read the notification. */
    readonly isReadable: boolean;
    /** Whether the recipient has an address on the channel. */
    readonly isAddressed: boolean;
    /** Whether the recipient is active on another device. */
    readonly isPresent: boolean;
    /** Whether the summary goes out now. */
    readonly isSummaryDue: boolean;
    /** The time of the decision, in UTC epoch milliseconds. */
    readonly now: number;
}

/** Decide what a channel does with a notification now. */
export function decide(circumstances: Circumstances): Decision {
    // read the circumstances
    const { channel, notification, interruption, preference, now } = circumstances;

    // skip unreadable, read and unaddressed notifications
    if (!circumstances.isReadable) {
        return { action: "skip", reason: "withheld" };
    } else if (notification.readAt !== null) {
        return { action: "skip", reason: "read" };
    } else if (!circumstances.isAddressed) {
        return { action: "skip", reason: "unaddressed" };
    }
    // wait out a snooze
    else if (notification.snoozedUntil !== null && notification.snoozedUntil > now) {
        return { action: "defer", until: notification.snoozedUntil, isSummarized: false };
    }
    // send critical notifications at once
    else if (interruption === "critical") {
        return { action: "send" };
    }
    // skip channels the recipient turned off
    else if (!preference.channels.includes(channel)) {
        return { action: "skip", reason: "preference" };
    }

    // summarize passive notifications and summarized preferences
    const isSummarized =
        interruption === "passive" ||
        (preference.delivery === "summary" && interruption !== "timeSensitive");
    if (
        isSummarized &&
        (channel === "desktop" || !circumstances.summary.channels.includes(channel))
    ) {
        return { action: "skip", reason: "preference" };
    } else if (isSummarized && circumstances.isSummaryDue) {
        return { action: "send" };
    } else if (isSummarized) {
        const until = TimeZone.next(circumstances.timeZone, circumstances.summary.times, now);

        return { action: "defer", until, isSummarized: true };
    }

    // wait out a focus unless allowed through
    const end = focusEnd(circumstances);
    const isAllowed =
        (interruption === "timeSensitive" && circumstances.focus.isTimeSensitiveAllowed) ||
        (notification.parentPackageId !== null &&
            circumstances.focus.allowed.includes(notification.parentPackageId));
    if (end !== undefined && !isAllowed) {
        return { action: "defer", until: end, isSummarized: false };
    }
    // email once unread for a while
    else if (channel === "email" && notification.occurredAt + EMAIL_DELAY > now) {
        return {
            action: "defer",
            until: notification.occurredAt + EMAIL_DELAY,
            isSummarized: false,
        };
    }
    // skip push while the recipient is active elsewhere
    else if (channel === "push" && circumstances.isPresent) {
        return { action: "skip", reason: "present" };
    }

    return { action: "send" };
}

/** Read when the recipient's focus ends, absent outside one. */
function focusEnd(circumstances: Circumstances): number | undefined {
    // take the later of the manual and scheduled ends
    const { focus, now } = circumstances;
    const manual = focus.until !== undefined && focus.until > now ? focus.until : undefined;
    const scheduled = TimeZone.end(circumstances.timeZone, focus.schedules, now);
    const ends = [manual, scheduled].filter((end) => end !== undefined);

    return ends.length === 0 ? undefined : Math.max(...ends);
}
