import { through } from "@destack/access";
import { check, index, sql, uniqueIndex, type Select } from "@destack/db";
import { Condition } from "@destack/db/query";
import { defineObject, field, method } from "@destack/object";
import { defineSchema, identifier, schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { notification } from "./notification.ts";

/** The channels that alert beside the inbox. */
export const CHANNELS = ["desktop", "push", "email"] as const;

/** A channel that alerts. */
export type Channel = (typeof CHANNELS)[number];

/** The states of a delivery. */
export const DELIVERY_STATES = ["pending", "sent", "skipped", "failed"] as const;

/** The reasons a delivery is skipped. */
export const SKIP_REASONS = [
    "withheld",
    "read",
    "unaddressed",
    "preference",
    "present",
    "gone",
] as const;

/** Why a delivery is skipped. */
export type SkipReason = (typeof SKIP_REASONS)[number];

/** Why a channel refused a delivery. */
export const DeliveryError = defineSchema(
    schema.object({
        /** The channel's code, such as an HTTP status. */
        code: schema.string().min(1),
        /** The channel's message. */
        message: schema.string(),
    }),
);
/** Why a channel refused a delivery. */
export type DeliveryError = schema.Infer<typeof DeliveryError>;

/** A message a desktop shows as a banner. */
export const Banner = defineSchema(
    schema.object({
        /** The headline. */
        title: schema.string(),
        /** The line below the headline. */
        subtitle: schema.string().optional(),
        /** The text. */
        body: schema.string(),
        /** The actions offered beside it. */
        actions: schema.array(
            schema.object({
                /** The action's declared name. */
                name: schema.string().min(1),
                /** The button's label. */
                title: schema.string(),
                /** Whether the action destroys or declines. */
                isDestructive: schema.boolean(),
                /** The text field the action asks for. */
                text: schema
                    .object({ placeholder: schema.string(), button: schema.string() })
                    .optional(),
            }),
        ),
    }),
);
/** A message a desktop shows as a banner. */
export type Banner = schema.Infer<typeof Banner>;

/** One notification sent on one channel to one address. */
export const delivery = defineObject({
    name: "delivery",
    plural: "deliveries",
    scope: space,
    nested: { in: notification, receive: "read" },
    fields: {
        /** The channel. */
        channel: field.enum(CHANNELS),
        /** The push endpoint, absent on other channels. */
        endpoint: field.string(identifier("push-endpoint")).optional(),
        /** The device with the desktop that shows it, absent on other channels. */
        device: field.string(identifier("device")).optional(),

        // schedule
        /** The state. */
        state: field.enum(DELIVERY_STATES),
        /** When it is due. */
        dueAt: field.time(),
        /** Whether it goes out in a summary. */
        isSummarized: field.boolean().default(false),

        // attempts
        /** The refused attempts so far. */
        attempts: field.integer().default(0),
        /** When it was sent. */
        sentAt: field.time().optional(),
        /** Why it was skipped. */
        reason: field.enum(SKIP_REASONS).optional(),
        /** The channel's last refusal. */
        error: field.json(DeliveryError).optional(),
        /** The banner a desktop shows, once sent there. */
        banner: field.json(Banner).optional(),
    },
    constraints: (delivery) => [
        uniqueIndex("delivery_target").on(
            delivery.parentId,
            delivery.channel,
            sql`coalesce(${delivery.endpoint}, ${delivery.device}, '')`,
        ),
        index("delivery_due").on(delivery.scope, delivery.state, delivery.dueAt),
        check("delivery_attempts", sql`${delivery.attempts} >= 0`),
        check(
            "delivery_endpoint",
            sql`(${delivery.channel} = 'push') = (${delivery.endpoint} IS NOT NULL)`,
        ),
        check(
            "delivery_device",
            sql`(${delivery.channel} = 'desktop') = (${delivery.device} IS NOT NULL)`,
        ),
        check(
            "delivery_banner",
            sql`(${delivery.channel} = 'desktop' AND ${delivery.state} = 'sent') = (${delivery.banner} IS NOT NULL)`,
        ),
        check(
            "delivery_sent",
            sql`(${delivery.state} = 'sent') = (${delivery.sentAt} IS NOT NULL)`,
        ),
        check(
            "delivery_skipped",
            sql`(${delivery.state} = 'skipped') = (${delivery.reason} IS NOT NULL)`,
        ),
        check(
            "delivery_failed",
            sql`${delivery.state} <> 'failed' OR ${delivery.error} IS NOT NULL`,
        ),
    ],
    permissions: { read: through("parent", "read") },
    // keep a finished delivery 7 days
    expiring: [{ after: { days: 7 }, from: "updatedAt", where: Condition.ne("state", "pending") }],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        schedule: method.create(null, { isSystem: true }),
        record: method.update(null, { isSystem: true }),
        discard: method.delete(null, { isSystem: true }),
        send: method({ permission: null, isSystem: true }),
    },
});

/** A delivery as its table stores it. */
export type Delivery = Select<typeof delivery.table>;
