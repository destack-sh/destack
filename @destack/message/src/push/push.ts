import type { Fetch } from "@destack/object";
import { type MessageProvider, Outcome } from "../provider/index.ts";
import { PushEncryption } from "./encryption.ts";
import type { Vapid } from "./vapid.ts";

/** A provider of push messages, which also answers the key browsers subscribe with. */
export interface PushProvider extends MessageProvider {
    /** Read the VAPID public key browsers subscribe to the pushes with (RFC 8292 3.2). */
    readonly applicationServerKey: () => Promise<string>;
}

/** Send push messages to browsers through their push services. */
export function pushProvider(
    vapid: () => Promise<Vapid>,
    fetch: Fetch = globalThis.fetch,
): PushProvider {
    return {
        async applicationServerKey() {
            return (await vapid()).publicKey();
        },
        async send(message, store): Promise<Outcome> {
            // require a push message
            const { to, content } = message;
            if (to.channel !== "push" || content.channel !== "push") {
                throw new TypeError(`message ${message.id} is not a push`);
            }

            // encrypt the payload for the browser and post it signed by the installation under its topic
            const signer = await vapid();
            const payload = new TextEncoder().encode(JSON.stringify(content.data));
            const response = await fetch(
                new Request(to.url, {
                    method: "POST",
                    headers: {
                        Authorization: await signer.authorization(to.url, store.clock.now()),
                        "Content-Encoding": "aes128gcm",
                        "Content-Type": "application/octet-stream",
                        TTL: String(content.ttl),
                        Urgency: content.urgency,
                        ...(content.topic === undefined ? {} : { Topic: content.topic }),
                    },
                    body: await PushEncryption.encrypt(to.keys, payload),
                }),
            );

            return Outcome.read(response, store.clock.now());
        },
    };
}
