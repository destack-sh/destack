import type { Outcome } from "../delivery/outcome.ts";
import type { Vapid } from "./vapid.ts";

/** A push message's urgency, as RFC 8030 names it. */
export type Urgency = "very-low" | "low" | "normal" | "high";

/** One encrypted push message, as RFC 8030 sends it. */
export interface PushRequest {
    /** The push resource URL. */
    readonly url: string;
    /** The body, encrypted for the browser. */
    readonly body: Uint8Array<ArrayBuffer>;
    /** The topic a later message replaces it by. */
    readonly topic: string;
    /** The urgency. */
    readonly urgency: Urgency;
    /** How long the push service keeps it, in seconds. */
    readonly ttl: number;
}

/** A way to send push messages. */
export interface PushTransport {
    /** Send one message. */
    send(request: PushRequest): Promise<Outcome>;
}

/** Send push messages directly, signed with VAPID keys. */
export class WebPushTransport implements PushTransport {
    /** The sender's identity. */
    readonly vapid: Vapid;
    /** The fetch function. */
    readonly #fetch: typeof fetch;

    /** Send as a VAPID identity. */
    constructor(vapid: Vapid, options: { readonly fetch?: typeof fetch } = {}) {
        this.vapid = vapid;
        this.#fetch = options.fetch ?? fetch;
    }

    /** Post one message and read its outcome. */
    async send(request: PushRequest): Promise<Outcome> {
        // post the signed, encrypted body
        const now = Date.now();
        const response = await this.#fetch(request.url, {
            method: "POST",
            headers: {
                Authorization: await this.vapid.authorization(request.url, now),
                "Content-Encoding": "aes128gcm",
                "Content-Type": "application/octet-stream",
                TTL: String(request.ttl),
                Topic: request.topic,
                Urgency: request.urgency,
            },
            body: request.body,
        });
        const error = { code: String(response.status), message: await response.text() };

        // map the status to an outcome
        if (response.ok) {
            return { outcome: "sent" };
        } else if (response.status === 404 || response.status === 410) {
            return { outcome: "gone" };
        } else if (response.status === 429 || response.status >= 500) {
            const after = retryAfter(response.headers.get("Retry-After"), now);

            return after === undefined
                ? { outcome: "retry", error }
                : { outcome: "retry", after, error };
        } else {
            return { outcome: "failed", error };
        }
    }
}

/** Read a Retry-After header as milliseconds from now. */
function retryAfter(header: string | null, now: number): number | undefined {
    if (header === null) {
        return undefined;
    }
    const seconds = Number(header);

    return Number.isFinite(seconds) ? seconds * 1000 : Math.max(0, Date.parse(header) - now);
}
