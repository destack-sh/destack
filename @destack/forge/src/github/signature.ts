import { CallError } from "@destack/runtime/error";
import { RequestBody } from "@destack/runtime/request";
import {
    WebhookDelivery,
    WebhookSignature,
    type WebhookMessage,
    type WebhookParameters,
} from "@destack/runtime/trigger";

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
        secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery> {
        // require the headers
        const event = request.headers.get("x-github-event");
        const signature = request.headers.get("x-hub-signature-256");
        if (event === null || signature === null) {
            throw new CallError("UNAUTHORIZED", { message: "missing github webhook headers" });
        }

        // match the signature
        const body = await RequestBody.text(request, "a webhook delivery");
        const shared = WebhookSignature.secret(secret, this.name);
        const key = await WebhookSignature.key(new TextEncoder().encode(shared), "verify");
        const digest = signature.startsWith(SIGNATURE_PREFIX)
            ? GitHubSignature.#decodeHex(signature.slice(SIGNATURE_PREFIX.length))
            : undefined;
        const content = new TextEncoder().encode(body);
        if (digest === undefined || !(await crypto.subtle.verify("HMAC", key, digest, content))) {
            throw new CallError("UNAUTHORIZED", { message: "webhook signature is invalid" });
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

    /** Decode a hexadecimal signature. */
    static #decodeHex(value: string): Uint8Array<ArrayBuffer> | undefined {
        try {
            return Uint8Array.fromHex(value);
        } catch {
            return undefined;
        }
    }
}
