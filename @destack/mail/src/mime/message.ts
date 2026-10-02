import { Digest } from "@destack/schema";
import { MimeError } from "./error.ts";
import { Header } from "./header.ts";
import { Mailbox } from "./mailbox.ts";

/** The hex digits a Message-ID and a boundary carry: 128 bits, unique at any mail volume. */
const DIGEST_HEX_LENGTH = 32;

/** The longest quoted-printable line before its `=`: RFC 2045 limits lines to 76 characters. */
const QUOTED_PRINTABLE_LIMIT = 75;

/** An RFC 5322 field name: printable ASCII except the colon. */
const FIELD_NAME = /^[\x21-\x39\x3B-\x7E]+$/u;

/** The fields the composer writes itself, in lower case, which extra headers may not repeat. */
const STRUCTURAL_HEADERS = new Set([
    "from",
    "to",
    "cc",
    "bcc",
    "reply-to",
    "subject",
    "date",
    "message-id",
    "mime-version",
    "content-type",
    "content-transfer-encoding",
]);

/** The fields a sender writes, which the composer turns into an RFC 5322 message. */
export interface Message {
    /** The author's mailbox, whose domain the Message-ID carries. */
    readonly from: string;
    /** The primary recipients' mailboxes. */
    readonly to: readonly string[];
    /** The copied recipients' mailboxes. */
    readonly cc?: readonly string[];
    /** The mailbox replies go to. */
    readonly replyTo?: string;
    /** The subject line. */
    readonly subject: string;
    /** The origination time. */
    readonly date: Date;
    /** The idempotency key the Message-ID digests with the From address, unique per sender. */
    readonly key: string;
    /** The plain text body. */
    readonly text: string;
    /** The HTML body, which makes the message multipart/alternative after the text. */
    readonly html?: string;
    /** The extra header fields, written after the structural ones. */
    readonly headers?: Readonly<Record<string, string>>;
}

/** A composed RFC 5322 message with MIME bodies, ready for SMTP DATA. */
export class MimeMessage {
    /** The Message-ID, in angle brackets. */
    readonly messageId: string;
    /** The message in 7-bit ASCII, lines ending in CRLF. */
    readonly content: string;

    /** Create a composed message. */
    private constructor(messageId: string, content: string) {
        this.messageId = messageId;
        this.content = content;
    }

    /** Compose a message: its headers, then its text body or both bodies as alternatives. */
    static async compose(message: Message): Promise<MimeMessage> {
        // refuse an empty key and an invalid date
        if (message.key === "") {
            throw new MimeError("INVALID_KEY", "message key is empty");
        } else if (Number.isNaN(message.date.getTime())) {
            throw new MimeError("INVALID_DATE", "message date is invalid");
        }

        // derive the Message-ID and the boundary from the digest of the From address and the key
        const from = Mailbox.parse(message.from);
        const digest = await digestKey(from.address, message.key);
        const messageId = `<${digest}@${from.domain}>`;
        const boundary = `=_${digest}`;

        // write the structural headers, skipping empty address lists
        const to = message.to.map((mailbox) => Mailbox.parse(mailbox));
        const cc = (message.cc ?? []).map((mailbox) => Mailbox.parse(mailbox));
        const headers = [
            Header.mailboxes("From", [from]),
            ...(to.length === 0 ? [] : [Header.mailboxes("To", to)]),
            ...(cc.length === 0 ? [] : [Header.mailboxes("Cc", cc)]),
            ...(message.replyTo === undefined
                ? []
                : [Header.mailboxes("Reply-To", [Mailbox.parse(message.replyTo)])]),
            Header.unstructured("Subject", message.subject),
            new Header("Date", [writeDate(message.date)]),
            new Header("Message-ID", [messageId]),
            new Header("MIME-Version", ["1.0"]),
        ];

        // write the extra headers, refusing a malformed or structural field
        for (const [name, value] of Object.entries(message.headers ?? {})) {
            if (!FIELD_NAME.test(name)) {
                throw new MimeError(
                    "INVALID_HEADER",
                    `header name ${JSON.stringify(name)} contains a character outside printable ASCII or a colon`,
                );
            } else if (STRUCTURAL_HEADERS.has(name.toLowerCase())) {
                throw new MimeError("INVALID_HEADER", `header ${name} is written by the composer`);
            }
            headers.push(Header.unstructured(name, value));
        }

        // write a text body alone
        const text = writePart("text/plain", message.text);
        if (message.html === undefined) {
            return new MimeMessage(messageId, `${writeHeaders(headers)}${text}`);
        }

        // write both bodies as alternatives, text first
        const html = writePart("text/html", message.html);
        const alternative = new Header("Content-Type", [
            "multipart/alternative;",
            `boundary="${boundary}"`,
        ]);
        const body = `--${boundary}\r\n${text}--${boundary}\r\n${html}--${boundary}--\r\n`;

        return new MimeMessage(messageId, `${writeHeaders([...headers, alternative])}\r\n${body}`);
    }
}

/** Digest a sender's address and a key with SHA-256, as a Message-ID and a boundary carry. */
async function digestKey(address: string, key: string): Promise<string> {
    // join the two with a space, which no address contains
    const digest = await Digest.of(`${address} ${key}`);

    return digest.slice(0, DIGEST_HEX_LENGTH);
}

/** Write header fields in order. */
function writeHeaders(headers: readonly Header[]): string {
    return headers.map((header) => header.write()).join("");
}

/** Write one UTF-8 body part: its content headers, a blank line, and its quoted-printable body. */
function writePart(type: "text/plain" | "text/html", body: string): string {
    const headers = writeHeaders([
        new Header("Content-Type", [`${type};`, "charset=utf-8"]),
        new Header("Content-Transfer-Encoding", ["quoted-printable"]),
    ]);

    return `${headers}\r\n${writeQuotedPrintable(body)}\r\n`;
}

/** Write an RFC 5322 date-time in UTC, such as `Tue, 29 Sep 2026 10:00:00 +0000`. */
function writeDate(date: Date): string {
    return date.toUTCString().replace("GMT", "+0000");
}

/** Encode text as RFC 2045 quoted-printable UTF-8 with CRLF line breaks and soft breaks. */
function writeQuotedPrintable(text: string): string {
    // encode each line apart, since line breaks stay literal
    const encoder = new TextEncoder();
    const lines = text.split(/\r\n|\r|\n/u).map((line) => {
        // escape each byte outside literal ASCII, and whitespace ending the line
        const bytes = encoder.encode(line);
        const pieces = Array.from(bytes, (byte, index) => {
            const isLiteral =
                (byte >= 0x21 && byte <= 0x7e && byte !== 0x3d) ||
                ((byte === 0x20 || byte === 0x09) && index < bytes.length - 1);

            return isLiteral
                ? String.fromCharCode(byte)
                : `=${byte.toString(16).toUpperCase().padStart(2, "0")}`;
        });

        // break softly before a piece that would overrun the line
        const written: string[] = [];
        let current = "";
        for (const piece of pieces) {
            if (current.length + piece.length > QUOTED_PRINTABLE_LIMIT) {
                written.push(`${current}=`);
                current = "";
            }
            current += piece;
        }
        written.push(current);

        return written.join("\r\n");
    });

    return lines.join("\r\n");
}
