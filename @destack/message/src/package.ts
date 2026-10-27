import { client, user } from "@destack/account/object";
import { definePackage } from "@destack/object";
import { defineMail } from "./email/kind.ts";
import { MessageError } from "./error/error.ts";
import { endpoint, mail, message, messageAttempt, task } from "./object/index.ts";

/** The capability through which the package reaches the hosts messages go to. */
export const DELIVER_CAPABILITY = "deliver";

/** The mail the package sends through, which its host binds to its provider. */
export const outboundMail = defineMail("outbound");

/** The handle stores serve messages, their attempts and webhook endpoints through. */
export default definePackage({
    objects: { message, attempt: messageAttempt, endpoint, mail, ...task.objects },
    resources: { outbound: outboundMail },
    permissions: {
        installation: {
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
    capabilities: {
        // reach the hosts messages go to, and the browsers people receive pushes at
        [DELIVER_CAPABILITY]: {
            reason: "Deliver messages to the webhooks, push services and mail providers they go to",
            optional: true,
            sandbox: { network: { connect: ["*"] } },
            permissions: {
                installation: {
                    universe: [user.permission("recipient"), client.permission("unsubscribe")],
                },
            },
        },
    },
    errors: MessageError,
});
