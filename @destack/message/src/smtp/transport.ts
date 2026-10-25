import { nth, present } from "@destack/schema";
import { Mail, type MailTransport } from "../email/index.ts";
import { type FamilyFailure } from "@destack/error";
import { MessageError, type MessageReason, SmtpReply } from "../error/error.ts";
import { Mailbox } from "../mime/index.ts";
import { Outcome } from "../provider/index.ts";
import type { SmtpClient, Submission } from "./client.ts";
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

    /** Compose one email under its key at its date and read the server's replies as an outcome. */
    async send(mail: Mail): Promise<Outcome> {
        try {
            // compose the message and submit it to its one recipient
            const { crypto } = this.client.options.environment;
            const message = await Mail.compose(mail, this.from, crypto);
            const envelope = { sender: this.#sender, recipients: [mail.to] };

            return readSubmission(await this.client.submit(envelope, message));
        } catch (error) {
            // read a composition's or a session's failure
            if (MessageError.is(error)) {
                return readFailure(error);
            }
            throw error;
        }
    }
}

/** Read the replies to one recipient and its data: sent once both complete, else the refusal. */
function readSubmission(submission: Submission): Outcome {
    // read the recipient's refusal, else the reply ending its data
    const recipient = nth(submission.recipients, 0).reply;
    const reply =
        recipient.class === 2
            ? present(submission.data, "an accepted recipient's data")
            : recipient;

    return reply.class === 2 ? { kind: "sent" } : readReply(reply);
}

/** Read a session's failure: a rejection by its reply, any other by its reason. */
function readFailure(error: FamilyFailure<MessageReason>): Outcome {
    // read a rejection by its reply
    if (MessageError.is(error, "COMMAND_REJECTED")) {
        const data = SmtpReply.parse(error.data);

        return readReply(new Reply(data.code, data.enhancedCode, data.text));
    }

    return Outcome.refuse({ reason: error.reason, message: error.message });
}

/** Read a negative reply: a transient 4xx deferred, a permanent 5xx rejected (RFC 5321 4.2.1). */
function readReply(reply: Reply): Outcome {
    return Outcome.refuse({
        reason: reply.class === 4 ? "COMMAND_DEFERRED" : "COMMAND_REJECTED",
        code: String(reply.code),
        message: reply.text,
    });
}
