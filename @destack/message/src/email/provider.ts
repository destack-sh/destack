import { MailError, type SesFailure, type SmtpReply } from "@destack/mail/error";
import type { MailTransport } from "@destack/mail/transport";
import { type MessageProvider, Outcome } from "../provider/index.ts";

/** Send email messages through a mail transport, keyed by each message's identifier. */
export function emailProvider(transport: MailTransport): MessageProvider {
    return {
        async send(message): Promise<Outcome> {
            // require an email message
            const { to, content } = message;
            if (to.channel !== "email" || content.channel !== "email") {
                throw new TypeError(`message '${message.id}' is not an email`);
            }

            // send the email, reading a mail failure as a refusal
            try {
                await transport.send({
                    to: to.address,
                    subject: content.subject,
                    text: content.text,
                    ...(content.html === undefined ? {} : { html: content.html }),
                    key: message.id,
                    date: message.createdAt,
                });
            } catch (error) {
                // refuse by the failure's reason, with the SMTP reply code or AWS code apart
                if (MailError.is(error)) {
                    const code = readCode(error.data);

                    return Outcome.refuse({
                        reason: error.reason,
                        ...(code === undefined ? {} : { code }),
                        message: error.message,
                    });
                }
                throw error;
            }

            return { kind: "sent" };
        },
    };
}

/** Read a mail failure's provider code: the SMTP reply code, or SES's AWS error code. */
function readCode(data: SmtpReply | SesFailure | undefined): string | undefined {
    if (data === undefined) {
        return undefined;
    } else if ("code" in data) {
        return String(data.code);
    } else {
        return data.awsCode;
    }
}
