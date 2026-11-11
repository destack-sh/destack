import { nth, present } from "@destack/schema";
import { Mail, type MailTransport } from "../transport/index.ts";
import { MailError } from "../error/error.ts";
import { Mailbox } from "../mime/index.ts";
import type { SmtpClient } from "./client.ts";
import { Reply } from "./reply.ts";

/** Send email through an SMTP submission server from one sender. */
export class SmtpTransport implements MailTransport {
    /** The SMTP client. */
    readonly client: SmtpClient;
    /** The sender's mailbox, such as `Destack <notices@destack.app>`. */
    readonly from: string;
    /** The sender's address in the envelope. */
    readonly #sender: string;

    /** Send through a client from a sender's mailbox, refusing a malformed one. */
    constructor(client: SmtpClient, options: { readonly from: string }) {
        this.client = client;
        this.from = options.from;
        this.#sender = Mailbox.parse(options.from).address;
    }

    /** Compose one email under its key at its date, refusing it by the server's negative reply. */
    async send(mail: Mail): Promise<void> {
        // compose the message and submit it to its one recipient
        const { crypto } = this.client.options.environment;
        const message = await Mail.compose(mail, this.from, crypto);
        const envelope = { sender: this.#sender, recipients: [mail.to] };
        const submission = await this.client.submit(envelope, message).catch((error: unknown) => {
            // refuse a session's rejection by its reply's class
            if (
                MailError.is(error, "COMMAND_INVALID") ||
                MailError.is(error, "COMMAND_UNAVAILABLE")
            ) {
                const { code, enhancedCode, text } = error.data;
                throw new Reply(code, enhancedCode, text).refuse({ cause: error });
            }
            throw error;
        });

        // refuse the recipient's negative reply, else the one ending its data
        const recipient = nth(submission.recipients, 0).reply;
        const reply =
            recipient.class === 2
                ? present(submission.data, "an accepted recipient's data")
                : recipient;
        if (reply.class !== 2) {
            throw reply.refuse();
        }
    }
}
