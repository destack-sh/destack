import { schema } from "@destack/schema";

/** The urgency a push service delivers a push with, after RFC 8030. */
export const URGENCIES = ["very-low", "low", "normal", "high"] as const;

/** A browser's keys encrypting a push's payload, after RFC 8291. */
export const PushKeys = schema.object({
    /** The browser's P-256 public key, base64url. */
    p256dh: schema.string().min(1),
    /** The browser's authentication secret, base64url. */
    auth: schema.string().min(1),
});
/** A browser's keys encrypting a push's payload. */
export type PushKeys = schema.Infer<typeof PushKeys>;

/** The browser's Web Push endpoint a message goes to. */
export const PushRecipient = schema.object({
    channel: schema.literal("push"),
    /** The push service's endpoint URL. */
    url: schema.url({ protocol: /^https$/u }),
    /** The browser's keys encrypting the payload. */
    keys: PushKeys,
});

/** A push's payload and how its push service delivers it. */
export const PushContent = schema.object({
    channel: schema.literal("push"),
    /** The payload the service worker receives. */
    data: schema.json(),
    /** How urgently the push service delivers it. */
    urgency: schema.enum(URGENCIES),
    /** How long the push service keeps it for an offline browser, in seconds. */
    ttl: schema.number().int().nonnegative(),
    /** The topic a later push with the same one replaces, after RFC 8030. */
    topic: schema
        .string()
        .regex(/^[A-Za-z0-9_-]{1,32}$/u)
        .exactOptional(),
});
