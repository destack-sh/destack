import { type Table } from "@destack/db";
import { activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { subscription } from "../object/subscription.ts";

/** The tables of activities, announcements and subscriptions on a space's objects. */
export const notificationTables: readonly Table[] = [activity, announcement, subscription].flatMap(
    (object) => object.tables,
);
