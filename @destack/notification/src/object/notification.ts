import { none, through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import type { Message } from "@destack/message/object";
import { defineObject, field } from "@destack/object";
import { Identifier, Instant, schema } from "@destack/schema";
import { NotificationError } from "../error/error.ts";
import { activity } from "./activity.ts";
import { ObjectReference, type ReferenceColumns } from "@destack/sync";
import { expiring } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A person's state of one activity in their home, beside the activity's replica. */
export const notification = defineObject(
    {
        name: "notification",
        plural: "notifications",
        title: t`${plural(argument("count"), { one: "Notification", other: "Notifications" })}`,
        scope: space,
        key: "activityId",
        fields: {
            /** The activity whose state it stores, its replica leaving with a revoked share. */
            activity: field.reference(activity, { delete: "keep" }).immutable(),
            /** When the person last read it, unread while before the activity's last occurrence. */
            readAt: field.time().optional(),
            /** When its snooze ends. */
            snoozedUntil: field.time().optional(),
            /** When the person last archived it, back in the inbox once the activity recurs. */
            archivedAt: field.time().optional(),
            /** The occurrence whose delivery it planned last, as its activity's occurrence time. */
            plannedAt: field.time(),
            /** When its delivery decides again, absent once every destination has it. */
            deliveryAt: field.time().optional(),
            /** The destinations its last occurrence reached or skipped, by `Destination.key`. */
            delivered: field.json(schema.array(schema.string().min(1))).default([]),
            /** The scheduled summary it waits for, absent outside one. */
            summaryAt: field.time().optional(),
        },
        constraints: {
            delivery: { kind: "index", on: ["deliveryAt"] },
            summary: { kind: "index", on: ["scope", "summaryAt"] },
        },
        access: {
            permissions: { read: through("scope", "read"), plan: none() },
        },
        traits: [
            // expire a notification 180 days after it was planned, past its activity's expiry
            expiring([{ after: { days: 180 }, from: "plannedAt" }]),
        ],
    },
    (method) => ({
        /** Create the notification of an activity replica. */
        create: method.create("plan", {
            visibility: "package",
            fields: ["activityId", "plannedAt", "deliveryAt"],
        }),
        /** Plan its delivery and the summary it waits for, as its controllers do. */
        update: method.update("plan", {
            visibility: "package",
            fields: ["plannedAt", "deliveryAt", "delivered", "summaryAt"],
        }),
        /** Deliver its last occurrence to each destination due now, deferring the others. */
        deliver: method.mutation({ title: t`Deliver`, permission: "plan", visibility: "package" }),
        /** Move every notification waiting for one summary to another, or out of any. */
        reschedule: method.updateMany("plan", {
            visibility: "package",
            fields: ["summaryAt"],
            match: ["summaryAt"],
        }),

        /** Mark it read now. */
        read: method.mutation({ title: t`Read`, permission: "read", inverse: "unread" }),
        /** Mark it unread. */
        unread: method.mutation({ title: t`Mark unread`, permission: "read", inverse: "read" }),
        /** Mark listed notifications read as of a time, each at its revision. */
        readMany: method.updateItems("read", { fields: ["readAt"] }),

        /** Defer it until a time in the future. */
        snooze: method.mutation({
            title: t`Snooze`,
            permission: "read",
            input: schema.object({
                /** When the snooze ends, in UTC epoch milliseconds. */
                until: Instant,
            }),
        }),

        /** Move it out of the inbox until the activity occurs again. */
        archive: method.mutation({ title: t`Archive`, permission: "read", inverse: "unarchive" }),
        /** Move it back into the inbox. */
        unarchive: method.mutation({ title: t`Unarchive`, permission: "read", inverse: "archive" }),
    }),
).implement({
    methods: {
        read: async (call) => call.update({ readAt: call.now }),
        unread: async (call) => call.update({ readAt: null }),
        readMany: async (call, next) => {
            // refuse a read in the future
            if (call.input.items.some((item) => (item.readAt ?? call.now) > call.now)) {
                throw NotificationError.of(
                    "READ_IN_FUTURE_INVALID",
                    "read time is invalid: a notification cannot be read in the future",
                );
            }

            return next();
        },
        snooze: async (call) => {
            // require a snooze ending later
            if (call.input.until <= call.now) {
                throw NotificationError.of(
                    "SNOOZE_IN_PAST_INVALID",
                    "snooze end is invalid: a snooze cannot end in the past",
                );
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
    /** Take the notification a message was sent for, absent for another source. */
    from(
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
};
