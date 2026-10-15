import type { ResourceDeclaration } from "@destack/resource";
import { type MailDescription, MailKind } from "../email/kind.ts";

/** Describe a declared mail resource for the package manifest. */
export function describeMail(mail: ResourceDeclaration<unknown, MailDescription>): MailDescription {
    return MailKind.description.parse({ name: mail.name, kind: mail.kind, spec: mail.spec });
}
