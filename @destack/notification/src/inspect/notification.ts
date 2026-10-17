import type { JsonValue } from "@destack/schema";
import type { Setting } from "@destack/setting";
import type { NotificationDescription } from "../notification/catalog.ts";
import type { NotificationType } from "../notification/notification.ts";

export { NotificationDescription } from "../notification/catalog.ts";

/** Describe a notification for manifests, as the build's inspector names it. */
export function describeNotification(notification: NotificationType): NotificationDescription {
    return notification.describe();
}

/** Read a notification's preference setting, as the build's inspector names it. */
export function notificationPreference(notification: NotificationType): Setting {
    return notification.preference;
}

/** List a notification's term, as the build's inspector names it. */
export function notificationVocabulary(notification: NotificationType): Record<string, JsonValue> {
    return notification.vocabulary();
}
