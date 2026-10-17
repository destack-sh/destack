import { type Table } from "@destack/db";
import { activity } from "../object/activity.ts";
import { announcement } from "../object/announcement.ts";
import { deliver, notification, scheduled, summarize } from "../object/notification.ts";
import { subscription } from "../object/subscription.ts";

/** The tables of a space's notification types and its workflows' runs, steps and triggers. */
export const notificationTables: readonly Table[] = [
    activity,
    announcement,
    subscription,
    notification,
    ...[deliver, summarize, scheduled].flatMap((workflow) => Object.values(workflow.objects)),
].flatMap((object) => object.tables);
