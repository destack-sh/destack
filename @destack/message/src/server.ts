import { defineServer } from "@destack/object/server";
import { emailProvider } from "./email/provider.ts";
import messagePackage, { outboundMail } from "./package.ts";
import { pushProvider } from "./push/push.ts";
import { Vapid } from "./push/vapid.ts";
import { webhookProvider } from "./webhook/webhook.ts";
import { implementEndpoint } from "./server/endpoint.ts";
import { implementMessage } from "./server/message.ts";

export * from "./server/index.ts";

/** The contact a push service reaches the platform's operator at, VAPID's `sub` (RFC 8292 2.1). */
const PUSH_CONTACT = "https://destack.app";

/** The message package's server: messages sent through each channel, and webhook endpoints. */
export default defineServer(messagePackage, (context) => {
    // send through the bound mail, the space's keys and the host's network
    const { keys } = context;
    const fetch = (request: Request) => context.fetch(request);
    const mail = context.resources.get(outboundMail);

    return {
        objects: [
            implementMessage({
                providers: {
                    email: emailProvider(mail),
                    push: pushProvider(() => Vapid.derive(keys, PUSH_CONTACT), fetch),
                    webhook: webhookProvider(fetch),
                },
                keys,
            }),
            implementEndpoint(keys),
        ],
    };
});
