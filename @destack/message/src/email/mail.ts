import { defineSchema, schema } from "@destack/schema";
import { EmailContent } from "./message.ts";
import type { Outcome } from "../provider/index.ts";

/** The longest deduplication key, in characters: a header line's length (RFC 5322 2.1.1). */
const KEY_LENGTH = 998;

/** One email to one address, in plain text and optionally HTML. */
export const Mail = defineSchema(
    EmailContent.omit({ channel: true }).extend({
        /** The address. */
        to: schema.email(),
        /** The key the provider deduplicates by, which the Message-ID digests. */
        key: schema.string().min(1).max(KEY_LENGTH),
        /** The origination time the Date header carries, in milliseconds since the epoch. */
        date: schema.number().int(),
    }),
);
/** One email to one address. */
export type Mail = schema.Infer<typeof Mail>;

/** A way to send email, such as a mail service's HTTP API, SMTP or a printer. */
export interface MailTransport {
    /** Send one email, reading what the provider made of it. */
    send(mail: Mail): Promise<Outcome>;
}
