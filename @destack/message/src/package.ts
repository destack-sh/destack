import { client } from "@destack/account/object";
import { definePackage } from "@destack/object";
import { defineMail } from "./email/kind.ts";
import { endpoint, mail, message, messageAttempt, task } from "./object/index.ts";

/** The mail the package sends through, which its host binds to its provider. */
export const outboundMail = defineMail("outbound");

/** The handle stores serve messages, their attempts and webhook endpoints through. */
export default definePackage({
    objects: { message, attempt: messageAttempt, endpoint, mail, ...task.objects },
    resources: { outbound: outboundMail },
    permissions: {
        // forget browsers their push services forgot, acting for the space a person receives in
        installation: { universe: [client.permission("unsubscribe")] },
    },
});
