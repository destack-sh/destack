import { MimeError } from "./error.ts";
import type { Mailbox } from "./mailbox.ts";

/** The line length folding aims for: RFC 2047 limits lines with encoded-words to 76 characters. */
const LINE_LIMIT = 76;

/** The longest header line written: RFC 5322 section 2.1.1 limits lines to 998 characters. */
const MAX_LINE_LENGTH = 998;

/** The most UTF-8 bytes one encoded-word carries: 36 bytes are 48 base64 characters. */
const ENCODED_WORD_BYTES = 36;

/** A value of printable ASCII and spaces, written as it stands. */
const PRINTABLE = /^[\x20-\x7E]*$/u;

/** One header field, as the tokens that folding may split between lines. */
export class Header {
    /** The field name, such as Subject. */
    readonly name: string;
    /** The value's tokens, which spaces join. */
    readonly tokens: readonly string[];

    /** Create a field from its name and its value's tokens. */
    constructor(name: string, tokens: readonly string[]) {
        this.name = name;
        this.tokens = tokens;
    }

    /** Create an unstructured field: printable ASCII as it stands, other text as encoded-words. */
    static unstructured(name: string, value: string): Header {
        return new Header(
            name,
            PRINTABLE.test(value) ? value.split(" ") : Header.encodeWords(value),
        );
    }

    /** Create an address list field, its mailboxes split by commas. */
    static mailboxes(name: string, mailboxes: readonly Mailbox[]): Header {
        // end every mailbox but the last with a comma
        const tokens = mailboxes.flatMap((mailbox, index) => {
            // read the mailbox's last token
            const written = mailbox.tokens();
            const last = written.at(-1);
            if (last === undefined) {
                throw new TypeError("mailbox has no tokens");
            }

            // leave the last mailbox as written
            if (index === mailboxes.length - 1) {
                return written;
            }

            return [...written.slice(0, -1), `${last},`];
        });

        return new Header(name, tokens);
    }

    /** Encode text as RFC 2047 B encoded-words, splitting it between code points. */
    static encodeWords(text: string): string[] {
        // gather code points into chunks of at most the encoded-word bytes
        const encoder = new TextEncoder();
        const chunks: string[] = [];
        let chunk = "";
        let bytes = 0;
        for (const character of text) {
            const length = encoder.encode(character).length;
            if (bytes + length > ENCODED_WORD_BYTES) {
                chunks.push(chunk);
                chunk = "";
                bytes = 0;
            }
            chunk += character;
            bytes += length;
        }

        // write each chunk as one encoded-word
        if (chunk !== "") {
            chunks.push(chunk);
        }

        return chunks.map((part) => `=?UTF-8?B?${encoder.encode(part).toBase64()}?=`);
    }

    /** Write the field with its line break, folded at spaces toward 76 characters a line. */
    write(): string {
        // fold before each word that would overrun its line
        const lines: string[] = [];
        let line = `${this.name}:`;
        for (const token of this.tokens) {
            if (token !== "" && line.length + 1 + token.length > LINE_LIMIT) {
                lines.push(line);
                line = ` ${token}`;
            } else {
                line = `${line} ${token}`;
            }
        }
        lines.push(line);

        // refuse a word too long for any line
        if (lines.some((written) => written.length > MAX_LINE_LENGTH)) {
            throw new MimeError(
                "INVALID_HEADER",
                `header ${this.name} has a line over ${MAX_LINE_LENGTH} characters`,
            );
        }

        return `${lines.join("\r\n")}\r\n`;
    }
}
