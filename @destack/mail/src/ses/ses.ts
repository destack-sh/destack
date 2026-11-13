import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { Mail, type MailTransport } from "../transport/index.ts";
import { Mailbox } from "../mime/index.ts";
import type { SesClient } from "./client.ts";

/** Send email through Amazon SES from one sender. */
export class SesTransport implements MailTransport {
    /** The SES client. */
    readonly client: SesClient;
    /** The sender's mailbox, such as `Destack <notices@destack.app>`. */
    readonly from: string;
    /** The sender's address in the envelope. */
    readonly #sender: string;
    /** The spans and durations of the sends. */
    readonly #scope: TelemetryScope;

    /** Send through a client from a sender's mailbox, refusing a malformed one. */
    constructor(client: SesClient, options: { readonly from: string }) {
        // keep the client and the sender, and instrument on the client's telemetry
        this.client = client;
        this.from = options.from;
        this.#sender = Mailbox.parse(options.from).address;
        this.#scope = TelemetryScope.of(
            import.meta.destack.package,
            client.options.environment.telemetry,
        );
    }

    /** Compose one email under its key at its date and send it, refusing it as SES refuses it. */
    send(mail: Mail): Promise<void> {
        // send it in a client span, its duration by the transport
        const transport = { "destack.mail.transport": "ses" };

        return this.#scope.span(
            "destack.mail.send",
            transport,
            async () => {
                // compose the message and send it to its one recipient
                const { crypto } = this.client.options.environment;
                const message = await Mail.compose(mail, this.from, crypto);
                const envelope = { sender: this.#sender, recipients: [mail.to] };
                await this.client.send(envelope, message, new Date(mail.date));
            },
            { kind: "client", duration: { attributes: transport } },
        );
    }
}
