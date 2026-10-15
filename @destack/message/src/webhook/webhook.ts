import type { Fetch } from "@destack/runtime";
import { ObjectReference } from "@destack/sync";
import { Secret } from "@destack/vault";
import { secret } from "@destack/vault/object";
import { Endpoint } from "../object/endpoint.ts";
import { encodeBody, type WebhookAuthentication } from "./message.ts";
import { type MessageProvider, Outcome } from "../provider/index.ts";
import type { ObjectStore } from "@destack/object/server";

/** The prefix of a Standard Webhooks signing secret. */
const SECRET_PREFIX = "whsec_";

/** How long a webhook endpoint may take before the send retries: Standard Webhooks' 15 seconds. */
const TIMEOUT_MILLISECONDS = 15_000;

/** The media type of each webhook body format. */
const MEDIA_TYPES = {
    event: "application/json",
    json: "application/json",
    ndjson: "application/x-ndjson",
} as const;

/** Send webhook messages as HTTPS posts to their endpoints, authenticated with its vault secret. */
export function webhookProvider(fetch: Fetch = globalThis.fetch): MessageProvider {
    return {
        async send(message, store): Promise<Outcome> {
            // require a webhook message
            const { to, content } = message;
            if (to.channel !== "webhook" || content.channel !== "webhook") {
                throw new TypeError(`message ${message.id} is not a webhook message`);
            }

            // read the endpoint the message's source names, and its secret as the installation consuming it
            const target = await Endpoint.read(store, message.source);
            const value = await readSecret(store, target.secret);

            // encode the body, then sign it with the secret or put the secret in the endpoint's header
            const body = encodeBody(message.id, message.createdAt, content);
            const timestamp = String(Math.floor(store.clock.now() / 1000));
            const authentication = await authenticate(target.authentication, value, {
                id: message.id,
                timestamp,
                body,
            });

            // post it and read the answer as an outcome
            const request = new Request(target.url, {
                method: "POST",
                headers: {
                    ...authentication,
                    "content-type": MEDIA_TYPES[content.format],
                    "webhook-id": message.id,
                    "webhook-timestamp": timestamp,
                },
                body,
                signal: AbortSignal.timeout(TIMEOUT_MILLISECONDS),
            });
            try {
                return Outcome.read(await fetch(request), store.clock.now());
            } catch (error) {
                // retry an endpoint the network failed to connect to or that did not answer in time
                if (isUnreachable(error)) {
                    return {
                        kind: "retry",
                        error: { code: "unreachable", message: error.message },
                    };
                }
                throw error;
            }
        },
    };
}

/** The signatures of webhook messages, after Standard Webhooks. */
export const WebhookSignature = {
    /** Sign a message's identifier, time and body with a secret, for `webhook-signature`. */
    async sign(signing: string, id: string, timestamp: string, body: string): Promise<string> {
        // key HMAC-SHA256 with the secret's bytes
        const key = await crypto.subtle.importKey(
            "raw",
            Uint8Array.fromBase64(signing.slice(SECRET_PREFIX.length)),
            { name: "HMAC", hash: "SHA-256" },
            false,
            ["sign"],
        );

        // sign the identifier and time and body joined by dots
        const content = new TextEncoder().encode(`${id}.${timestamp}.${body}`);
        const signed = await crypto.subtle.sign("HMAC", key, content);

        return `v1,${new Uint8Array(signed).toBase64()}`;
    },
};

/** Build the header authenticating a webhook post: a signature, or the secret in a header. */
async function authenticate(
    authentication: WebhookAuthentication,
    value: string,
    post: { readonly id: string; readonly timestamp: string; readonly body: string },
): Promise<Record<string, string>> {
    // put the secret in the endpoint's header
    if (authentication.kind === "header") {
        return { [authentication.name]: value };
    }

    // sign the identifier, time and body
    const signature = await WebhookSignature.sign(value, post.id, post.timestamp, post.body);

    return { "webhook-signature": signature };
}

/** Decide whether a fetch failed on the network or ran out of time. */
function isUnreachable(error: unknown): error is TypeError | DOMException {
    return (
        error instanceof TypeError ||
        (error instanceof DOMException && error.name === "TimeoutError")
    );
}

/** Read a vault secret's text as the store's principal, which consumes it. */
function readSecret(
    store: Pick<ObjectStore, "query" | "self">,
    reference: ObjectReference,
): Promise<string> {
    return Secret.text({
        read: () => {
            const target = secret.target({
                scope: reference.scope,
                id: secret.identifier(reference.id),
            });

            return store.query(store.self, secret.call("read", target));
        },
    });
}
