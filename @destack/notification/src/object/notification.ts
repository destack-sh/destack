import { condition, intersection, relation, through, union } from "@destack/access";
import { check, index, sql, unique, type Select } from "@destack/db";
import { Condition } from "@destack/db/query";
import { defineObject, field, method } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { space } from "@destack/space/object";
import { REASONS } from "./subscription.ts";

/** The most recipients one notify call names: 100 inserts of about 0.3 ms. */
export const NOTIFY_RECIPIENTS = 100;

/** The longest key or thread. */
const KEY_LENGTH = 200;

/** A notification's name, one segment of its preference setting's name. */
export const NotificationName = defineSchema(
    schema.string().regex(/^[a-z][a-zA-Z0-9]*$(?![\s\S])/),
);

/** A notification key or thread. */
export const NotificationKey = defineSchema(schema.string().min(1).max(KEY_LENGTH));

/** Mark a notification read, keeping the first read time. */
const read = method({ permission: "read", inverse: "unread" }).handle(async (call) => {
    const { readAt } = call.target as { readonly readAt: number | null };

    return readAt === null ? call.revise({ readAt: call.now }) : call.target;
});

/** Mark a notification unread. */
const unread = method({ permission: "read", inverse: "read" }).handle(async (call) =>
    call.revise({ readAt: null }),
);

/** Hold a notification back until a time. */
const snooze = method({
    permission: "read",
    input: schema.object({
        /** When the snooze ends, in UTC epoch milliseconds. */
        until: schema.number().int().nonnegative(),
    }),
}).handle(async (call) => {
    // require a snooze ending later
    const { until } = call.input as { readonly until: number };
    if (until <= call.now) {
        throw new ServiceError("BAD_REQUEST", { message: "a snooze ends in the future" });
    }

    return call.revise({ snoozedUntil: until });
});

/** One recipient's notification about one source object. */
export const notification = defineObject({
    name: "notification",
    plural: "notifications",
    scope: space,
    nested: { in: "any", receive: "notify" },
    fields: {
        /** The recipient. */
        recipient: field.subject().personal(),

        // declaration and identity
        /** The package declaring the notification. */
        packageId: field.string(PackageId),
        /** The notification's name in its package. */
        name: field.string(NotificationName),
        /** The identity a later notice with the same key replaces, absent for one never replaced. */
        key: field.string(NotificationKey).optional(),
        /** The thread it groups in. */
        thread: field.string(NotificationKey),

        // content
        /** Why the recipient receives it. */
        reason: field.enum(REASONS),
        /** The values its content renders. */
        payload: field.json(schema.json()),
        /** The occurrences collapsed into it. */
        count: field.integer().default(1),
        /** When it last occurred. */
        occurredAt: field.time(),

        // recipient state
        /** When the recipient read it, absent while unread. */
        readAt: field.time().optional(),
        /** When its snooze ends, absent unless snoozed. */
        snoozedUntil: field.time().optional(),
        /** When its deliveries were planned, absent until the dispatcher plans them. */
        plannedAt: field.time().optional(),
    },
    constraints: (notification) => [
        unique("notification_key").on(
            notification.scope,
            notification.recipient,
            notification.packageId,
            notification.name,
            notification.parentPackageId,
            notification.parentType,
            notification.parentId,
            notification.key,
        ),
        index("notification_thread").on(
            notification.scope,
            notification.recipient,
            notification.thread,
            notification.packageId,
            notification.name,
        ),
        check("notification_count", sql`${notification.count} >= 1`),
    ],
    permissions: {
        // the recipient reads it through its source, and its home copy
        read: union(
            intersection(relation("recipient"), through("parent", "read")),
            intersection(
                relation("recipient"),
                condition(Condition.not(Condition.missing("origin"))),
            ),
        ),
    },
    addressed: { recipient: "recipient" },
    expiring: [
        // keep read notifications 30 days, as GitHub's inbox keeps done ones
        { after: { days: 30 }, from: "readAt" },
        // keep unread ones 180 days, as GitHub keeps notifications five months
        { after: { days: 180 }, from: "occurredAt", where: Condition.missing("readAt") },
    ],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        read,
        unread,
        readAll: method.updateMany("read", { fields: ["readAt"], match: ["thread", "readAt"] }),
        snooze,
        post: method.create(null, { isSystem: true }),
        occur: method.update(null, { isSystem: true }),
        deliver: method({ permission: null, isSystem: true }),
    },
});

/** A notification as its table holds it. */
export type NotificationRow = Select<typeof notification.table>;
