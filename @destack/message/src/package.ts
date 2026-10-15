import { definePackage } from "@destack/runtime";
import { defineMail } from "./email/kind.ts";
import { endpoint, mail, message, messageAttempt } from "./object/index.ts";

/** The mail the package sends through, which its host binds to its provider. */
export const outboundMail = defineMail("outbound");

/** The handle stores serve messages, their attempts and webhook endpoints through. */
export default definePackage({
    objects: { message, attempt: messageAttempt, endpoint, mail },
    resources: { outbound: outboundMail },
});
