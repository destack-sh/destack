import { aligned, present } from "@destack/schema";
import type { Mail, MailTransport } from "../email/index.ts";
import { Mailbox, MimeError, MimeMessage } from "../mime/index.ts";
import type { Outcome } from "../provider/index.ts";
import type { SmtpClient, Submission } from "./client.ts";
import { SmtpError } from "./error.ts";
import type { Reply } from "./reply.ts";

/** Send email through an SMTP submission server from one sender. */
export class SmtpTransport implements MailTransport {
    /** The SMTP client. */
    readonly client: SmtpClient;
    /** The sender's mailbox, such as `Destack <notices@destack.app>`. */
    readonly from: string;
    /** The sender's address, which the envelope names. */
    readonly #sender: string;

    /** Send through a client from a sender's mailbox, refusing a malformed one. */
    constructor(client: SmtpClient, options: { readonly from: string }) {
        this.client = client;
        this.from = options.from;
        this.#sender = Mailbox.parse(options.from).address;
    }

    /** Compose one email under its key at its date and read the server's replies as an outcome. */
    async send(mail: Mail): Promise<Outcome> {
        try {
            // compose the message, its Message-ID digesting the key
            const message = await MimeMessage.compose({
                from: this.from,
                to: [mail.to],
                subject: mail.subject,
                date: new Date(mail.date),
                key: mail.key,
                text: mail.text,
                ...(mail.html === undefined ? {} : { html: mail.html }),
            });
            const envelope = { sender: this.#sender, recipients: [mail.to] };

            return readSubmission(await this.client.submit(envelope, message));
        } catch (error) {
            // fail an address the message cannot hold, and read the session's failure
            if (error instanceof MimeError) {
                return { kind: "failed", error: { code: error.code, message: error.message } };
            } else if (error instanceof SmtpError) {
                return readFailure(error);
            }
            throw error;
        }
    }
}

/** Read the replies to one recipient and its data: sent once both complete, else the refusal. */
function readSubmission(submission: Submission): Outcome {
    // read the recipient's refusal, else the reply ending its data
    const recipient = aligned(submission.recipients, 0).reply;
    const reply =
        recipient.class === 2
            ? present(submission.data, "an accepted recipient's data")
            : recipient;

    return reply.class === 2 ? { kind: "sent" } : readReply(reply);
}

/** Read a session's failure: a lost connection or a timeout retried, a rejection by its reply, anything else failed. */
function readFailure(error: SmtpError): Outcome {
    const refusal = { code: error.code, message: error.message };

    // retry a lost connection or a silent server
    if (error.code === "CONNECTION" || error.code === "TIMEOUT") {
        return { kind: "retry", error: refusal };
    }
    // read a rejection by the reply that rejected the session
    else if (error.code === "REJECTED" && error.reply !== undefined) {
        return readReply(error.reply);
    }
    // fail anything else
    else {
        return { kind: "failed", error: refusal };
    }
}

/** Read a negative reply: a transient 4xx retried, a permanent 5xx failed (RFC 5321 4.2.1). */
function readReply(reply: Reply): Outcome {
    const error = { code: String(reply.code), message: reply.text };

    return reply.class === 4 ? { kind: "retry", error } : { kind: "failed", error };
}
