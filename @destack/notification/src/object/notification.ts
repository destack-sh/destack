import { none, through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { Destination, type Message, message } from "@destack/message";
import { defineObject, field } from "@destack/object";
import { defineWorkflow, ScheduledCall } from "@destack/workflow";
import { Identifier, Instant, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { ObjectReference, type ReferenceColumns } from "@destack/sync";
import { Decision } from "../message/decide.ts";
import type { Activity } from "./activity.ts";

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
            /** The scheduled summary it waits for, absent outside one. */
            summaryAt: field.time().optional(),
        },
        permissions: { read: through("scope", "read"), plan: none() },
        // keep a notification as long as its activity
        expiring: [{ after: { days: 180 }, from: "plannedAt" }],
        constraints: { summary: { kind: "index", on: ["scope", "summaryAt"] } },
    },
    (method) => ({
        /** Create the notification of an activity replica. */
        create: method.create("plan", { isInternal: true, fields: ["plannedAt"] }),
        /** Start the delivery of the activity's last occurrence. */
        plan: method.mutation({ permission: "plan", isInternal: true }),
        /** Decide what each waiting destination of the person does with the occurrence now. */
        decide: method.query({
            permission: "plan",
            isInternal: true,
            input: schema.object({
                /** The occurrence the delivery is of, which a later one replaces. */
                occurredAt: Instant,
                /** The destination keys still waiting, every destination when absent. */
                destinations: schema.array(schema.string().min(1)).exactOptional(),
            }),
            output: schema.object({
                /** One decision per destination. */
                items: schema.array(
                    schema.object({
                        /** The destination's key, as `Destination.key` writes it. */
                        key: schema.string().min(1),
                        /** The destination. */
                        to: Destination,
                        /** What it does now. */
                        decision: Decision,
                    }),
                ),
            }),
        }),
        /** Send the occurrence to one destination now. */
        send: method.mutation({
            permission: "plan",
            isInternal: true,
            input: schema.object({
                /** The destination. */
                to: Destination,
            }),
            output: schema.object({
                /** The message sent. */
                message: schema.identifier("message"),
            }),
        }),
        /** Keep the occurrence for a scheduled summary, starting the summary's run once. */
        schedule: method.mutation({
            permission: "plan",
            isInternal: true,
            input: schema.object({
                /** When the summary goes out. */
                at: Instant,
            }),
        }),
        /** Send the scheduled summary due at a time to each summary destination. */
        summarize: method.mutation({
            permission: "plan",
            target: "scope",
            isInternal: true,
            input: schema.object({
                /** When the summary goes out. */
                at: Instant,
            }),
            output: schema.object({}),
        }),
        /** Move every notification waiting for one summary to another, or out of any. */
        reschedule: method.updateMany("plan", {
            isInternal: true,
            fields: ["summaryAt"],
            match: ["summaryAt"],
        }),
        /** Forget a browser whose push service forgot its subscription. */
        forget: method.mutation({
            permission: "plan",
            isInternal: true,
            input: schema.object({
                /** The browser's push endpoint. */
                url: schema.url(),
            }),
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

/** Deliver one occurrence of a notification to its person's destinations, in rounds. */
export const deliver = defineWorkflow({
    name: "deliver",
    input: schema.object({
        /** The notification. */
        notification: schema.identifier("notification"),
        /** The occurrence, which a later one replaces. */
        occurredAt: Instant,
    }),
    waits: [message],
});

/** Send a home's scheduled summary due at a time. */
export const summarize = defineWorkflow({
    name: "summarize",
    input: schema.object({
        /** When the summary goes out. */
        at: Instant,
    }),
});

/** The calls notifications and their actions make once at their target's home, as the caller. */
export const scheduled = defineWorkflow({
    name: "call",
    input: ScheduledCall.input,
    output: ScheduledCall.output,
});

/** A notification as its table stores it. */
export type Notification = Select<typeof notification.table>;

/** The notification of an activity, and the person's state it derives against the activity. */
export const Notification = {
    /** Derive the identifier of an activity's notification, which shares its UUID. */
    of(activityId: Identifier<"activity">): Identifier<"notification"> {
        return Identifier.derive("notification", Identifier.uuid(activityId));
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
