import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import { defineSchema, schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { describeSetting, SettingDescription } from "@destack/setting/inspect";
import { NotificationMetadata } from "../declare/notification.ts";
import { ActionMetadata } from "../notification/content.ts";
import type { NotificationType } from "../notification/notification.ts";
import { NotificationName } from "../object/activity.ts";

/** A notification as manifests describe it. */
export const NotificationDescription = defineSchema(
    NotificationMetadata.extend({
        /** The declaring package. */
        package: Package,
        /** The payload's JSON Schema. */
        payload: schema.record(schema.string(), schema.json()),
        /** The actions by name. */
        actions: schema.record(NotificationName, ActionMetadata),
    }),
);
/** A declared notification as manifests describe it. */
export type NotificationDescription = schema.Infer<typeof NotificationDescription>;

/** Describe a notification. */
export function describeNotification(notification: NotificationType): NotificationDescription {
    const { name, title, description, interruption, preference, payload } = notification.definition;

    return {
        name,
        title,
        description,
        interruption,
        preference,
        package: notification.package,
        payload: schema.record(schema.string(), schema.json()).parse(toJsonSchema(payload)),
        actions: Object.fromEntries(
            Object.entries(notification.definition.actions ?? {}).map(([action, declared]) => [
                action,
                ActionMetadata.parse(
                    schema.defined({
                        title: declared.title,
                        isDestructive: declared.isDestructive,
                        text: declared.text,
                    }),
                ),
            ]),
        ),
    };
}

/** Describe a notification's preference as the setting recipients change. */
export function describeNotificationPreference(notification: NotificationType): SettingDescription {
    return describeSetting(notification.preference);
}

/** Read the notifications a build declares. */
export async function readNotifications(reader: BuildReader): Promise<NotificationDescription[]> {
    const declared = await reader.declared(
        import.meta.destack.package.id,
        "notification",
        NotificationDescription,
    );

    return declared.map((declaration) => declaration.description);
}

/** List a notification's term: its name, with its payload's shape. */
export function notificationVocabulary(
    input: Record<string, JsonValue>,
): Record<string, JsonValue> {
    const description = NotificationDescription.parse(input);

    return { [description.name]: { payload: description.payload } };
}
