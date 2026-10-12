import { none, through } from "@destack/access";
import { AccessName, Subject } from "@destack/sync";
import { type Select } from "@destack/db";
import { defineObject, field, ParentReference } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { space } from "@destack/account/object";

import { NOTIFY_RECIPIENTS, NotificationKey, NotificationName } from "./activity.ts";
import { REASONS } from "./subscription.ts";

/** The principals an announcement notifies. */
export const Audience = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The source's subscribers with their reasons. */
            kind: schema.literal("subscribers"),
        }),
        schema.object({
            /** The space's members, as a chat's @channel. */
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
/** The principals an announcement notifies. */
export type Audience = schema.Infer<typeof Audience>;

/** One notification to an audience, expanded into its recipients' activities in batches. */
export const announcement = defineObject({
    name: "announcement",
    plural: "announcements",
    scope: space,
    nested: { in: "any", receive: "announce" },
    fields: {
        /** The principal who announced it, absent for the system. */
        author: field.subject().personal().optional(),
        /** The principals it notifies. */
        audience: field.json(Audience),
        /** The subject keys it skips besides its author. */
        excluded: field
            .json(schema.array(schema.string().min(1)).max(NOTIFY_RECIPIENTS))
            .default([]),

        // notification
        /** The package declaring the notification. */
        packageId: field.string(PackageId),
        /** The notification's name in its package. */
        name: field.string(NotificationName),
        /** The identity a later announcement with the same key replaces. */
        key: field.string(NotificationKey).optional(),
        /** The thread its activities group in. */
        thread: field.string(NotificationKey),
        /** The values its content renders. */
        payload: field.json(schema.json()),

        // expansion
        /** The last audience entry expanded, absent before the first batch. */
        cursor: field.string().optional(),
        /** When the last batch was expanded. */
        expandedAt: field.time().optional(),
    },
    permissions: {
        read: through("parent", "read"),
        // the source's readers announce what their calls do to it
        post: through("parent", "read"),
        expand: none(),
    },
    // keep an expanded announcement 30 days
    expiring: [{ after: { days: 30 }, from: "expandedAt" }],
    constraints: {
        key: {
            kind: "unique",
            on: ["scope", "packageId", "name", "parentPackageId", "parentType", "parentId", "key"],
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    post: method.create("post", { isInternal: true }),
    replace: method.update("post", { isInternal: true }),
    announce: method.mutation({
        permission: "post",
        target: "none",
        isInternal: true,
        input: schema.object({
            /** The object it is about. */
            parent: ParentReference,
            /** The package declaring the notification. */
            packageId: PackageId,
            /** The notification's name in its package. */
            name: NotificationName,
            /** The principals it notifies, and why. */
            audience: Audience,
            /** The principals it skips besides the caller. */
            excluded: schema.array(Subject).max(NOTIFY_RECIPIENTS).exactOptional(),
            /** The values its content renders. */
            payload: schema.json(),
            /** The identity a later announcement with the same key replaces. */
            key: NotificationKey.exactOptional(),
            /** The thread, the source's identifier by default. */
            thread: NotificationKey.exactOptional(),
        }),
        output: schema.object({}),
    }),
    expand: method.mutation({ permission: "expand", isInternal: true }),
}));

/** An announcement as its table stores it. */
export type Announcement = Select<typeof announcement.table>;
