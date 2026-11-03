import { defineObject } from "@destack/object";
import { MailKind } from "@destack/mail";
import { space } from "@destack/account/object";

/** A space's mail resource: the sender its installations' email leaves through. */
export const mail = defineObject({
    name: "mail",
    plural: "mails",
    scope: space,
    provisioned: { kind: MailKind },
    fields: {},
    administration: ["read", "manage"],
});
