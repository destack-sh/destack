import { client, user } from "@destack/account/object";
import { definePackage } from "@destack/object";
import { defineMail } from "@destack/mail/declare";
import { MessageError } from "./error/error.ts";
import { endpoint, message, messageAttempt, task } from "./object/index.ts";
import { t } from "@destack/locale";

/** The capability through which the package reaches the hosts messages go to. */
export const DELIVER_CAPABILITY = "deliver";

/** The mail the package sends through, which its host binds to its provider. */
export const outboundMail = defineMail({ name: "outbound" });

/** The handle stores serve messages, their attempts and webhook endpoints through. */
export default definePackage({
    title: t`Messages`,
    description: t`Send messages on a channel, email, push, webhook or a desktop.`,
    icon: "chat-circle",
    color: "violet",
    objects: [message, messageAttempt, endpoint, ...task.objects],
    resources: [outboundMail],
    access: {
        permissions: {
            space: {
                // send messages and deliver endpoints in the space, recording each attempt
                space: [
                    message.permission("read"),
                    message.permission("send"),
                    message.permission("control"),
                    messageAttempt.permission("record"),
                    endpoint.permission("read"),
                    endpoint.permission("manage"),
                    endpoint.permission("control"),
                ],
            },
        },
    },
    capabilities: [
        {
            name: DELIVER_CAPABILITY,
            reason: t`Deliver messages to the webhooks, push services and mail providers they go to`,
            optional: true,
            sandbox: { network: { connect: ["*"] } },
            permissions: {
                space: {
                    universe: [user.permission("recipient"), client.permission("unsubscribe")],
                },
            },
        },
    ],
    errors: MessageError,
});
