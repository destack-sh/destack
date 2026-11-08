import type { Crypto, Environment } from "@destack/environment";
import { StandardWebhook } from "@destack/object";
import { schema, type JsonValue } from "@destack/schema";
import { WorkflowError } from "../error/index.ts";
import { WebhookDelivery, type WebhookParameters } from "./webhook.ts";

/** A signed webhook message. */
export interface WebhookMessage {
    /** The delivery identifier. */
    readonly id: string;
    /** The event type. */
    readonly event: string;
    /** The exact body bytes as text. */
    readonly body: string;
    /** The sending time, in UTC epoch milliseconds. */
    readonly sentAt: number;
}

/** Verify a request's signature by any of a webhook's secrets, and read its delivery. */
export type WebhookVerifier = (
    request: Request,
    secrets: readonly [string, ...string[]],
    parameters: WebhookParameters,
    now: number,
) => Promise<WebhookDelivery>;

/** A webhook signature scheme. */
export interface WebhookSignature {
    /** The scheme's name, which the manifest lists as a webhook's verification. */
    readonly name: string;
    /** Sign a message with the secret it shares with the sender, returning the headers. */
    readonly sign: (message: WebhookMessage, secret: string, crypto: Crypto) => Promise<Headers>;
    /** Build the verifier of an installation's deliveries in the installation's environment. */
    readonly verifier: (environment: Environment) => WebhookVerifier;
}

/** What every signature scheme reads. */
export const WebhookSignature = {
    /** Parse a verified body as JSON. */
    payload(body: string): JsonValue {
        try {
            return schema.json().parse(JSON.parse(body));
        } catch {
            throw WorkflowError.of("BODY_INVALID", "webhook body is not JSON");
        }
    },
};

/** The Standard Webhooks signature over `id.timestamp.body`. */
export class StandardSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name: string;

    /** Create the Standard Webhooks scheme. */
    constructor() {
        this.name = "standard";
    }

    /** Sign the identifier, sending time in whole seconds and body. */
    async sign(message: WebhookMessage, secret: string, crypto: Crypto): Promise<Headers> {
        const timestamp = Math.floor(message.sentAt / 1000);
        const webhook = await StandardWebhook.of([secret], crypto);

        return webhook.sign({ id: message.id, timestamp, body: message.body });
    }

    /** Verify deliveries on an environment's crypto. */
    verifier(environment: Environment): WebhookVerifier {
        return (request, secrets, parameters, now) =>
            this.verify(request, secrets, parameters, now, environment.crypto);
    }

    /** Verify a delivery's signature by any secret, and read its event type. */
    async verify(
        request: Request,
        secrets: readonly [string, ...string[]],
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // verify the signature with the webhook's secrets
        const webhook = await StandardWebhook.of(secrets, crypto);
        const { id, body } = await webhook.verify(request, now);

        // read the event type
        const payload = WebhookSignature.payload(body);
        const event = schema
            .object({ type: schema.string().min(1) })
            .loose()
            .safeParse(payload);
        if (!event.success) {
            throw WorkflowError.of("EVENT_TYPE_MISSING", "standard webhook payload has no type");
        }

        return WebhookDelivery.parse({
            id,
            event: event.data.type,
            payload,
            parameters,
            receivedAt: now,
        });
    }
}
