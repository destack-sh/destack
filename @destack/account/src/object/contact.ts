import { defineSchema, schema, TimeZone } from "@destack/schema";
import { LocaleTag } from "@destack/locale";

/** The longest push service URL a browser hands out. */
const PUSH_URL_LENGTH = 2048;

/** A browser's Web Push subscription, as the Push API hands it out (RFC 8030, RFC 8291). */
export const PushSubscription = defineSchema(
    schema.object({
        /** The push service URL the browser receives pushes at. */
        url: schema.url({ protocol: /^https$/u }).max(PUSH_URL_LENGTH),
        /** The browser's keys encrypting each push, base64url encoded. */
        keys: schema.object({
            /** The browser's uncompressed P-256 public key. */
            p256dh: schema.string().min(1),
            /** The authentication secret. */
            auth: schema.string().min(1),
        }),
    }),
);
/** A browser's Web Push subscription. */
export type PushSubscription = schema.Infer<typeof PushSubscription>;

/** Where a person receives notifications beyond an inbox, and in which language. */
export const Contact = defineSchema(
    schema.object({
        /** The desktop clients showing the person's notifications. */
        desktops: schema.array(schema.identifier("client")),
        /** The browser clients receiving the person's pushes, with their subscriptions. */
        browsers: schema.array(PushSubscription.extend({ id: schema.identifier("client") })),
        /** The person's verified email address, absent without one. */
        email: schema.email().exactOptional(),
        /** The IANA time zone the person lives in. */
        timeZone: TimeZone,
        /** The language and region the person reads. */
        locale: LocaleTag,
    }),
);
/** Where a person receives notifications beyond an inbox, and in which language. */
export type Contact = schema.Infer<typeof Contact>;
