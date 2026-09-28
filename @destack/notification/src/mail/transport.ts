import type { Outcome } from "../delivery/outcome.ts";

/** One plain-text email. */
export interface Mail {
    /** The address. */
    readonly to: string;
    /** The subject. */
    readonly subject: string;
    /** The plain text body. */
    readonly text: string;
    /** The key the provider deduplicates by. */
    readonly key: string;
}

/** A way to send email. */
export interface MailTransport {
    /** Send one email. */
    send(mail: Mail): Promise<Outcome>;
}
