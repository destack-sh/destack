import type { Outcome } from "../provider/index.ts";
import type { Mail, MailTransport } from "../email/index.ts";

/** Print each email instead of sending it, as a machine on a person's computer prints its mail. */
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

        return { kind: "sent" };
    }
}
