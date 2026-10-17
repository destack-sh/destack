import type { MethodContext } from "@destack/object";
import type { Subject } from "@destack/sync";
import { Activity, activity } from "../object/activity.ts";
import { Notification } from "../object/notification.ts";

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
    }

    /** Read a notification's entry with its activity, absent once the home lost the activity. */
    static async read(
        context: Pick<MethodContext, "read" | "now">,
        notification: Notification,
    ): Promise<InboxEntry | undefined> {
        const occurrence = await context
            .read(activity)
            .findFirst({ where: { id: Notification.activity(notification.id) } });

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

    /** Join each notification with its activity replica, leaving out one whose activity expired. */
    constructor(
        notifications: readonly Notification[],
        activities: readonly Activity[],
        now: number,
    ) {
        // pair each notification with its activity
        const byId = new Map(activities.map((row) => [row.id, row]));
        const entries = notifications.flatMap((notification) => {
            const occurrence = byId.get(Notification.activity(notification.id));

            return occurrence === undefined ? [] : [new InboxEntry(notification, occurrence, now)];
        });

        // order the latest occurrence first
        this.entries = entries.toSorted(
            (left, right) => right.activity.occurredAt - left.activity.occurredAt,
        );
    }

    /** Count the unread entries, of one space or app where given. */
    unread(filter: { readonly space?: string; readonly packageId?: string } = {}): number {
        return this.entries.filter(
            (entry) =>
                entry.isUnread &&
                !entry.isArchived &&
                !entry.isSnoozed &&
                (filter.space === undefined || entry.activity.scope === filter.space) &&
                (filter.packageId === undefined || entry.activity.packageId === filter.packageId),
        ).length;
    }
}
