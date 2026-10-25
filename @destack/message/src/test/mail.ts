import { present } from "@destack/schema";
import type { Mail, MailTransport } from "../email/index.ts";
import type { Outcome } from "../provider/index.ts";

/** A mail transport answering each email with the next scripted outcome, sent by default. */
export class TestMailTransport implements MailTransport {
    /** The emails sent, in order. */
    readonly sent: Mail[] = [];
    /** The outcomes the transport answers in turn, sent for every email when empty. */
    readonly #outcomes: readonly Outcome[];

    /** Answer each email with the next of some outcomes, or every one as sent. */
    constructor(outcomes: readonly Outcome[] = []) {
        this.#outcomes = outcomes;
    }

    /** Store an email, and answer the next outcome. */
    async send(mail: Mail): Promise<Outcome> {
        this.sent.push(mail);

        return this.#outcomes.length === 0
            ? { kind: "sent" }
            : present(this.#outcomes[this.sent.length - 1], "an outcome for each email");
    }
}
