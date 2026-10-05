import { defineSchema, schema } from "@destack/schema";
import { EmailContent } from "@destack/message/email";
import type { Outcome } from "@destack/message/provider";

/** The longest deduplication key, in characters: a header line's length (RFC 5322 2.1.1). */
const KEY_LENGTH = 998;

/** One email to one address, in plain text and optionally HTML. */
export const Mail = defineSchema(
    EmailContent.omit({ channel: true }).extend({
        /** The address. */
        to: schema.email(),
        /** The key the provider deduplicates by, which the Message-ID digests. */
        key: schema.string().min(1).max(KEY_LENGTH),
    }),
);
/** One email to one address. */
export type Mail = schema.Infer<typeof Mail>;

/** A way to send email, such as SES, SMTP or a printer. */
export interface MailTransport {
    /** Send one email, reading what the provider made of it. */
    send(mail: Mail): Promise<Outcome>;
}
