import type { Crypto, Environment } from "@destack/environment";
import { HmacKey, ObjectError, RequestBody } from "@destack/object";
import { schema } from "@destack/schema";
import {
    WebhookDelivery,
    type WebhookMessage,
    type WebhookParameters,
    WebhookSignature,
    type WebhookVerifier,
} from "@destack/workflow";

/** How far a signature's timestamp may lie from now: five minutes, Stripe's default tolerance. */
const TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The scheme tag of Stripe's HMAC-SHA256 signatures in the `Stripe-Signature` header. */
const SCHEME = "v1";

/** The event a Stripe delivery's body has: its identifier and type. */
const StripeDeliveryEvent = schema.looseObject({
    id: schema.string().min(1),
    type: schema.string().min(1),
});

/** Stripe's webhook signature over `timestamp.body`, keyed by the endpoint's secret. */
export class StripeSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name: string;

    /** Create the Stripe scheme. */
    constructor() {
        this.name = "stripe";
    }

    /** Sign the sending time in whole seconds and the body, as Stripe signs its deliveries. */
    async sign(message: WebhookMessage, secret: string, crypto: Crypto): Promise<Headers> {
        // sign the timestamp and the body with the secret's bytes
        const timestamp = String(Math.floor(message.sentAt / 1000));
        const key = await HmacKey.of(new TextEncoder().encode(secret), crypto);
        const digest = await key.sign(new TextEncoder().encode(`${timestamp}.${message.body}`));

        return new Headers({ "stripe-signature": `t=${timestamp},${SCHEME}=${digest.toHex()}` });
    }

    /** Verify deliveries on an environment's crypto. */
    verifier(environment: Environment): WebhookVerifier {
        return (request, secrets, parameters, now) =>
            this.verify(request, secrets, parameters, now, environment.crypto);
    }

    /** Verify a delivery's signature by any secret within the tolerance, and read its event. */
    async verify(
        request: Request,
        secrets: readonly [string, ...string[]],
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // read the timestamp and the v1 signatures, refusing a header without them
        const header = request.headers.get("stripe-signature") ?? "";
        const pairs = header.split(",").map((pair) => pair.split("="));
        const timestamp = pairs.find(([name]) => name === "t")?.[1];
        const signatures = pairs.filter(([name]) => name === SCHEME).map(([, value]) => value);
        if (timestamp === undefined || signatures.length === 0) {
            throw ObjectError.of("SIGNATURE_MISSING", "missing stripe webhook signature");
        }

        // refuse a timestamp outside the tolerance
        const sentAt = Number(timestamp) * 1000;
        if (!/^\d+$/u.test(timestamp) || Math.abs(now - sentAt) > TOLERANCE_MILLISECONDS) {
            throw ObjectError.of(
                "TIMESTAMP_OUTSIDE_TOLERANCE",
                "webhook timestamp is outside the tolerance",
            );
        }

        // match any v1 signature over the timestamp and the body's exact bytes, by any secret
        const body = new TextDecoder().decode(
            await RequestBody.bytes(request, "a webhook delivery"),
        );
        const keys = await Promise.all(
            secrets.map(async (secret) => HmacKey.of(new TextEncoder().encode(secret), crypto)),
        );
        const signed = new TextEncoder().encode(`${timestamp}.${body}`);
        let isSigned = false;
        for (const signature of signatures) {
            const digest = StripeSignature.#decodeHex(signature);
            for (const key of keys) {
                isSigned ||= digest !== undefined && (await key.verify(digest, signed));
            }
        }
        if (!isSigned) {
            throw ObjectError.of("SIGNATURE_MISMATCH", "webhook signature does not match");
        }

        // read the event's identifier and type
        const payload = WebhookSignature.payload(body);
        const event = StripeDeliveryEvent.parse(payload);

        return WebhookDelivery.parse({
            id: event.id,
            event: event.type,
            payload,
            parameters,
            receivedAt: now,
        });
    }

    /** Decode a hexadecimal signature, absent when malformed. */
    static #decodeHex(value: string | undefined): Uint8Array<ArrayBuffer> | undefined {
        try {
            return value === undefined ? undefined : Uint8Array.fromHex(value);
        } catch {
            return undefined;
        }
    }
}
