import { MimeError } from "./error.ts";
import { Header } from "./header.ts";

/** A dot-atom local part, which needs no quoting and no SMTPUTF8. */
const LOCAL_PART = /^[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+)*$/;

/** A hostname label of letters, digits and inner hyphens. */
const LABEL = /^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/;

/** The longest address a path carries: RFC 5321 limits a path to 256 octets with angle brackets. */
const MAX_ADDRESS_LENGTH = 254;

/** The longest local part: RFC 5321 section 4.5.3.1.1 limits it to 64 octets. */
const MAX_LOCAL_PART_LENGTH = 64;

/** The longest domain: RFC 5321 section 4.5.3.1.2 limits it to 255 octets. */
const MAX_DOMAIN_LENGTH = 255;

/** The longest domain label: RFC 1035 section 2.3.4 limits it to 63 octets. */
const MAX_LABEL_LENGTH = 63;

/** A display name of atoms split by spaces, which needs no quoting. */
const ATOM_PHRASE = /^[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+(?: [A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+)*$/;

/** A display name of printable ASCII, which quoting keeps. */
const PRINTABLE = /^[\x20-\x7E]*$/;

/** A mailbox written with a display name: the name, then the address in angle brackets. */
const NAMED_MAILBOX = /^(.*)<([^<>]*)>$/;

/** An RFC 5322 mailbox: an address with an optional display name. */
export class Mailbox {
    /** The addr-spec, such as ada@example.com. */
    readonly address: string;
    /** The display name, such as Ada Lovelace, absent for a bare address. */
    readonly name: string | undefined;

    /** Create a mailbox, refusing an address outside the dot-atom form. */
    constructor(address: string, name?: string) {
        // require an address a path carries
        if (!Mailbox.isAddress(address)) {
            throw new MimeError(
                "INVALID_ADDRESS",
                `address ${JSON.stringify(address)} is not a dot-atom addr-spec`,
            );
        }
        this.address = address;
        this.name = name;
    }

    /** Parse a mailbox written as `Name <address>`, `"Quoted Name" <address>` or a bare address. */
    static parse(text: string): Mailbox {
        // split a display name from its address in angle brackets
        const named = NAMED_MAILBOX.exec(text.trim());
        if (named === null) {
            return new Mailbox(text.trim());
        }

        // unquote a quoted display name
        const name = named[1]!.trim();
        const isQuoted = name.length >= 2 && name.startsWith('"') && name.endsWith('"');
        const unquoted = isQuoted ? name.slice(1, -1).replaceAll(/\\(.)/g, "$1") : name;

        return new Mailbox(named[2]!, unquoted === "" ? undefined : unquoted);
    }

    /** Report whether text is a dot-atom addr-spec within the RFC 5321 length limits. */
    static isAddress(text: string): boolean {
        const at = text.lastIndexOf("@");
        const localPart = text.slice(0, at);

        return (
            at !== -1 &&
            text.length <= MAX_ADDRESS_LENGTH &&
            localPart.length <= MAX_LOCAL_PART_LENGTH &&
            LOCAL_PART.test(localPart) &&
            Mailbox.isDomain(text.slice(at + 1))
        );
    }

    /** Report whether text is a domain of hostname labels within the RFC 5321 length limits. */
    static isDomain(text: string): boolean {
        const labels = text.split(".");

        return (
            text.length <= MAX_DOMAIN_LENGTH &&
            labels.every((label) => label.length <= MAX_LABEL_LENGTH && LABEL.test(label))
        );
    }

    /** The domain after the address's `@`. */
    get domain(): string {
        return this.address.slice(this.address.lastIndexOf("@") + 1);
    }

    /** Write the mailbox as header tokens: the display name, then the address. */
    tokens(): string[] {
        // write a bare address alone
        const name = this.name;
        if (name === undefined) {
            return [this.address];
        }

        // write the display name as atoms, a quoted string folded at spaces, or encoded-words
        const address = `<${this.address}>`;
        if (ATOM_PHRASE.test(name) && !name.includes("=?")) {
            return [...name.split(" "), address];
        } else if (PRINTABLE.test(name)) {
            return [...`"${name.replaceAll(/["\\]/g, "\\$&")}"`.split(" "), address];
        } else {
            return [...Header.encodeWords(name), address];
        }
    }
}
