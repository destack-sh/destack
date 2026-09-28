import { Package } from "@destack/package";
import type {} from "@destack/package/import-meta";
import type { PackageReader } from "@destack/package/manifest";
import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { describeSetting, SettingDescription } from "@destack/setting/inspect";
import { ActionMetadata, NotificationMetadata } from "../declare/notification.ts";
import type { Notification } from "../notification/notification.ts";
import { NotificationName } from "../object/notification.ts";

/** A notification as manifests describe it. */
export const NotificationDescription = defineSchema(
    NotificationMetadata.extend({
        /** The declaring package. */
        package: Package,
        /** The payload's JSON Schema. */
        payload: schema.record(schema.string(), schema.json()),
        /** The actions by name. */
        actions: schema.record(NotificationName, ActionMetadata),
        /** The preference setting. */
        setting: SettingDescription,
    }),
);
/** A declared notification as manifests describe it. */
export type NotificationDescription = schema.Infer<typeof NotificationDescription>;

/** Describe a notification. */
export function describeNotification(notification: Notification): NotificationDescription {
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
                ActionMetadata.parse({
                    title: declared.title,
                    isDestructive: declared.isDestructive,
                    text: declared.text,
                }),
            ]),
        ),
        setting: describeSetting(notification.preference),
    };
}

/** Read the notifications a build declares. */
export function readNotifications(reader: PackageReader): Promise<NotificationDescription[]> {
    return reader.declared(import.meta.destack.package.id, "notification", NotificationDescription);
}
