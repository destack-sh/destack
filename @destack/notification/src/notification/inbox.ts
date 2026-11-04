import type { MethodContext } from "@destack/object";
import type { Subject } from "@destack/sync";
import { Activity, activity } from "../object/activity.ts";
import type { Notification } from "../object/notification.ts";

/** A notification as an inbox reads it, with its activity included. */
export type InboxRow = Notification & { readonly activity: Activity | null };

/** One notification in a person's inbox, with its activity and the state the two derive. */
export class InboxEntry {
    /** The person's state of the activity. */
    readonly notification: Notification;
    /** What happened, as the home's replica stores it. */
    readonly activity: Activity;
    /** Whether the person has not read its last occurrence. */
    readonly isUnread: boolean;
    /** Whether the person archived it since its last occurrence. */
    readonly isArchived: boolean;
    /** Whether it is snoozed now. */
    readonly isSnoozed: boolean;
    /** Whether the delivery of the activity's last occurrence started. */
    readonly isPlanned: boolean;

    /** Join a notification with its activity, deriving the person's state of it now. */
    constructor(notification: Notification, occurrence: Activity, now: number) {
        // join the two
        this.notification = notification;
        this.activity = occurrence;

        // derive the read, archived and snoozed state against the last occurrence
        this.isUnread = notification.readAt === null || notification.readAt < occurrence.occurredAt;
        this.isArchived =
            notification.archivedAt !== null && notification.archivedAt >= occurrence.occurredAt;
        this.isSnoozed = notification.snoozedUntil !== null && notification.snoozedUntil > now;
        this.isPlanned = notification.plannedAt >= occurrence.occurredAt;
    }

    /** Read a notification's entry with its activity, absent while the home lacks the activity. */
    static async read(
        context: Pick<MethodContext, "read" | "now">,
        notification: Notification,
    ): Promise<InboxEntry | undefined> {
        const occurrence = await context
            .read(activity)
            .findFirst({ where: { id: notification.activityId } });

        return occurrence === undefined
            ? undefined
            : new InboxEntry(notification, occurrence, context.now);
    }

    /** The person the notification is for. */
    get person(): Subject {
        return Activity.recipient(this.activity);
    }
}

/** A person's inbox: their notifications joined with their activities, latest occurrence first. */
export class Inbox {
    /** The entries, latest occurrence first. */
    readonly entries: readonly InboxEntry[];

    /** Take notifications read with their activity, leaving out those whose activity left. */
    constructor(notifications: readonly InboxRow[], now: number) {
        // derive each notification's state against its activity
        const entries = notifications.flatMap(({ activity: occurrence, ...notification }) =>
            occurrence === null ? [] : [new InboxEntry(notification, occurrence, now)],
        );

        // order the latest occurrence first
        this.entries = entries.toSorted(
            (left, right) => right.activity.occurredAt - left.activity.occurredAt,
        );
    }

    /** List the active entries: unarchived and unsnoozed. */
    active(): readonly InboxEntry[] {
        return this.entries.filter((entry) => !entry.isArchived && !entry.isSnoozed);
    }

    /** Count the unread entries, of one space or app where given. */
    unread(filter: { readonly space?: string; readonly packageId?: string } = {}): number {
        return this.active().filter(
            (entry) =>
                entry.isUnread &&
                (filter.space === undefined || entry.activity.scope === filter.space) &&
                (filter.packageId === undefined || entry.activity.packageId === filter.packageId),
        ).length;
    }
}
