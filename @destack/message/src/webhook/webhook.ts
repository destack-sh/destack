import type { MessageProvider, Outcome } from "../provider/index.ts";

/** The prefix of a Standard Webhooks signing secret. */
const SECRET_PREFIX = "whsec_";

/** How long a webhook endpoint takes to answer before the send is retried: Standard Webhooks' 15 seconds. */
const TIMEOUT_MILLISECONDS = 15_000;

/** The statuses an endpoint answers to be asked again later: timeouts, throttling and server failures. */
const RETRIED_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** Generate a Standard Webhooks signing secret: 24 random bytes, base64, prefixed. */
export function generateSecret(): string {
    return `${SECRET_PREFIX}${crypto.getRandomValues(new Uint8Array(24)).toBase64()}`;
}

/** Send webhook messages as signed HTTPS posts, after the Standard Webhooks specification. */
export function webhookProvider(
    send: (url: string, init: RequestInit) => Promise<Response> = fetch,
): MessageProvider {
    return {
        channel: "webhook",
        async send(message): Promise<Outcome> {
            // require a webhook message with its endpoint's secret
            const { to, content } = message;
            if (
                to.channel !== "webhook" ||
                content.channel !== "webhook" ||
                message.secret === null
            ) {
                throw new TypeError(`message ${message.id} is no signed webhook message`);
            }

            // sign the identifier, time and body with the endpoint's secret
            const body = JSON.stringify({
                type: content.type,
                timestamp: new Date(message.createdAt).toISOString(),
                data: content.data,
            });
            const timestamp = String(Math.floor(Date.now() / 1000));
            const signature = await WebhookSignature.sign(
                message.secret,
                message.id,
                timestamp,
                body,
            );

            // post it, reading the status as sent, retried or failed
            try {
                const response = await send(to.url, {
                    method: "POST",
                    headers: {
                        "content-type": "application/json",
                        "webhook-id": message.id,
                        "webhook-timestamp": timestamp,
                        "webhook-signature": signature,
                    },
                    body,
                    signal: AbortSignal.timeout(TIMEOUT_MILLISECONDS),
                });
                const error = { code: String(response.status), message: response.statusText };
                if (response.ok) {
                    return { outcome: "sent" };
                } else if (response.status >= 500 || RETRIED_STATUSES.has(response.status)) {
                    return { outcome: "retry", error };
                }

                return { outcome: "failed", error };
            } catch (error) {
                // retry an endpoint that did not answer
                return {
                    outcome: "retry",
                    error: {
                        code: "unreachable",
                        message: error instanceof Error ? error.message : String(error),
                    },
                };
            }
        },
    };
}

/** The signatures of webhook messages, after Standard Webhooks. */
export const WebhookSignature = {
    /** Sign a message's identifier, time and body with an endpoint's secret, as the `webhook-signature` header carries it. */
    async sign(secret: string, id: string, timestamp: string, body: string): Promise<string> {
        // key HMAC-SHA256 with the secret's bytes
        const key = await crypto.subtle.importKey(
            "raw",
            Uint8Array.fromBase64(secret.slice(SECRET_PREFIX.length)),
            { name: "HMAC", hash: "SHA-256" },
            false,
            ["sign"],
        );

        // sign the identifier, time and body joined by dots
        const content = new TextEncoder().encode(`${id}.${timestamp}.${body}`);
        const signed = await crypto.subtle.sign("HMAC", key, content);

        return `v1,${new Uint8Array(signed).toBase64()}`;
    },
};
