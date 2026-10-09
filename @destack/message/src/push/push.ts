import type { Fetch } from "@destack/service";
import { type MessageProvider, Outcome } from "../provider/index.ts";
import { PushEncryption } from "./encryption.ts";
import type { Vapid } from "./vapid.ts";

/** A provider of push messages, which also answers the key a scope's browsers subscribe with. */
export interface PushProvider extends MessageProvider {
    /** Read the application server key browsers subscribe to a scope's pushes with, its VAPID public key (RFC 8292 3.2). */
    applicationServerKey(scope: string): Promise<string>;
}

/** Send push messages to browsers through their push services, encrypted for each browser and signed with the VAPID keys of the message's scope. */
export function pushProvider(
    vapid: (scope: string) => Promise<Vapid>,
    fetch: Fetch = globalThis.fetch,
): PushProvider {
    return {
        async applicationServerKey(scope) {
            return (await vapid(scope)).publicKey();
        },
        async send(message, server): Promise<Outcome> {
            // require a push message
            const { to, content } = message;
            if (to.channel !== "push" || content.channel !== "push") {
                throw new TypeError(`message ${message.id} is no push`);
            }

            // encrypt the payload for the browser and post it signed by its scope under its topic
            const signer = await vapid(message.scope);
            const payload = new TextEncoder().encode(JSON.stringify(content.data));
            const response = await fetch(
                new Request(to.url, {
                    method: "POST",
                    headers: {
                        Authorization: await signer.authorization(to.url, server.clock()),
                        "Content-Encoding": "aes128gcm",
                        "Content-Type": "application/octet-stream",
                        TTL: String(content.ttl),
                        Urgency: content.urgency,
                        ...(content.topic === undefined ? {} : { Topic: content.topic }),
                    },
                    body: await PushEncryption.encrypt(to.keys, payload),
                }),
            );

            return Outcome.read(response, server.clock());
        },
    };
}
