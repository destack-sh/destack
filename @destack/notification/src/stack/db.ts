import { type Table } from "@destack/db";
import { activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { notification } from "../object/notification.ts";
import { subscription } from "../object/subscription.ts";
import { summary } from "../object/summary.ts";

/** The tables of a space's notification types. */
export const notificationTables: readonly Table[] = [
    activity,
    announcement,
    subscription,
    notification,
    summary,
].flatMap((object) => object.tables);
