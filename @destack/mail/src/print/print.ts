import type { Outcome } from "@destack/message/provider";
import type { Mail, MailTransport } from "../mail/index.ts";

/** Print each email instead of sending it, as a host on a person's machine prints its mail. */
export class PrintTransport implements MailTransport {
    /** Write one printed email. */
    readonly #print: (text: string) => void;

    /** Print emails through a writer, such as a process's standard output. */
    constructor(print: (text: string) => void) {
        this.#print = print;
    }

    /** Print one email's address, subject and plain text body. */
    async send(mail: Mail): Promise<Outcome> {
        this.#print(`email to ${mail.to}: ${mail.subject}\n\n${mail.text}\n`);

        return { outcome: "sent" };
    }
}
