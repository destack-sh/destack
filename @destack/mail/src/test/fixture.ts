import type { Outcome } from "@destack/message/provider";
import type { Mail, MailTransport } from "../mail/index.ts";

/** A mail transport keeping the emails it sends in memory. */
export class MailFixture implements MailTransport {
    /** The emails sent, in order. */
    readonly sent: Mail[] = [];

    /** Keep an email as sent. */
    async send(mail: Mail): Promise<Outcome> {
        this.sent.push(mail);

        return { outcome: "sent" };
    }
}
