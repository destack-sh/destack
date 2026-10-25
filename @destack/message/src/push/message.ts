import { schema } from "@destack/schema";

/** The most payload bytes a push carries, 4096 less header, tag and delimiter (RFC 8291 4). */
export const PUSH_DATA_BYTES = 3993;

/** The bytes of an uncompressed P-256 public key, its 0x04 prefix included. */
export const PUBLIC_KEY_BYTES = 65;

/** The urgency a push service delivers a push with, after RFC 8030. */
export const URGENCIES = ["very-low", "low", "normal", "high"] as const;

/** The browser client a push goes to, whose subscription the person's account keeps. */
export const PushDestination = schema.object({
    /** The push channel. */
    channel: schema.literal("push"),
    /** The browser's client. */
    client: schema.identifier("client"),
    /** The person the client belongs to. */
    user: schema.identifier("user"),
});

/** A push's payload and how its push service delivers it. */
export const PushContent = schema.object({
    /** The push channel. */
    channel: schema.literal("push"),
    /** The payload the service worker receives, as JSON within a push's bytes. */
    data: schema
        .json()
        .refine(
            (data) => new TextEncoder().encode(JSON.stringify(data)).length <= PUSH_DATA_BYTES,
            `at most ${PUSH_DATA_BYTES} bytes of JSON`,
        ),
    /** How urgently the push service delivers it. */
    urgency: schema.enum(URGENCIES),
    /** How long the push service stores it for an offline browser, in seconds. */
    ttl: schema.number().int().nonnegative(),
    /** The topic a later push with the same one replaces, after RFC 8030. */
    topic: schema
        .string()
        .regex(/^[A-Za-z0-9_-]{1,32}$/u)
        .exactOptional(),
});
