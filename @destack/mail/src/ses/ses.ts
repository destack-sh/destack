import { Mail, type MailTransport } from "../transport/index.ts";
import { Mailbox } from "../mime/index.ts";
import type { SesClient } from "./client.ts";

/** Send email through Amazon SES from one sender. */
export class SesTransport implements MailTransport {
    /** The SES client. */
    readonly client: SesClient;
    /** The sender's mailbox, such as `Destack <notices@destack.app>`. */
    readonly from: string;
    /** The sender's address in the envelope. */
    readonly #sender: string;

    /** Send through a client from a sender's mailbox, refusing a malformed one. */
    constructor(client: SesClient, options: { readonly from: string }) {
        this.client = client;
        this.from = options.from;
        this.#sender = Mailbox.parse(options.from).address;
    }

    /** Compose one email under its key at its date and send it, refusing it as SES refuses it. */
    async send(mail: Mail): Promise<void> {
        // compose the message and send it to its one recipient
        const { crypto } = this.client.options.environment;
        const message = await Mail.compose(mail, this.from, crypto);
        const envelope = { sender: this.#sender, recipients: [mail.to] };
        await this.client.send(envelope, message, new Date(mail.date));
    }
}
