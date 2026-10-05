import type { MessageProvider, Outcome } from "@destack/message/provider";
import type { MailTransport } from "../mail/index.ts";

/** Send email messages through a mail transport, keyed by each message's identifier. */
export function mailProvider(transport: MailTransport): MessageProvider {
    return {
        channel: "email",
        async send(message): Promise<Outcome> {
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
            });
        },
    };
}
