import { client, type PushSubscription, user } from "@destack/account/object";
import { type Transaction } from "@destack/db";
import type { LocaleTag } from "@destack/locale";
import { type Identifier, present, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import type { Delivery } from "../object/delivery.ts";

/** Where and when a recipient receives notifications beyond the inbox, and in which language. */
export interface Contact {
    /** The desktop clients showing the recipient's notifications. */
    readonly desktops: readonly Identifier<"client">[];
    /** The browser clients receiving the recipient's pushes, with their subscriptions. */
    readonly browsers: readonly (PushSubscription & { readonly id: Identifier<"client"> })[];
    /** The recipient's verified email address, absent without one. */
    readonly email?: string;
    /** The IANA time zone the recipient lives in. */
    readonly timeZone: string;
    /** The language and region the recipient reads. */
    readonly locale: LocaleTag;
}

/** A recipient's contact, read from the inbox's copies of the recipient's scope. */
export const Contact = {
    /** Read a recipient's desktop and browser clients, verified email, time zone and locale. */
    async read(transaction: Transaction, recipient: Subject): Promise<Contact> {
        // read the recipient's profile and unrevoked clients
        const scope = schema.identifier("user").parse(recipient.id);
        const copy = await transaction.row(user.table, { id: scope });
        const profile = present(copy, `the user copy of recipient ${scope}`);
        const clients = await transaction.rows(client.table, {
            scope,
            revokedAt: { isNull: true },
        });

        return {
            desktops: clients.filter((row) => row.kind === "native").map((row) => row.id),
            browsers: clients.flatMap((row) =>
                row.kind === "browser" && row.push !== null ? [{ id: row.id, ...row.push }] : [],
            ),
            ...(profile.emailVerified === true && profile.email !== null
                ? { email: profile.email }
                : {}),
            timeZone: present(profile.timeZone, `the time zone of recipient ${scope}`),
            locale: present(profile.locale, `the locale of recipient ${scope}`),
        };
    },

    /** Report whether a contact still has the address a delivery goes to. */
    addresses(contact: Contact, delivery: Pick<Delivery, "channel" | "client">): boolean {
        // find the desktop client
        if (delivery.channel === "desktop") {
            return contact.desktops.some((known) => known === delivery.client);
        }
        // find the browser client
        else if (delivery.channel === "push") {
            return contact.browsers.some((known) => known.id === delivery.client);
        }
        // find the email address
        else {
            return contact.email !== undefined;
        }
    },
};
