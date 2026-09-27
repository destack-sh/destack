import { schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { WebhookDelivery, type Webhook } from "./webhook.ts";

/** How far a Standard Webhooks timestamp may lie from the receiver's clock, the five minutes the reference libraries allow. */
const STANDARD_TOLERANCE_MS = 5 * 60 * 1000;

/** The prefix of a serialized Standard Webhooks symmetric secret. */
const STANDARD_SECRET_PREFIX = "whsec_";

/** The version tag of a Standard Webhooks HMAC-SHA256 signature. */
const STANDARD_SIGNATURE_VERSION = "v1";

/** The prefix of GitHub's HMAC-SHA256 signature header value. */
const GITHUB_SIGNATURE_PREFIX = "sha256=";

/** One message a sender signs: its identifier, event, exact body and sending time. */
export interface WebhookMessage {
    /** The delivery identifier. */
    readonly id: string;
    /** The event type. */
    readonly event: string;
    /** The exact body bytes as text. */
    readonly body: string;
    /** The sending time in UTC epoch milliseconds. */
    readonly sentAt: number;
}

/** A scheme proving that a delivery came from the holder of the webhook's secret. */
export interface WebhookSignature {
    /** Sign a message as its sender does, returning the headers carrying identity and signature. */
    sign(message: WebhookMessage, secret: string): Promise<Headers>;
    /** Verify a received request's signature and read its delivery, refusing forged or stale ones. */
    verify(request: Request, secret: string, now: number): Promise<WebhookDelivery>;
}

/** Standard Webhooks: webhook-id, webhook-timestamp and webhook-signature over `id.timestamp.body`. */
export class StandardSignature implements WebhookSignature {
    /** Sign the identifier, timestamp in seconds and body with the base64 secret. */
    async sign(message: WebhookMessage, secret: string): Promise<Headers> {
        // sign the content in whole seconds
        const timestamp = Math.floor(message.sentAt / 1000);
        const key = await StandardSignature.#key(secret, "sign");
        const content = new TextEncoder().encode(`${message.id}.${timestamp}.${message.body}`);
        const digest = new Uint8Array(await crypto.subtle.sign("HMAC", key, content));

        return new Headers({
            "webhook-id": message.id,
            "webhook-timestamp": String(timestamp),
            "webhook-signature": `${STANDARD_SIGNATURE_VERSION},${digest.toBase64()}`,
        });
    }

    /** Verify one of the request's v1 signatures within the timestamp tolerance, reading the event from the payload's type. */
    async verify(request: Request, secret: string, now: number): Promise<WebhookDelivery> {
        // require the three headers and a timestamp within the tolerance
        const id = request.headers.get("webhook-id");
        const timestamp = request.headers.get("webhook-timestamp");
        const signatures = request.headers.get("webhook-signature");
        if (id === null || timestamp === null || signatures === null) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing standard webhook headers" });
        }
        const sentAt = Number(timestamp) * 1000;
        if (!/^\d+$/.test(timestamp) || Math.abs(now - sentAt) > STANDARD_TOLERANCE_MS) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "webhook timestamp is outside the tolerance",
            });
        }

        // accept the body when any v1 signature matches it
        const body = await request.text();
        const key = await StandardSignature.#key(secret, "verify");
        const content = new TextEncoder().encode(`${id}.${timestamp}.${body}`);
        let isSigned = false;
        for (const entry of signatures.split(" ")) {
            const [version, signature] = entry.split(",");
            const digest =
                version === STANDARD_SIGNATURE_VERSION ? decodeBase64(signature) : undefined;
            isSigned ||=
                digest !== undefined && (await crypto.subtle.verify("HMAC", key, digest, content));
        }
        if (!isSigned) {
            throw new ServiceError("UNAUTHORIZED", { message: "webhook signature does not match" });
        }

        // read the event type the payload names
        const payload = parsePayload(body);
        const event = schema
            .object({ type: schema.string().min(1) })
            .passthrough()
            .safeParse(payload);
        if (!event.success) {
            throw new ServiceError("BAD_REQUEST", {
                message: "standard webhook payload names no type",
            });
        }

        return WebhookDelivery.parse({ id, event: event.data.type, payload, receivedAt: now });
    }

    /** Import a serialized whsec_ secret as its HMAC key. */
    static #key(secret: string, usage: "sign" | "verify"): Promise<CryptoKey> {
        if (!secret.startsWith(STANDARD_SECRET_PREFIX)) {
            throw new TypeError("standard webhook secrets start with whsec_");
        }

        return hmacKey(Uint8Array.fromBase64(secret.slice(STANDARD_SECRET_PREFIX.length)), usage);
    }
}

