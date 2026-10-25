import type { Fetch } from "@destack/object";
import { type MessageProvider, Outcome } from "../provider/index.ts";
import { PushEncryption } from "./encryption.ts";
import type { Vapid } from "./vapid.ts";

/** How long a push service may take before the send retries, as long as a webhook endpoint. */
const TIMEOUT_MILLISECONDS = 15_000;

/** A provider of push messages, which also answers the key browsers subscribe with. */
export interface PushProvider extends MessageProvider {
    /** Read the VAPID public key browsers subscribe to the pushes with (RFC 8292 3.2). */
    readonly applicationServerKey: () => Promise<string>;
}

/** Send push messages to browsers through their push services. */
export function pushProvider(vapid: () => Promise<Vapid>, fetch: Fetch): PushProvider {
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
            const request = new Request(to.url, {
                method: "POST",
                redirect: "manual",
                signal: store.environment.clock.timeout(TIMEOUT_MILLISECONDS),
                headers: {
                    Authorization: await signer.authorization(
                        to.url,
                        store.environment.clock.now(),
                    ),
                    "Content-Encoding": "aes128gcm",
                    "Content-Type": "application/octet-stream",
                    TTL: String(content.ttl),
                    Urgency: content.urgency,
                    ...(content.topic === undefined ? {} : { Topic: content.topic }),
                },
                body: await PushEncryption.encrypt(to.keys, payload, store.environment),
            });

            return Outcome.post(fetch, request, store.environment.clock);
        },
    };
}
