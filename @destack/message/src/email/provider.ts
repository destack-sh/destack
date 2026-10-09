import type { MessageProvider, Outcome } from "../provider/index.ts";
import type { MailTransport } from "./mail.ts";

/** Send email messages through a mail transport, keyed by each message's identifier. */
export function emailProvider(transport: MailTransport): MessageProvider {
    return {
        async send(message, server): Promise<Outcome> {
            // require an email message
            const { to, content } = message;
            if (to.channel !== "email" || content.channel !== "email") {
                throw new TypeError(`message ${message.id} is no email`);
            }

            return transport.send({
                to: to.address,
                subject: content.subject,
                text: content.text,
                ...(content.html === undefined ? {} : { html: content.html }),
                key: message.id,
                date: server.clock(),
            });
        },
    };
}
