import type { MessageProvider, Outcome } from "../provider/index.ts";
import { PushEncryption } from "./encryption.ts";
import type { Vapid } from "./vapid.ts";

/** The code of a refusal for good from a push service that forgot the endpoint (RFC 8030 7.3). */
export const GONE = "gone";

/** The statuses of a push service that forgot the endpoint (RFC 8030 7.3). */
const GONE_STATUSES: ReadonlySet<number> = new Set([404, 410]);

/** The milliseconds in a second, as Retry-After counts seconds. */
const MILLISECONDS = 1000;

/** Post one request to a push service, as fetch does. */
export type PushFetch = (url: string, init: RequestInit) => Promise<Response>;

/** Send push messages to browsers through their push services, encrypted for each browser and signed with VAPID keys. */
export function pushProvider(vapid: Vapid, send: PushFetch = fetch): MessageProvider {
    return {
        channel: "push",
        async send(message): Promise<Outcome> {
            // require a push message
            const { to, content } = message;
            if (to.channel !== "push" || content.channel !== "push") {
                throw new TypeError(`message ${message.id} is no push`);
            }

            // encrypt the payload for the browser and post it signed, under its topic
            const now = Date.now();
            const payload = new TextEncoder().encode(JSON.stringify(content.data));
            const response = await send(to.url, {
                method: "POST",
                headers: {
                    Authorization: await vapid.authorization(to.url, now),
                    "Content-Encoding": "aes128gcm",
                    "Content-Type": "application/octet-stream",
                    TTL: String(content.ttl),
                    Urgency: content.urgency,
                    ...(content.topic === undefined ? {} : { Topic: content.topic }),
                },
                body: await PushEncryption.encrypt(to.keys, payload),
            });
            const error = { code: String(response.status), message: await response.text() };

            // read the status as sent, forgotten, retried or failed
            if (response.ok) {
                return { outcome: "sent" };
            } else if (GONE_STATUSES.has(response.status)) {
                return { outcome: "failed", error: { ...error, code: GONE } };
            } else if (response.status === 429 || response.status >= 500) {
                const after = retryAfter(response.headers.get("Retry-After"), now);

                return after === undefined
                    ? { outcome: "retry", error }
                    : { outcome: "retry", after, error };
            }

            return { outcome: "failed", error };
        },
    };
}

/** Read a Retry-After header as milliseconds from now, absent without one. */
function retryAfter(header: string | null, now: number): number | undefined {
    if (header === null) {
        return undefined;
    }
    const seconds = Number(header);

    return Number.isFinite(seconds)
        ? seconds * MILLISECONDS
        : Math.max(0, Date.parse(header) - now);
}
