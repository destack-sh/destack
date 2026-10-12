import { intersection, none, relation, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field, type ObjectType, ParentReference } from "@destack/object";
import { Package, PackageId } from "@destack/package";
import { defineSchema, Instant, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Subject } from "@destack/sync";
import { account, residence, space } from "@destack/account/object";

import { ActionMetadata, Wording } from "../notification/content.ts";
import { INTERRUPTION_LEVELS, Preference } from "../preference/preference.ts";
import { REASONS } from "./subscription.ts";

/** The most recipients one notify call lists: 100 inserts of about 0.3 ms. */
export const NOTIFY_RECIPIENTS = 100;

/** The longest key or thread. */
const KEY_LENGTH = 200;

/** A notification's name, one segment of its preference setting's name. */
export const NotificationName = defineSchema(schema.string().regex(/^[a-z][a-zA-Z0-9]*$/u));

/** A notification key or thread. */
export const NotificationKey = defineSchema(schema.string().min(1).max(KEY_LENGTH));

/** What happened to an object, for one person. */
export const activity = defineObject({
    name: "activity",
    plural: "activities",
    scope: space,
    nested: { in: "any", receive: "notify" },
    fields: {
        /** The person it is for. */
        recipient: field.subject().personal(),
        /** The principal whose call made it, absent for the system's. */
        actor: field.subject().personal().optional(),

        // declaration and identity
        /** The package declaring the notification. */
        packageId: field.string(PackageId),
        /** The release of the declaring package, whose preference setting it follows. */
        release: field.json(Package),
        /** The notification's name in its package. */
        name: field.string(NotificationName),
        /** The identity a later notice with the same key replaces. */
        key: field.string(NotificationKey).optional(),
        /** The thread it groups in. */
        thread: field.string(NotificationKey),

        // occurrence
        /** Why the recipient hears of it. */
        reason: field.enum(REASONS),
        /** The values its content renders. */
        payload: field.json(schema.json()),
        /** The occurrences collapsed into it. */
        count: field.integer().default(1),
        /** When it last occurred. */
        occurredAt: field.time(),

        // rendering in the recipient's locale, as their home shows and delivers it
        /** The text every channel shows, rendered in its recipient's locale. */
        content: field.json(Wording),
        /** The line a summary shows for it, such as "3 mentions", in its recipient's locale. */
        summary: field.string(),
        /** The actions offered beside it, by name. */
        actions: field.json(schema.record(NotificationName, ActionMetadata)),
        /** How strongly it interrupts. */
        interruption: field.enum(INTERRUPTION_LEVELS),
        /** The preference its recipient starts from. */
        preference: field.json(Preference),
    },
    projections: (): readonly ObjectType[] => [notification],
    permissions: {
        // the recipient reads it through its source
        read: intersection(relation("recipient"), through("parent", "read")),
        // the source's readers post what their calls do to it
        post: through("parent", "read"),
    },
    // keep an activity 180 days, as code forges keep notifications five months
    expiring: [{ after: { days: 180 }, from: "occurredAt" }],
    constraints: {
        key: {
            kind: "unique",
            on: [
                "scope",
                "recipient",
                "packageId",
                "name",
                "parentPackageId",
                "parentType",
                "parentId",
                "key",
            ],
        },
        thread: { kind: "index", on: ["scope", "recipient", "thread", "packageId", "name"] },
        count: { kind: "check", condition: { count: { gte: 1 } } },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    post: method.create("post", { isInternal: true }),
    occur: method.update("post", { isInternal: true }),
    act: method.mutation({
        permission: "read",
        input: schema.object({
            /** The declared action's name. */
            action: NotificationName,
            /** The text the action asks for. */
            text: schema.string().min(1).exactOptional(),
        }),
    }),
    notify: method.mutation({
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
            /** The principals it notifies. */
            recipients: schema.array(Subject).max(NOTIFY_RECIPIENTS),
            /** Why they receive it. */
            reason: schema.enum(REASONS),
            /** The values its content renders. */
            payload: schema.json(),
            /** The identity a later notice with the same key replaces. */
            key: NotificationKey.exactOptional(),
            /** The thread, the source's identifier by default. */
            thread: NotificationKey.exactOptional(),
        }),
        output: schema.object({}),
    }),
}));

