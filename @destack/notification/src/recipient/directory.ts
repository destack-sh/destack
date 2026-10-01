import type { Subject } from "@destack/access";
import type { Setting } from "@destack/setting";
import type { SettingValue } from "@destack/setting/object";
import type { Delivery } from "../object/delivery.ts";
import type { PushEndpoint } from "../object/endpoint.ts";

/** Where a recipient receives notifications beyond the inbox. */
export interface Contact {
    /** The devices whose desktops show the recipient's banners. */
    readonly desktops: readonly string[];
    /** The push endpoints of the recipient's browsers. */
    readonly endpoints: readonly Pick<PushEndpoint, "id" | "url" | "keys" | "device">[];
    /** The recipient's verified email address, absent without one. */
    readonly email?: string;
}

/** A recipient's contact: where notifications reach them. */
export const Contact = {
    /** Decide whether a contact still has the address a delivery goes to. */
    reaches(contact: Contact, row: Delivery): boolean {
        // find the desktop's device
        if (row.channel === "desktop") {
            return contact.desktops.includes(row.device!);
        }
        // find the push endpoint
        else if (row.channel === "push") {
            return contact.endpoints.some((known) => known.id === row.endpoint);
        }
        // find the email address
        else {
            return contact.email !== undefined;
        }
    },
};

/** The recipients' settings, contact and presence, as the host reaches them. */
export interface RecipientDirectory {
    /** Read the values a recipient placed for some settings. */
    settings(recipient: Subject, settings: readonly Setting[]): Promise<readonly SettingValue[]>;
    /** Read where a recipient receives notifications beyond the inbox. */
    contact(recipient: Subject): Promise<Contact>;
    /** Read the devices a recipient is active on now, as their presence shows. */
    devices(recipient: Subject): Promise<readonly string[]>;
    /** Read the recipient's IANA time zone. */
    timeZone(recipient: Subject): Promise<string>;
    /** Forget a push endpoint its push service no longer knows. */
    forget(recipient: Subject, endpoint: string): Promise<void>;
}
