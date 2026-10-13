import { space } from "@destack/account/object";
import { none, through } from "@destack/access";
import { type Select } from "@destack/db";
import { MessageError } from "@destack/message";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";

import { CHANNELS } from "../preference/preference.ts";
import { notification } from "./activity.ts";

/** The states of a delivery: waiting, handed to a message, and settled. */
export const DELIVERY_STATES = ["pending", "sending", "sent", "skipped", "failed"] as const;

/** The reasons a delivery is skipped. */
export const SKIP_REASONS = ["read", "unaddressed", "preference", "gone"] as const;

/** Why a delivery is skipped. */
export type SkipReason = (typeof SKIP_REASONS)[number];

/** The activity a notification shows, in the space it happened in. */
export const ActivityReference = defineSchema(
    schema.object({
        /** The activity's space. */
        scope: schema.identifier("space"),
        /** The activity. */
        id: schema.identifier("activity"),
    }),
);
/** The activity a notification shows. */
export type ActivityReference = schema.Infer<typeof ActivityReference>;

/** One notification on one channel to one address: shown on a desktop, or sent as a message. */
export const delivery = defineObject({
    name: "delivery",
    plural: "deliveries",
    scope: space,
    nested: { in: notification, receive: "read" },
    fields: {
        /** The channel. */
        channel: field.enum(CHANNELS),
        /** The client showing it on its desktop or receiving its push, absent for email. */
        client: field.string(schema.identifier("client")).optional(),

        // schedule
        /** The state. */
        state: field.enum(DELIVERY_STATES),
        /** When it is due. */
        dueAt: field.time(),
        /** Whether it goes out in a summary. */
        isSummarized: field.boolean().default(false),

        // outcome
        /** The message sending it by email or push, once handed over. */
        message: field.string(schema.identifier("message")).optional(),
        /** When it was sent. */
        sentAt: field.time().optional(),
        /** Why it was skipped. */
        reason: field.enum(SKIP_REASONS).optional(),
        /** Why its message failed. */
        error: field.json(MessageError).optional(),
        /** When its desktop client received it, absent until then. */
        receivedAt: field.time().optional(),
    },
    permissions: { read: through("parent", "read"), deliver: none() },
    // keep a finished delivery 7 days
    expiring: [
        {
            after: { days: 7 },
            from: "updatedAt",
            where: { state: { notIn: ["pending", "sending"] } },
        },
    ],
    constraints: {
        target: {
            kind: "index",
            on: ["parentId", "channel", "client"],
            unique: true,
            nullsNotDistinct: true,
        },
        due: { kind: "index", on: ["scope", "state", "dueAt"] },
        message: { kind: "index", on: ["message"] },
        client: {
            kind: "check",
            condition: {
                OR: [
                    { channel: { in: ["desktop", "push"] }, client: { isNotNull: true } },
                    { channel: { notIn: ["desktop", "push"] }, client: { isNull: true } },
                ],
            },
        },
        sending: {
            kind: "check",
            condition: {
                OR: [{ state: { ne: "sending" } }, { message: { isNotNull: true } }],
            },
        },
        sent: {
            kind: "check",
            condition: {
                OR: [
                    { state: "sent", sentAt: { isNotNull: true } },
                    { state: { ne: "sent" }, sentAt: { isNull: true } },
                ],
            },
        },
        skipped: {
            kind: "check",
            condition: {
                OR: [
                    { state: "skipped", reason: { isNotNull: true } },
                    { state: { ne: "skipped" }, reason: { isNull: true } },
                ],
            },
        },
        failed: {
            kind: "check",
            condition: {
                OR: [{ state: { ne: "failed" } }, { error: { isNotNull: true } }],
            },
        },
        received: {
            kind: "check",
            condition: {
                OR: [{ receivedAt: { isNull: true } }, { channel: "desktop", state: "sent" }],
            },
        },
    },
}).methods((method) => ({
    get: method.get("read"),
    list: method.list("read"),
    schedule: method.create("deliver", { isInternal: true }),
    record: method.update("deliver", { isInternal: true }),
    discard: method.delete("deliver", { isInternal: true }),
    send: method.mutation({
        permission: "deliver",
        isInternal: true,
        output: schema.object({}),
    }),
    settle: method.mutation({
        permission: "deliver",
        isInternal: true,
        output: schema.object({}),
    }),
    /** Record its desktop client receiving it, keeping the first time. */
    receive: method.mutation({ permission: "read" }),
}));

/** A delivery as its table stores it. */
export type Delivery = Select<typeof delivery.table>;
