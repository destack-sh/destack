import { through } from "@destack/access";
import { check, index, sql, uniqueIndex, type Select } from "@destack/db";
import { MessageError } from "@destack/message";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { space } from "@destack/space/object";
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
        /** The push endpoint, absent on other channels. */
        endpoint: field.string(schema.identifier("push-endpoint")).optional(),
        /** The device with the desktop that shows it, absent on other channels. */
        device: field.string(schema.identifier("device")).optional(),

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
    },
    constraints: (entry) => [
        uniqueIndex("delivery_target").on(
            entry.parentId,
            entry.channel,
            sql`coalesce(${entry.endpoint}, ${entry.device}, '')`,
        ),
        index("delivery_due").on(entry.scope, entry.state, entry.dueAt),
        index("delivery_message").on(entry.message),
        check(
            "delivery_endpoint",
            sql`(${entry.channel} = 'push') = (${entry.endpoint} IS NOT NULL)`,
        ),
        check(
            "delivery_device",
            sql`(${entry.channel} = 'desktop') = (${entry.device} IS NOT NULL)`,
        ),
        check("delivery_sending", sql`${entry.state} <> 'sending' OR ${entry.message} IS NOT NULL`),
        check("delivery_sent", sql`(${entry.state} = 'sent') = (${entry.sentAt} IS NOT NULL)`),
        check(
            "delivery_skipped",
            sql`(${entry.state} = 'skipped') = (${entry.reason} IS NOT NULL)`,
        ),
        check("delivery_failed", sql`${entry.state} <> 'failed' OR ${entry.error} IS NOT NULL`),
    ],
    permissions: { read: through("parent", "read") },
    // keep a finished delivery 7 days
    expiring: [
        {
            after: { days: 7 },
            from: "updatedAt",
            where: { state: { notIn: ["pending", "sending"] } },
        },
    ],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        schedule: method.create(null, { isSystem: true }),
        record: method.update(null, { isSystem: true }),
        discard: method.delete(null, { isSystem: true }),
        send: method.mutation({ permission: null, isSystem: true, output: schema.object({}) }),
        settle: method.mutation({ permission: null, isSystem: true, output: schema.object({}) }),
    }),
});

/** A delivery as its table stores it. */
export type Delivery = Select<typeof delivery.table>;
