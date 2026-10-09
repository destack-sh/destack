import type { CallServer } from "@destack/object";
import type { Outcome } from "../provider/index.ts";
import type { Mail, MailTransport } from "../email/index.ts";

/** A mail transport keeping the emails it sends in memory. */
export class MailFixture implements MailTransport {
    /** The emails sent, in order. */
    readonly sent: Mail[] = [];

    /** Keep an email as sent. */
    async send(mail: Mail): Promise<Outcome> {
        this.sent.push(mail);

        return { kind: "sent" };
    }
}

/** A server a provider reads no objects through, as a test sending without a space. */
export const UNQUERIED: Pick<CallServer, "query" | "clock"> = {
    query: async () => {
        throw new TypeError("the test reads no objects");
    },
    clock: Date.now,
};
