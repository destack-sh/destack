import { ServiceError } from "../error/index.ts";
import {
    WebhookDelivery,
    WebhookSignature,
    type WebhookMessage,
    type WebhookParameters,
} from "../trigger/index.ts";

/** The prefix of GitHub's HMAC-SHA256 signature header value. */
const SIGNATURE_PREFIX = "sha256=";

/** The GitHub signature over the body. */
export class GitHubSignature implements WebhookSignature {
    /** The scheme's name. */
    readonly name: string;

    /** Name the GitHub scheme. */
    constructor() {
        this.name = "github";
    }

    /** Sign the body. */
    async sign(message: WebhookMessage, secret: string): Promise<Headers> {
        // sign the body alone
        const key = await WebhookSignature.key(new TextEncoder().encode(secret), "sign");
        const content = new TextEncoder().encode(message.body);
        const digest = new Uint8Array(await crypto.subtle.sign("HMAC", key, content));

        return new Headers({
            "x-github-delivery": message.id,
            "x-github-event": message.event,
            "x-hub-signature-256": `${SIGNATURE_PREFIX}${digest.toHex()}`,
        });
    }

    /** Verify the body's signature and read the delivery. */
    async verify(
        request: Request,
        secret: string,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery> {
        // require the headers
        const event = request.headers.get("x-github-event");
        const signature = request.headers.get("x-hub-signature-256");
        if (event === null || signature === null) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing github webhook headers" });
        }

        // match the signature
        const body = await request.text();
        const key = await WebhookSignature.key(new TextEncoder().encode(secret), "verify");
        const digest = signature.startsWith(SIGNATURE_PREFIX)
            ? decodeHex(signature.slice(SIGNATURE_PREFIX.length))
            : undefined;
        const content = new TextEncoder().encode(body);
        if (digest === undefined || !(await crypto.subtle.verify("HMAC", key, digest, content))) {
            throw new ServiceError("UNAUTHORIZED", { message: "webhook signature does not match" });
        }

        // know a GitHub delivery by its signed body
        return WebhookDelivery.parse({
            id: digest.toHex(),
            event,
            payload: WebhookSignature.payload(body),
            parameters,
            receivedAt: now,
        });
    }
}

/** Decode a hexadecimal signature. */
function decodeHex(value: string): Uint8Array<ArrayBuffer> | undefined {
    try {
        return Uint8Array.fromHex(value);
    } catch {
        return undefined;
    }
}
