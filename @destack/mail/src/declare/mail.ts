import { defineKind, ModuleMetadata } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import { schema } from "@destack/schema";
import type { MailTransport } from "../transport/mail.ts";

/** A mail resource's specification, empty: its host's provider sends from the host's address. */
export const MailSpec = schema.object({});
/** The mail resource kind: sending email through its host's provider, such as SES or SMTP. */
export const MailKind = defineKind({ name: "mail", provisioned: { spec: MailSpec } });
/** A named mail dependency. */
export type MailDescription = schema.Infer<typeof MailKind.description>;

/** Declare a mail dependency. */
export function defineMail(
    options: { readonly name: string },
    module?: ModuleMetadata,
): ResourceDeclaration<MailTransport, typeof MailSpec> {
    const owner = ModuleMetadata.require(module, "defineMail").package;

    return new ResourceDeclaration<MailTransport, typeof MailSpec>(owner, MailKind, {
        name: options.name,
        spec: {},
    });
}
