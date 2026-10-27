import type { Crypto } from "@destack/environment";
import type { InstalledServerContext } from "@destack/object/workload";
import { HmacKey, ObjectError, RequestBody } from "@destack/object";
import { present } from "@destack/schema";
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
    /** Whether the scheme verifies with a shared secret, which it does. */
    readonly isKeyed: boolean;

    /** Create the GitHub scheme. */
    constructor() {
        this.name = "github";
        this.isKeyed = true;
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

    /** Verify deliveries alike for every installation, on its environment's crypto. */
    verifier(
        context: Pick<InstalledServerContext, "resources" | "keys" | "environment">,
    ): WebhookVerifier {
        return (request, secret, parameters, now) =>
            this.verify(request, secret, parameters, now, context.environment.crypto);
    }

    /** Verify the body's signature and read the delivery. */
    async verify(
        request: Request,
        secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // require the signature
        const signature = request.headers.get("x-hub-signature-256");
        if (signature === null) {
            throw ObjectError.of("SIGNATURE_MISSING", "missing github webhook signature");
        }

        // match it over the body's exact bytes
        const body = await RequestBody.bytes(request, "a webhook delivery");
        const key = await HmacKey.of(
            new TextEncoder().encode(present(secret, "the trigger's secret")),
            crypto,
        );
        const digest = signature.startsWith(SIGNATURE_PREFIX)
            ? GitHubSignature.#decodeHex(signature.slice(SIGNATURE_PREFIX.length))
            : undefined;
        if (digest === undefined || !(await key.verify(digest, body))) {
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
