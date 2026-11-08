import type { Crypto, Environment } from "@destack/environment";
import { HmacKey, ObjectError, RequestBody } from "@destack/object";
import {
    WebhookDelivery,
    WebhookSignature,
    type WebhookMessage,
    type WebhookVerifier,
    type WebhookParameters,
} from "@destack/workflow";
import { ForgeError } from "../error/index.ts";

/** The prefix of GitHub's HMAC-SHA256 signature header value. */
const SIGNATURE_PREFIX = "sha256=";

/** The GitHub signature over the body. */
export class GitHubSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name: string;

    /** Create the GitHub scheme. */
    constructor() {
        this.name = "github";
    }

    /** Sign the body. */
    async sign(message: WebhookMessage, secret: string, crypto: Crypto): Promise<Headers> {
        // sign the body alone
        const key = await HmacKey.of(new TextEncoder().encode(secret), crypto);
        const digest = await key.sign(new TextEncoder().encode(message.body));

        return new Headers({
            "x-github-delivery": message.id,
            "x-github-event": message.event,
            "x-hub-signature-256": `${SIGNATURE_PREFIX}${digest.toHex()}`,
        });
    }

    /** Verify deliveries on an environment's crypto. */
    verifier(environment: Environment): WebhookVerifier {
        return (request, secrets, parameters, now) =>
            this.verify(request, secrets, parameters, now, environment.crypto);
    }

    /** Verify the body's signature by any secret and read the delivery. */
    async verify(
        request: Request,
        secrets: readonly [string, ...string[]],
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // require the signature
        const signature = request.headers.get("x-hub-signature-256");
        if (signature === null) {
            throw ObjectError.of("SIGNATURE_MISSING", "missing github webhook signature");
        }

        // match it over the body's exact bytes, by any secret
        const body = await RequestBody.bytes(request, "a webhook delivery");
        const keys = await Promise.all(
            secrets.map(async (secret) => HmacKey.of(new TextEncoder().encode(secret), crypto)),
        );
        const digest = signature.startsWith(SIGNATURE_PREFIX)
            ? GitHubSignature.#decodeHex(signature.slice(SIGNATURE_PREFIX.length))
            : undefined;
        let isSigned = false;
        for (const key of keys) {
            isSigned ||= digest !== undefined && (await key.verify(digest, body));
        }
        if (digest === undefined || !isSigned) {
            throw ObjectError.of("SIGNATURE_MISMATCH", "webhook signature does not match");
        }

        // require the event type
        const event = request.headers.get("x-github-event");
        if (event === null) {
            throw ForgeError.of("EVENT_TYPE_MISSING", "github webhook has no event type");
        }

        // know a GitHub delivery by its signed body
        return WebhookDelivery.parse({
            id: digest.toHex(),
            event,
            payload: WebhookSignature.payload(new TextDecoder().decode(body)),
            parameters,
            receivedAt: now,
        });
    }

    /** Decode a hexadecimal signature. */
    static #decodeHex(value: string): Uint8Array<ArrayBuffer> | undefined {
        try {
            return Uint8Array.fromHex(value);
        } catch {
            return undefined;
        }
    }
}
