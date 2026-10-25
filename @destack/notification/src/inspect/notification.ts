import type { JsonValue } from "@destack/schema";
import type { NotificationDescription } from "../notification/notification.ts";
import type { NotificationType } from "../notification/notification.ts";

export { NotificationDescription } from "../notification/notification.ts";

/** Describe a notification for manifests. */
export function describeNotification(notification: NotificationType): NotificationDescription {
    return notification.describe();
}

/** List a notification's terms for manifests. */
export function notificationVocabulary(notification: NotificationType): Record<string, JsonValue> {
    return notification.vocabulary();
}
