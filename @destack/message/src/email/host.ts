import type { Connector, Provider } from "@destack/resource";
import { type MailDeclaration, MailKind } from "./kind.ts";
import type { MailTransport } from "./mail.ts";

/** Provide a host's mail resources under its provider code, which need nothing provisioned. */
export function mailProvider<Object>(
    code: string,
    object: Object,
): Provider<typeof MailKind, Object> {
    return {
        kind: MailKind,
        code,
        object,
        provision: {
            provision: async () => ({ reference: code }),
            destroy: async () => {},
        },
    };
}

/** Connect bound mail resources to a host's transport under its provider code. */
export function mailConnector(
    code: string,
    transport: MailTransport,
): Connector<MailTransport, MailDeclaration> {
    return {
        code,
        connect: async () => ({
            send: (mail) => transport.send(mail),
            [Symbol.asyncDispose]: async () => {},
        }),
    };
}
