import { schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { WebhookDelivery, type WebhookParameters } from "./delivery.ts";
import type { Webhook } from "./webhook.ts";

/** The Standard Webhooks timestamp tolerance, five minutes as in the reference libraries. */
const STANDARD_TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The prefix of a serialized Standard Webhooks symmetric secret. */
const STANDARD_SECRET_PREFIX = "whsec_";

/** The version tag of a Standard Webhooks HMAC-SHA256 signature. */
const STANDARD_SIGNATURE_VERSION = "v1";

/** The prefix of GitHub's HMAC-SHA256 signature header value. */
const GITHUB_SIGNATURE_PREFIX = "sha256=";

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

/** A webhook signature scheme. */
export interface WebhookSignature {
    /** Sign a message, returning the headers. */
    sign(message: WebhookMessage, secret: string): Promise<Headers>;
    /** Verify a request's signature and read its delivery. */
    verify(
        request: Request,
        secret: string,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery>;
}

/** The Standard Webhooks signature over `id.timestamp.body`. */
export class StandardSignature implements WebhookSignature {
    /** Sign the identifier, timestamp and body. */
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

    /** Verify a v1 signature within the timestamp tolerance. */
    async verify(
        request: Request,
        secret: string,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery> {
        // require the headers and a current timestamp
        const id = request.headers.get("webhook-id");
        const timestamp = request.headers.get("webhook-timestamp");
        const signatures = request.headers.get("webhook-signature");
        if (id === null || timestamp === null || signatures === null) {
            throw new ServiceError("UNAUTHORIZED", { message: "missing standard webhook headers" });
        }
        const sentAt = Number(timestamp) * 1000;
        if (!/^\d+$/.test(timestamp) || Math.abs(now - sentAt) > STANDARD_TOLERANCE_MILLISECONDS) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "webhook timestamp is outside the tolerance",
            });
        }

        // match any v1 signature
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

        // read the event type
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

        return WebhookDelivery.parse({
            id,
            event: event.data.type,
            payload,
            parameters,
            receivedAt: now,
        });
    }

    /** Import a whsec_ secret as an HMAC key. */
    static #key(secret: string, usage: "sign" | "verify"): Promise<CryptoKey> {
        if (!secret.startsWith(STANDARD_SECRET_PREFIX)) {
            throw new TypeError("standard webhook secrets start with whsec_");
        }

        return hmacKey(Uint8Array.fromBase64(secret.slice(STANDARD_SECRET_PREFIX.length)), usage);
    }
}

/** The GitHub signature over the body. */
export class GitHubSignature implements WebhookSignature {
    /** Sign the body. */
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
        const key = await hmacKey(new TextEncoder().encode(secret), "verify");
        const digest = signature.startsWith(GITHUB_SIGNATURE_PREFIX)
            ? decodeHex(signature.slice(GITHUB_SIGNATURE_PREFIX.length))
            : undefined;
        const content = new TextEncoder().encode(body);
        if (digest === undefined || !(await crypto.subtle.verify("HMAC", key, digest, content))) {
            throw new ServiceError("UNAUTHORIZED", { message: "webhook signature does not match" });
        }

        // know the delivery by its signed body, since GitHub signs no delivery header
        return WebhookDelivery.parse({
            id: digest.toHex(),
            event,
            payload: parsePayload(body),
            parameters,
            receivedAt: now,
        });
    }
}

/** The signature scheme of each verification. */
export const WEBHOOK_SIGNATURES: Readonly<Record<Webhook["verification"], WebhookSignature>> = {
    standard: new StandardSignature(),
    github: new GitHubSignature(),
};

/** Import raw bytes as an HMAC SHA-256 key. */
function hmacKey(bytes: Uint8Array<ArrayBuffer>, usage: "sign" | "verify"): Promise<CryptoKey> {
    return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [usage]);
}

/** Decode a base64 signature. */
function decodeBase64(value: string | undefined): Uint8Array<ArrayBuffer> | undefined {
    try {
        return value === undefined ? undefined : Uint8Array.fromBase64(value);
    } catch {
        return undefined;
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

/** Parse a verified body as JSON. */
function parsePayload(body: string) {
    try {
        return schema.json().parse(JSON.parse(body));
    } catch {
        throw new ServiceError("BAD_REQUEST", { message: "webhook body is not JSON" });
    }
}
