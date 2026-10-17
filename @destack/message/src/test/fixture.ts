import { AccessContext, Caller, principal } from "@destack/access";
import type { Outcome } from "../provider/index.ts";
import type { Mail, MailTransport } from "../email/index.ts";
import type { ObjectStore } from "@destack/object/server";
import { Clock } from "@destack/environment";

/** A mail transport storing the emails it sends in memory. */
export class MailFixture implements MailTransport {
    /** The emails sent, in order. */
    readonly sent: Mail[] = [];

    /** Store an email as sent. */
    async send(mail: Mail): Promise<Outcome> {
        this.sent.push(mail);

        return { kind: "sent" };
    }
}

/** A server without object reads, for a provider in a test that sends without a space. */
export const UNQUERIED: Pick<ObjectStore, "query" | "clock" | "serving"> = {
    query: async () => {
        throw new TypeError("the test cannot read objects");
    },
    clock: Clock.system,
    serving() {
        const space = principal.space.reference(
            "universe",
            "space-019f5530-8000-7000-8000-0000000000e9",
        );

        return AccessContext.of(Caller.of(space), Clock.system.now());
    },
};
