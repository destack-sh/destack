import { defineSchema, present, schema, TimeZone } from "@destack/schema";
import { LocaleTag } from "@destack/locale";
import type { Client } from "./client.ts";
import type { User } from "./user.ts";

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

/** The schema of a recipient. */
const RecipientSchema = defineSchema(
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
export const Recipient = Object.assign(RecipientSchema, {
    /** Read where a person receives notifications from their profile and unrevoked clients. */
    of(
        person: Pick<User, "email" | "emailVerified" | "timeZone" | "locale">,
        clients: readonly Pick<Client, "id" | "kind" | "push" | "revokedAt">[],
    ): Recipient {
        const live = clients.filter((client) => client.revokedAt === null);

        return {
            desktops: live.filter((client) => client.kind === "native").map((client) => client.id),
            browsers: live.flatMap((client) =>
                client.kind === "browser" && client.push !== null
                    ? [{ id: client.id, ...client.push }]
                    : [],
            ),
            ...(person.emailVerified === true && person.email !== null
                ? { email: person.email }
                : {}),
            timeZone: present(person.timeZone, "the person's time zone"),
            locale: person.locale,
        };
    },
});
/** Where a person receives notifications beyond an inbox, and in which language. */
export type Recipient = schema.Infer<typeof RecipientSchema>;
