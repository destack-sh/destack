import { defineObject } from "@destack/object";
import { MailKind } from "@destack/mail";
import { space } from "@destack/account/object";
import { provisioned } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A space's mail resource: the sender its installations' email leaves through. */
export const mail = defineObject({
    name: "mail",
    plural: "mails",
    title: t`${plural(argument("count"), { one: "Mail service", other: "Mail services" })}`,
    scope: space,
    fields: {},
    access: {
        suspended: ["read", "manage"],
    },
    traits: [provisioned({ kind: MailKind })],
});