/** An activity as its table stores it. */
export type Activity = Select<typeof activity.table>;

/** A person's notification of an activity in their home. */
export const notification = defineObject({
    name: "notification",
    plural: "notifications",
    scope: space,
    fields: {
        /** The activity it shows. */
        source: field.reference(activity, { qualified: true }),

        // the activity's fields, kept current
        /** The person it is for. */
        recipient: field.subject().personal(),
        /** The space of the activity. */
        space: field.string(schema.identifier("space")),
        /** The package declaring the notification. */
        packageId: field.string(PackageId),
        /** The release of the declaring package, whose preference setting it follows. */
        release: field.json(Package),
        /** The notification's name in its package. */
        name: field.string(NotificationName),
        /** The thread it groups in. */
        thread: field.string(NotificationKey),
        /** Why the recipient hears of it. */
        reason: field.enum(REASONS),
        /** The occurrences collapsed into it. */
        count: field.integer(),
        /** When it last occurred. */
        occurredAt: field.time(),
        /** The text every channel shows, rendered in its recipient's locale. */
        content: field.json(Wording),
        /** The line a summary shows for it, rendered in its recipient's locale. */
        summary: field.string(),
        /** The actions offered beside it, by name. */
        actions: field.json(schema.record(NotificationName, ActionMetadata)),
        /** How strongly it interrupts. */
        interruption: field.enum(INTERRUPTION_LEVELS),
        /** The preference its recipient starts from. */
        preference: field.json(Preference),

        // person state a new occurrence clears
        /** When the person read it, absent while unread. */
        readAt: field.time().optional(),
        /** When its snooze ends, absent unless snoozed. */
        snoozedUntil: field.time().optional(),
        /** When the person archived it, absent while in the inbox. */
        archivedAt: field.time().optional(),
        /** When its deliveries were planned, absent until its controller plans them. */
        plannedAt: field.time().optional(),
    },
    projected: {
        from: () => activity,
        to: "recipient",
        source: "source",
        fields: {
            recipient: "recipient",
            space: "scope",
            packageId: "packageId",
            release: "release",
            name: "name",
            thread: "thread",
            reason: "reason",
            count: "count",
            occurredAt: "occurredAt",
            content: "content",
            summary: "summary",
            actions: "actions",
            interruption: "interruption",
            preference: "preference",
        },
        clears: ["readAt", "snoozedUntil", "archivedAt", "plannedAt"],
        residence: { account, residence },
    },
    permissions: { read: relation("recipient"), deliver: none() },
    constraints: {
        thread: { kind: "index", on: ["scope", "recipient", "thread"] },
        occurred: { kind: "index", on: ["scope", "recipient", "occurredAt"] },
    },
})
    .methods((method) => ({
        get: method.get("read"),
        list: method.list("read"),
        // mark read and keep the first read time
        read: method.mutation({ permission: "read", inverse: "unread" }),
        // mark unread
        unread: method.mutation({ permission: "read", inverse: "read" }),
        readAll: method.updateMany("read", { fields: ["readAt"], match: ["thread", "readAt"] }),

        // defer until a time in the future
        snooze: method.mutation({
            permission: "read",
            input: schema.object({
                /** When the snooze ends, in UTC epoch milliseconds. */
                until: Instant,
            }),
        }),

        // move out of the inbox until it occurs again, or back
        archive: method.mutation({ permission: "read", inverse: "unarchive" }),
        unarchive: method.mutation({ permission: "read", inverse: "archive" }),

        // plan the deliveries of each channel
        plan: method.mutation({
            permission: "deliver",
            isInternal: true,
            output: schema.object({}),
        }),
    }))
    .implement({
        methods: {
            read: async (call) =>
                call.target.readAt === null ? call.update({ readAt: call.now }) : call.target,
            unread: async (call) => call.update({ readAt: null }),
            snooze: async (call) => {
                // require a snooze ending later
                if (call.input.until <= call.now) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: "a snooze ends in the future",
                    });
                }

                return call.update({ snoozedUntil: call.input.until });
            },
            archive: async (call) => call.update({ archivedAt: call.now }),
            unarchive: async (call) => call.update({ archivedAt: null }),
        },
    });

/** A notification as its table stores it. */
export type Notification = Select<typeof notification.table>;
