import { type DigestCrypto, schema } from "@destack/schema";
import { MimeMessage } from "../mime/index.ts";

/** The longest subject line, in characters: RFC 5322 2.1.1's line length. */
const SUBJECT_LENGTH = 998;

/** The longest text or HTML body, in characters: 64 KiB per message. */
const BODY_LENGTH = 64 * 1024;

/** The longest deduplication key, in characters: a header line's length (RFC 5322 2.1.1). */
const KEY_LENGTH = 998;

/** One email to one address, in plain text and optionally HTML. */
const mail = schema.object({
    /** The address. */
    to: schema.email(),
    /** The subject. */
    subject: schema.string().min(1).max(SUBJECT_LENGTH),
    /** The plain text body. */
    text: schema.string().max(BODY_LENGTH),
    /** The HTML body, absent for plain text only. */
    html: schema.string().max(BODY_LENGTH).exactOptional(),
    /** The key the provider deduplicates by, which the Message-ID digests. */
    key: schema.string().min(1).max(KEY_LENGTH),
    /** The origination time in the Date header, in milliseconds since the epoch. */
    date: schema.number().int(),
});

/** One email to one address, and how a transport composes it. */
export const Mail = Object.assign(mail, {
    /** Compose an email from a sender's mailbox, its Message-ID digesting its key. */
    compose(sent: Mail, from: string, crypto: DigestCrypto): Promise<MimeMessage> {
        return MimeMessage.compose(
            {
                from,
                to: [sent.to],
                subject: sent.subject,
                date: new Date(sent.date),
                key: sent.key,
                text: sent.text,
                ...(sent.html === undefined ? {} : { html: sent.html }),
            },
            crypto,
        );
    },
});
/** One email to one address. */
export type Mail = schema.Infer<typeof mail>;

/** A way to send email, such as a mail service's HTTP API, SMTP or a printer. */
export interface MailTransport {
    /** Send one email, resolving once sent and rejecting with a `MailError` on a refusal. */
    readonly send: (mail: Mail) => Promise<void>;
}
