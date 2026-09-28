import { Condition } from "@destack/db/query";
import type { ObjectQuery } from "@destack/object";
import type { PackageId } from "@destack/package";

/** A person's notifications, latest first. */
export const NOTIFICATIONS: ObjectQuery = {
    object: "notification",
    order: [{ column: "occurredAt", direction: "desc" }],
};

/** A person's unread notifications counted per space and app. */
export const UNREAD: ObjectQuery = {
    object: "notification",
    where: Condition.all(Condition.missing("readAt"), Condition.missing("snoozedUntil")),
    aggregate: {
        groupBy: ["origin", "parentPackageId"],
        values: {
            unread: { function: "count" },
            latestAt: { function: "max", column: "occurredAt" },
        },
    },
};

/** The unread notifications of one app in one space. */
export interface Badge {
    /** The space holding the notifications. */
    readonly space: string;
    /** The app. */
    readonly packageId: PackageId;
    /** The unread count. */
    readonly unread: number;
    /** When the latest occurred. */
    readonly latestAt: number;
}
