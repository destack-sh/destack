import { through } from "@destack/access";
import { AccessName } from "@destack/sync";
import { unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { NOTIFY_RECIPIENTS, NotificationKey, NotificationName } from "./notification.ts";
import { REASONS } from "./subscription.ts";

/** The principals an announcement reaches. */
export const Audience = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The source's subscribers with their reasons. */
            kind: schema.literal("subscribers"),
        }),
        schema.object({
            /** The space's members, as Slack's @channel. */
            kind: schema.literal("members"),
            /** Why the members receive it. */
            reason: schema.enum(REASONS),
        }),
        schema.object({
            /** The users with a permission on the source. */
            kind: schema.literal("permission"),
            /** The permission, such as "edit". */
            permission: AccessName,
            /** Why those users receive it. */
            reason: schema.enum(REASONS),
        }),
    ]),
);
/** The principals an announcement reaches. */
export type Audience = schema.Infer<typeof Audience>;

/** One notification to an audience, expanded into recipients' notifications in batches. */
export const announcement = defineObject({
    name: "announcement",
    plural: "announcements",
    scope: space,
    nested: { in: "any", receive: "announce" },
    fields: {
        /** The principal who announced it. */
        author: field.subject().personal(),
        /** The principals it reaches. */
        audience: field.json(Audience),
        /** The subject keys it skips beside its author. */
        excluded: field
            .json(schema.array(schema.string().min(1)).max(NOTIFY_RECIPIENTS))
            .default([]),

        // notification
        /** The package declaring the notification. */
        packageId: field.string(PackageId),
        /** The notification's name in its package. */
        name: field.string(NotificationName),
        /** The identity a later announcement with the same key replaces, absent for one never replaced. */
        key: field.string(NotificationKey).optional(),
        /** The thread its notifications group in. */
        thread: field.string(NotificationKey),
        /** The values its content renders. */
        payload: field.json(schema.json()),

        // expansion
        /** The last audience entry expanded, absent before the first batch. */
        cursor: field.string().optional(),
        /** When the last batch was expanded. */
        expandedAt: field.time().optional(),
    },
    constraints: (announcement) => [
        unique("announcement_key").on(
            announcement.scope,
            announcement.packageId,
            announcement.name,
            announcement.parentPackageId,
            announcement.parentType,
            announcement.parentId,
            announcement.key,
        ),
    ],
    permissions: { read: through("parent", "read") },
    // keep an expanded announcement 30 days, as long as read notifications
    expiring: [{ after: { days: 30 }, from: "expandedAt" }],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        post: method.create(null, { isSystem: true }),
        replace: method.update(null, { isSystem: true }),
        expand: method({ permission: null, isSystem: true }),
    },
});

/** An announcement as its table stores it. */
export type Announcement = Select<typeof announcement.table>;
