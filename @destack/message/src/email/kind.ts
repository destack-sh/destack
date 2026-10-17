import { ModuleMetadata } from "@destack/package";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import { schema } from "@destack/schema";
import type { MailTransport } from "./mail.ts";

/** A mail resource's specification, empty: its host's provider sends from the host's address. */
export const MailSpec = schema.object({});
/** The mail resource kind: sending email through its host's provider, such as SES or SMTP. */
export const MailKind = defineResourceKind("mail", { spec: MailSpec });
/** A named mail dependency. */
export type MailDescription = schema.Infer<typeof MailKind.description>;

/** A declared mail resource, which its host connects to a transport. */
export class MailDeclaration extends ResourceDeclaration<MailTransport, MailDescription> {}

/** Declare a mail dependency. */
export function defineMail(name: string, module?: ModuleMetadata): MailDeclaration {
    const owner = ModuleMetadata.require(module, "defineMail").package;

    return new MailDeclaration(
        owner,
        MailKind.description.parse({ name, kind: MailKind.name, spec: {} }),
    );
}