/** GitHub: x-hub-signature-256 over the body, with x-github-delivery and x-github-event. */
export class GitHubSignature implements WebhookSignature {
    /** Sign the body with the secret's UTF-8 bytes. */
    async sign(message: WebhookMessage, secret: string): Promise<Headers> {
        // sign the body alone
        const key = await hmacKey(new TextEncoder().encode(secret), "sign");
        const content = new TextEncoder().encode(message.body);
        const digest = new Uint8Array(await crypto.subtle.sign("HMAC", key, content));

        return new Headers({
            "x-github-delivery": message.id,
            "x-github-event": message.event,
            "x-hub-signature-256": `${GITHUB_SIGNATURE_PREFIX}${digest.toHex()}`,
        });
    }

    /** Verify the body's signature and read the delivery and event the headers name. */
    async verify(request: Request, secret: string, now: number): Promise<WebhookDelivery> {
        // require the delivery, event and signature headers
        const id = request.headers.get("x-github-delivery");
        const event = request.headers.get("x-github-event");
        const signature = request.headers.get("x-hub-signature-256");
        if (id === null || event === null || signature === null) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing github webhook headers" });
        }

        // accept the body when the signature matches it
        const body = await request.text();
        const key = await hmacKey(new TextEncoder().encode(secret), "verify");
        const digest = signature.startsWith(GITHUB_SIGNATURE_PREFIX)
            ? decodeHex(signature.slice(GITHUB_SIGNATURE_PREFIX.length))
            : undefined;
        const content = new TextEncoder().encode(body);
        if (digest === undefined || !(await crypto.subtle.verify("HMAC", key, digest, content))) {
            throw new ServiceError("UNAUTHORIZED", { message: "webhook signature does not match" });
        }

        return WebhookDelivery.parse({ id, event, payload: parsePayload(body), receivedAt: now });
    }
}

/** The signature scheme of each webhook verification. */
export const WEBHOOK_SIGNATURES: Readonly<Record<Webhook["verification"], WebhookSignature>> = {
    standard: new StandardSignature(),
    github: new GitHubSignature(),
};

/** Import raw bytes as an HMAC SHA-256 key. */
function hmacKey(bytes: Uint8Array<ArrayBuffer>, usage: "sign" | "verify"): Promise<CryptoKey> {
    return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [usage]);
}

/** Decode a base64 signature, absent when it is not base64. */
function decodeBase64(value: string | undefined): Uint8Array<ArrayBuffer> | undefined {
    try {
        return value === undefined ? undefined : Uint8Array.fromBase64(value);
    } catch {
        return undefined;
    }
}

/** Decode a hexadecimal signature, absent when it is not hexadecimal. */
function decodeHex(value: string): Uint8Array<ArrayBuffer> | undefined {
    try {
        return Uint8Array.fromHex(value);
    } catch {
        return undefined;
    }
}

/** Parse a verified body as JSON. */
function parsePayload(body: string) {
    try {
        return schema.json().parse(JSON.parse(body));
    } catch {
        throw new ServiceError("BAD_REQUEST", { message: "webhook body is not JSON" });
    }
}
