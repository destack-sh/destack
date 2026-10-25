import type { Crypto } from "@destack/environment";
import { CallError, type Fetch, type StandardMessage, StandardWebhook } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import { SecretReading } from "@destack/vault";
import { secret } from "@destack/vault/object";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { encodeBody, type WebhookAuthentication } from "./message.ts";
import { type MessageProvider, Outcome } from "../provider/index.ts";
import type { ObjectStore } from "@destack/object/server";
import type { MessageReason } from "../error/error.ts";

/** How long a webhook endpoint may take before the send retries: Standard Webhooks' 15 seconds. */
const TIMEOUT_MILLISECONDS = 15_000;

/** The media type of each webhook body format. */
const MEDIA_TYPES = {
    event: "application/json",
    json: "application/json",
    ndjson: "application/x-ndjson",
} as const;

/** Send webhook messages as HTTPS posts to their endpoints, authenticated with its vault secret. */
export function webhookProvider(fetch: Fetch): MessageProvider {
    return {
        async send(message, store): Promise<Outcome> {
            // require a webhook message
            const { to, content } = message;
            if (to.channel !== "webhook" || content.channel !== "webhook") {
                throw new TypeError(`message ${message.id} is not a webhook message`);
            }

            // fail a message whose endpoint is gone, in the trash, or disabled
            const target = await Endpoint.read(store, message.source).catch(refused);
            if (target === undefined || target.deletedAt !== null) {
                return failed("RECIPIENT_GONE", `endpoint ${message.source.id} is gone`);
            } else if (target.status === "disabled") {
                return failed("RECIPIENT_DISABLED", `endpoint ${target.id} is disabled`);
            }

            // fail a message whose endpoint's secret is unreadable, as the installation consuming it
            const value = await readSecret(store, target.secret).catch(refused);
            if (value === undefined) {
                return failed("SECRET_UNAVAILABLE", `endpoint ${target.id} has no readable secret`);
            }

            // encode the body, then sign it with the secret or put the secret in the endpoint's header
            const body = encodeBody(message.createdAt, content);
            const timestamp = Math.floor(store.environment.clock.now() / 1000);
            const headers = await authenticate(
                target.authentication,
                value,
                store.environment.crypto,
                {
                    id: message.id,
                    timestamp,
                    body,
                },
            );
            headers.set("content-type", MEDIA_TYPES[content.format]);

            // post it within the timeout without following redirects, and read the answer as an outcome
            const request = new Request(target.url, {
                method: "POST",
                headers,
                body,
                redirect: "manual",
                signal: store.environment.clock.timeout(TIMEOUT_MILLISECONDS),
            });

            return Outcome.post(fetch, request, store.environment.clock);
        },
    };
}

/** Build a webhook post's Standard Webhooks headers, signed, or with the secret in a header. */
async function authenticate(
    authentication: WebhookAuthentication,
    value: string,
    crypto: Crypto,
    post: StandardMessage,
): Promise<Headers> {
    // put the secret in the endpoint's header beside the identifier and time
    if (authentication.kind === "header") {
        return new Headers({
            [authentication.name]: value,
            "webhook-id": post.id,
            "webhook-timestamp": String(post.timestamp),
        });
    }

    // sign the identifier, time and body
    const webhook = await StandardWebhook.of(value, crypto);

    return webhook.sign(post);
}

/** Read a refusal as absent, passing on any failure a later try may get past. */
function refused(error: unknown): undefined {
    if (CallError.refusal(error) === undefined) {
        throw error;
    }

    return undefined;
}

/** Fail a message for good with a reason. */
function failed(reason: MessageReason, message: string): Outcome {
    return { kind: "failed", refusal: { reason, message } };
}

/** Read a vault secret's text as the code serving the endpoints, which consumes it. */
function readSecret(
    store: Pick<ObjectStore, "query" | "as">,
    reference: ObjectReference,
): Promise<string> {
    const target = secret.target({ scope: reference.scope, id: secret.identifier(reference.id) });

    // read as the installation serving the endpoints
    return store
        .query(store.as(endpoint), secret.call("read", target))
        .then((read) => SecretReading.text(SecretReading.parse(read)));
}
