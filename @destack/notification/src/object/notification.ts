import { none, through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select, type SQL, type SQLWrapper, sql } from "@destack/db";
import type { Message } from "@destack/message";
import { defineObject, field } from "@destack/object";
import { Identifier, Instant, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { ObjectReference, type ReferenceColumns } from "@destack/sync";
import type { Activity } from "./activity.ts";

/** The prefix of activity identifiers, ahead of their UUID. */
const ACTIVITY_PREFIX = "activity-";

/** The prefix of notification identifiers, ahead of the UUID they share with their activity. */
const NOTIFICATION_PREFIX = "notification-";

/** A person's state of one activity in their home, keyed by the activity's replica. */
export const notification = defineObject(
    {
        name: "notification",
        plural: "notifications",
        scope: space,
        fields: {
            /** When the person last read it, unread while before the activity's last occurrence. */
            readAt: field.time().optional(),
            /** When its snooze ends. */
            snoozedUntil: field.time().optional(),
            /** When the person last archived it, back in the inbox once the activity recurs. */
            archivedAt: field.time().optional(),
            /** When the delivery of its last occurrence started. */
            plannedAt: field.time(),
            /** When its delivery decides again, absent once every destination has it. */
            deliveryAt: field.time().optional(),
            /** The destinations its last occurrence reached or skipped, by `Destination.key`. */
            delivered: field.json(schema.array(schema.string().min(1))).default([]),
            /** The scheduled summary it waits for, absent outside one. */
            summaryAt: field.time().optional(),
        },
        permissions: { read: through("scope", "read"), plan: none() },
        // keep a notification as long as its activity
        expiring: [{ after: { days: 180 }, from: "plannedAt" }],
        constraints: {
            delivery: { kind: "index", on: ["deliveryAt"] },
            summary: { kind: "index", on: ["scope", "summaryAt"] },
        },
    },
    (method) => ({
        /** Create the notification of an activity replica. */
        create: method.create("plan", { isInternal: true, fields: ["plannedAt", "deliveryAt"] }),
        /** Plan its delivery and the summary it waits for, as its controllers do. */
        update: method.update("plan", {
            isInternal: true,
            fields: ["plannedAt", "deliveryAt", "delivered", "summaryAt"],
        }),
        /** Deliver its last occurrence to each destination due now, deferring the others. */
        deliver: method.mutation({ permission: "plan", isInternal: true }),
        /** Move every notification waiting for one summary to another, or out of any. */
        reschedule: method.updateMany("plan", {
            isInternal: true,
            fields: ["summaryAt"],
            match: ["summaryAt"],
        }),

        /** Mark it read now. */
        read: method.mutation({ permission: "read", inverse: "unread" }),
        /** Mark it unread. */
        unread: method.mutation({ permission: "read", inverse: "read" }),
        /** Mark every notification read. */
        readAll: method.updateMany("read", { fields: ["readAt"], match: [] }),

        /** Defer it until a time in the future. */
        snooze: method.mutation({
            permission: "read",
            input: schema.object({
                /** When the snooze ends, in UTC epoch milliseconds. */
                until: Instant,
            }),
        }),

        /** Move it out of the inbox until the activity occurs again. */
        archive: method.mutation({ permission: "read", inverse: "unarchive" }),
        /** Move it back into the inbox. */
        unarchive: method.mutation({ permission: "read", inverse: "archive" }),
    }),
).implement({
    methods: {
        read: async (call) => call.update({ readAt: call.now }),
        unread: async (call) => call.update({ readAt: null }),
        snooze: async (call) => {
            // require a snooze ending later
            if (call.input.until <= call.now) {
                throw new CallError("BAD_REQUEST", {
                    message: "a snooze cannot end in the past",
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

/** The notification of an activity, and the person's state it derives against the activity. */
export const Notification = {
    /** Derive the identifier of an activity's notification, which shares its UUID. */
    of(activityId: Identifier<"activity">): Identifier<"notification"> {
        return Identifier.derive("notification", Identifier.uuid(activityId));
    },

    /** Derive the notification identifier of an activity identifier column in SQL, as `of` does. */
    ofColumn(activityId: SQLWrapper): SQL<string> {
        const uuid = sql`substr(${activityId}, ${sql.raw(String(ACTIVITY_PREFIX.length + 1))})`;

        return sql<string>`${sql.raw(`'${NOTIFICATION_PREFIX}'`)} || ${uuid}`;
    },

    /** Derive the identifier of a notification's activity, which shares its UUID. */
    activity(notificationId: Identifier<"notification">): Identifier<"activity"> {
        return Identifier.derive("activity", Identifier.uuid(notificationId));
    },

    /** Read the notification a message was sent for, absent for another source. */
    read(
        sent: Pick<Message, keyof ReferenceColumns<"source">>,
    ): Identifier<"notification"> | undefined {
        const source = ObjectReference.at("source", sent);

        return notification.is(source) ? notification.identifier(source.id) : undefined;
    },

    /** Read a push topic from a notification's 32 hexadecimal digits, as RFC 8030 allows. */
    topic(id: string): string {
        return schema
            .string()
            .length(32)
            .parse(id.slice(id.indexOf("-") + 1).replaceAll("-", ""));
    },

    /** Report whether the delivery of the activity's last occurrence started. */
    isPlanned(
        state: Pick<Notification, "plannedAt"> | undefined,
        occurrence: Pick<Activity, "occurredAt">,
    ) {
        return state !== undefined && state.plannedAt >= occurrence.occurredAt;
    },
};
