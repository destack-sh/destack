import type { FamilyError } from "@destack/error";
import { present } from "@destack/schema";
import type { Mail, MailTransport } from "../transport/index.ts";
import type { MailReason } from "../error/error.ts";

/** How a test transport answers one email: sending it, or refusing it with a failure. */
export type TestMailAnswer = "sent" | FamilyError<MailReason>;

/** A mail transport answering each email with the next scripted answer, sent by default. */
export class TestMailTransport implements MailTransport {
    /** The emails sent, in order. */
    readonly sent: Mail[] = [];
    /** The answers the transport gives in turn, sent for every email when empty. */
    readonly #answers: readonly TestMailAnswer[];

    /** Answer each email with the next of some answers, or send every one. */
    constructor(answers: readonly TestMailAnswer[] = []) {
        this.#answers = answers;
    }

    /** Store an email, and refuse it when its answer is a failure. */
    async send(mail: Mail): Promise<void> {
        // store the email and read its answer
        this.sent.push(mail);
        const answer =
            this.#answers.length === 0
                ? "sent"
                : present(this.#answers[this.sent.length - 1], "an answer for each email");
        if (answer !== "sent") {
            throw answer;
        }
    }
}
