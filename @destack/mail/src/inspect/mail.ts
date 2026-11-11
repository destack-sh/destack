import type { ResourceDeclaration } from "@destack/resource";
import { type MailDescription, MailKind, type MailSpec } from "../declare/mail.ts";

/** Describe a declared mail resource for the package manifest. */
export function describeMail(mail: ResourceDeclaration<unknown, typeof MailSpec>): MailDescription {
    return MailKind.description.parse({ name: mail.name, spec: mail.spec });
}
