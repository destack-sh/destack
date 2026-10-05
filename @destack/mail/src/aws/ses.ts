import type { Outcome } from "@destack/message/provider";
import type { Mail, MailTransport } from "../mail/index.ts";
import { Mailbox, MimeError, MimeMessage } from "../mime/index.ts";
import type { SesClient } from "./client.ts";
import { SesError } from "./error.ts";

/** Send email through Amazon SES from one sender. */
export class SesTransport implements MailTransport {
    /** The SES client. */
    readonly client: SesClient;
    /** The sender's mailbox, such as `Destack <notices@destack.app>`. */
    readonly from: string;
    /** The sender's address, which the envelope names. */
    readonly #sender: string;

    /** Send through a client from a sender's mailbox, refusing a malformed one. */
    constructor(client: SesClient, options: { readonly from: string }) {
        this.client = client;
        this.from = options.from;
        this.#sender = Mailbox.parse(options.from).address;
    }

    /** Compose one email under its key and read SES's answer as an outcome. */
    async send(mail: Mail): Promise<Outcome> {
        try {
            // compose the message, its Message-ID digesting the key
            const message = await MimeMessage.compose({
                from: this.from,
                to: [mail.to],
                subject: mail.subject,
                date: new Date(),
                key: mail.key,
                text: mail.text,
                ...(mail.html === undefined ? {} : { html: mail.html }),
            });
            await this.client.send({ sender: this.#sender, recipients: [mail.to] }, message);

            return { outcome: "sent" };
        } catch (error) {
            // fail an address the message cannot hold, and retry what SES may take later
            if (error instanceof MimeError) {
                return { outcome: "failed", error: { code: error.code, message: error.message } };
            } else if (error instanceof SesError) {
                const refusal = { code: error.awsCode ?? error.code, message: error.message };

                return error.isRetryable
                    ? { outcome: "retry", error: refusal }
                    : { outcome: "failed", error: refusal };
            }
            throw error;
        }
    }
}
