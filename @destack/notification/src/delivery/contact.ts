import { device, pushEndpoint, type PushEndpoint, user } from "@destack/account/object";
import { locale } from "@destack/account/setting";
import { and, type DatabaseConnection, eq, isNull, Snapshot } from "@destack/db";
import type { LocaleTag } from "@destack/locale";
import { type Identifier, schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { setting, type SettingValue } from "@destack/setting/object";
import type { Subject } from "@destack/sync";
import type { Delivery } from "../object/delivery.ts";

/** The language a recipient reads who set none: the source language of the inbox's messages. */
const SOURCE_LOCALE: LocaleTag = "en";

/** The time zone of a recipient whose profile names none. */
const UNIVERSAL_TIME_ZONE = "UTC";

/** Where and when a recipient receives notifications beyond the inbox, and in which language. */
export interface Contact {
    /** The devices whose desktops show the recipient's notifications. */
    readonly desktops: readonly Identifier<"device">[];
    /** The push endpoints of the recipient's browsers. */
    readonly endpoints: readonly Pick<PushEndpoint, "id" | "url" | "keys">[];
    /** The recipient's verified email address, absent without one. */
    readonly email?: string;
    /** The IANA time zone the recipient's profile names, UTC without one. */
    readonly timeZone: string;
    /** The language and region the recipient reads, the source language while unset. */
    readonly locale: LocaleTag;
}

/** A recipient's contact, read from the inbox's copies of the recipient's scope. */
export const Contact = {
    /** Read a recipient's desktops, push endpoints, verified email, time zone and locale. */
    async read(database: DatabaseConnection, recipient: Subject): Promise<Contact> {
        // read the recipient's profile and unrevoked desktops
        const scope = schema.identifier("user").parse(recipient.id);
        const [profile] = await database
            .select({
                email: user.table.email,
                emailVerified: user.table.emailVerified,
                timeZone: user.table.timeZone,
            })
            .from(user.table)
            .where(eq(user.table.id, scope));
        const desktops = await database
            .select({ id: device.table.id })
            .from(device.table)
            .where(
                and(
                    eq(device.table.scope, scope),
                    eq(device.table.kind, "desktop"),
                    isNull(device.table.revokedAt),
                ),
            );

        // read their push endpoints and the language they set
        const endpoints = await database
            .select({
                id: pushEndpoint.table.id,
                url: pushEndpoint.table.url,
                keys: pushEndpoint.table.keys,
            })
            .from(pushEndpoint.table)
            .where(eq(pushEndpoint.table.scope, scope));
        const [language] = await Contact.settings(database, recipient, [locale]);
        const selection = { scope };
        const set = locale.resolve(selection, language === undefined ? [] : [language], [
            scope,
        ]).value;

        return {
            desktops: desktops.map((row) => row.id),
            endpoints,
            ...(profile?.emailVerified === true && profile.email !== null
                ? { email: profile.email }
                : {}),
            timeZone: profile?.timeZone ?? UNIVERSAL_TIME_ZONE,
            locale: set ?? SOURCE_LOCALE,
        };
    },

    /** Read the values a recipient placed in their scope for some settings. */
    async settings(
        database: DatabaseConnection,
        recipient: Subject,
        settings: readonly Setting[],
    ): Promise<SettingValue[]> {
        return Snapshot.live(database).rows(setting.table, {
            AND: [
                { scope: recipient.id },
                {
                    OR: settings.map((declared) => ({
                        packageId: declared.reference.packageId,
                        name: declared.reference.name,
                    })),
                },
            ],
        });
    },

    /** Decide whether a contact still has the address a delivery goes to. */
    addresses(
        contact: Contact,
        delivery: Pick<Delivery, "id" | "channel" | "device" | "endpoint">,
    ): boolean {
        // find the desktop's device
        if (delivery.channel === "desktop") {
            if (delivery.device === null) {
                throw new TypeError(`desktop delivery ${delivery.id} has no device`);
            }

            return contact.desktops.includes(delivery.device);
        }
        // find the push endpoint
        else if (delivery.channel === "push") {
            return contact.endpoints.some((known) => known.id === delivery.endpoint);
        }
        // find the email address
        else {
            return contact.email !== undefined;
        }
    },
};
