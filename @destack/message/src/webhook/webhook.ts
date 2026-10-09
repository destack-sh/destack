import type { CallServer } from "@destack/object";
import type { schema } from "@destack/schema";
import type { Fetch } from "@destack/service";
import { type ObjectReference, Subject } from "@destack/sync";
import { encodeBody, WebhookDestination } from "./message.ts";
import { type MessageProvider, Outcome } from "../provider/index.ts";

/** The prefix of a Standard Webhooks signing secret. */
const SECRET_PREFIX = "whsec_";

/** How long a webhook endpoint takes to answer before the send is retried: Standard Webhooks' 15 seconds. */
const TIMEOUT_MILLISECONDS = 15_000;

/** The media type of each webhook body format. */
const MEDIA_TYPES = {
    event: "application/json",
    json: "application/json",
    ndjson: "application/x-ndjson",
} as const;

/** Send webhook messages as HTTPS posts authenticated with the vault secret of each: signed after the Standard Webhooks specification, or with the secret's value in a header. */
export function webhookProvider(
    readSecret: (
        server: Pick<CallServer, "query" | "clock">,
        secret: ObjectReference,
        reader: Subject,
    ) => Promise<string>,
    fetch: Fetch = globalThis.fetch,
): MessageProvider {
    return {
        async send(message, server): Promise<Outcome> {
            // require a webhook message
            const { to, content } = message;
            if (to.channel !== "webhook" || content.channel !== "webhook") {
                throw new TypeError(`message ${message.id} is no webhook message`);
            }

            // encode the body and read the secret as the message's reader
            const body = encodeBody(message.id, message.createdAt, content);
            const value = await readSecret(server, to.secret, to.reader);

            // sign the body with the secret or put the secret in the endpoint's header
            const timestamp = String(Math.floor(server.clock() / 1000));
            const authentication = await authenticate(to, value, {
                id: message.id,
                timestamp,
                body,
            });

            // post it and read the answer as an outcome
            const request = new Request(to.url, {
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
                return Outcome.read(await fetch(request), server.clock());
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
    /** Sign a message's identifier, time and body with an endpoint's secret, for the `webhook-signature` header. */
    async sign(secret: string, id: string, timestamp: string, body: string): Promise<string> {
        // key HMAC-SHA256 with the secret's bytes
        const key = await crypto.subtle.importKey(
            "raw",
            Uint8Array.fromBase64(secret.slice(SECRET_PREFIX.length)),
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

/** Build the header authenticating a webhook post: its signature, or the secret's value in the endpoint's header. */
async function authenticate(
    to: Pick<schema.Infer<typeof WebhookDestination>, "authentication">,
    secret: string,
    post: { readonly id: string; readonly timestamp: string; readonly body: string },
): Promise<Record<string, string>> {
    // put the secret in the endpoint's header
    if (to.authentication.kind === "header") {
        return { [to.authentication.name]: secret };
    }

    // sign the identifier, time and body
    const signature = await WebhookSignature.sign(secret, post.id, post.timestamp, post.body);

    return { "webhook-signature": signature };
}

/** Decide whether a fetch failed on the network, as the Fetch Standard's TypeError, or ran out of time, as an AbortSignal's TimeoutError. */
function isUnreachable(error: unknown): error is TypeError | DOMException {
    return (
        error instanceof TypeError ||
        (error instanceof DOMException && error.name === "TimeoutError")
    );
}
