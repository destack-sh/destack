import { intersection, relation, through } from "@destack/access";
import { check, index, sql, unique, type Select } from "@destack/db";
import { defineObject, field, type ObjectType } from "@destack/object";
import { Package, PackageId } from "@destack/package";
import { defineSchema, Instant, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { address, user } from "@destack/account/object";
import { space } from "@destack/space/object";
import { ActionMetadata, Content } from "../notification/content.ts";
import { INTERRUPTION_LEVELS, Preference } from "../preference/preference.ts";
import { REASONS } from "./subscription.ts";

/** The most recipients one notify call names: 100 inserts of about 0.3 ms. */
export const NOTIFY_RECIPIENTS = 100;

/** The longest key or thread. */
const KEY_LENGTH = 200;

/** A notification's name, one segment of its preference setting's name. */
export const NotificationName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*$(?![\s\S])/u),
);

/** A notification key or thread. */
export const NotificationKey = defineSchema(schema.string().min(1).max(KEY_LENGTH));

/** What happened to an object, for one person: the declared notification it is, why they hear of it, and its rendered text. */
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
        /** The identity a later notice with the same key replaces, absent for one never replaced. */
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
        content: field.json(Content),
        /** The line a summary shows for it, such as "3 mentions", rendered in its recipient's locale. */
        summary: field.string(),
        /** The actions offered beside it, by name. */
        actions: field.json(schema.record(NotificationName, ActionMetadata)),
        /** How strongly it interrupts. */
        interruption: field.enum(INTERRUPTION_LEVELS),
        /** The preference its recipient starts from. */
        preference: field.json(Preference),
    },
    constraints: (entry) => [
        unique("activity_key").on(
            entry.scope,
            entry.recipient,
            entry.packageId,
            entry.name,
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.key,
        ),
        index("activity_thread").on(
            entry.scope,
            entry.recipient,
            entry.thread,
            entry.packageId,
            entry.name,
        ),
        check("activity_count", sql`${entry.count} >= 1`),
    ],
    projections: (): readonly ObjectType[] => [notification],
    permissions: {
        // the recipient reads it through its source
        read: intersection(relation("recipient"), through("parent", "read")),
    },
    // keep an activity 180 days, as GitHub keeps notifications five months
    expiring: [{ after: { days: 180 }, from: "occurredAt" }],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        post: method.create(null, { isSystem: true }),
        occur: method.update(null, { isSystem: true }),
        act: method.mutation({
            permission: "read",
            input: schema.object({
                /** The declared action's name. */
                action: NotificationName,
                /** The text the action asks for. */
                text: schema.string().min(1).exactOptional(),
            }),
        }),
    }),
});

/** An activity as its table stores it. */
export type Activity = Select<typeof activity.table>;

/** A person's notification of an activity in their home: the activity as they see it, with their read state and deliveries. */
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
        content: field.json(Content),
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
        residence: { user, address },
    },
    constraints: (entry) => [
        index("notification_thread").on(entry.scope, entry.recipient, entry.thread),
        index("notification_occurred").on(entry.scope, entry.recipient, entry.occurredAt),
    ],
    permissions: { read: relation("recipient") },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        // mark read and keep the first read time
        read: method
            .mutation({ permission: "read", inverse: "unread" })
            .handle(async (call) =>
                call.target.readAt === null ? call.update({ readAt: call.now }) : call.target,
            ),
        // mark unread
        unread: method
            .mutation({ permission: "read", inverse: "read" })
            .handle(async (call) => call.update({ readAt: null })),
        readAll: method.updateMany("read", { fields: ["readAt"], match: ["thread", "readAt"] }),

        // defer until a time in the future
        snooze: method
            .mutation({
                permission: "read",
                input: schema.object({
                    /** When the snooze ends, in UTC epoch milliseconds. */
                    until: Instant,
                }),
            })
            .handle(async (call) => {
                // require a snooze ending later
                if (call.input.until <= call.now) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: "a snooze ends in the future",
                    });
                }

                return call.update({ snoozedUntil: call.input.until });
            }),

        // move out of the inbox until it occurs again, or back
        archive: method
            .mutation({ permission: "read", inverse: "unarchive" })
            .handle(async (call) => call.update({ archivedAt: call.now })),
        unarchive: method
            .mutation({ permission: "read", inverse: "archive" })
            .handle(async (call) => call.update({ archivedAt: null })),
        deliver: method.mutation({ permission: null, isSystem: true }),
    }),
});

/** A notification as its table stores it. */
export type Notification = Select<typeof notification.table>;
