import type { AggregateOptions, FindOptions } from "@destack/object";
import type { PackageId } from "@destack/package";
import type { notification } from "../object/activity.ts";

/** A person's notifications in their inbox, latest first. */
export const NOTIFICATIONS = {
    where: { archivedAt: { isNull: true } },
    orderBy: { occurredAt: "desc" },
} as const satisfies FindOptions<typeof notification, typeof notification>;

/** A person's unread notifications counted per space and app. */
export const UNREAD = {
    where: {
        readAt: { isNull: true },
        snoozedUntil: { isNull: true },
        archivedAt: { isNull: true },
    },
    groupBy: ["space", "packageId"],
    values: {
        unread: { function: "count" },
        latestAt: { function: "max", column: "occurredAt" },
    },
} as const satisfies AggregateOptions<typeof notification, typeof notification>;

/** The unread notifications of one app in one space. */
export interface Badge {
    /** The space of the notifications. */
    readonly space: string;
    /** The app. */
    readonly packageId: PackageId;
    /** The unread count. */
    readonly unread: number;
    /** When the latest occurred. */
    readonly latestAt: number;
}
