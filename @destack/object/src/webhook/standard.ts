import type { Crypto } from "@destack/environment";
import { ObjectError } from "../error/error.ts";
import { RequestBody } from "../server/body.ts";
import { HmacKey } from "./hmac.ts";
import { nth } from "@destack/schema";

/** How far a timestamp may lie from now: five minutes, as the reference libraries allow. */
const TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The prefix of a serialized Standard Webhooks symmetric secret. */
const SECRET_PREFIX = "whsec_";

/** The version tag of a Standard Webhooks HMAC-SHA256 signature. */
const SIGNATURE_VERSION = "v1";

/** A Standard Webhooks message: its identifier, its sending time and its exact body. */
export interface StandardMessage {
    /** The message identifier, sent as `webhook-id`. */
    readonly id: string;
    /** The sending time in whole seconds since the epoch, sent as `webhook-timestamp`. */
    readonly timestamp: number;
    /** The exact body bytes as text. */
    readonly body: string;
}

/** A webhook's Standard Webhooks secrets, several while one rotates, over `id.timestamp.body`. */
export class StandardWebhook {
    /** The secrets' HMAC keys, the current one first. */
    readonly #keys: readonly [HmacKey, ...HmacKey[]];

    /** Wrap the secrets' keys. */
    private constructor(keys: readonly [HmacKey, ...HmacKey[]]) {
        this.#keys = keys;
    }

    /** Import `whsec_` secrets on an environment's WebCrypto, refusing a malformed one. */
    static async of(
        secrets: readonly [string, ...string[]],
        crypto: Crypto,
    ): Promise<StandardWebhook> {
        // import each secret's key, the current one first
        const keys = await Promise.all(
            secrets.map(async (secret) => StandardWebhook.#key(secret, crypto)),
        );

        return new StandardWebhook([nth(keys, 0), ...keys.slice(1)]);
    }

    /** Import a `whsec_` secret's key, refusing a malformed one. */
    static async #key(secret: string, crypto: Crypto): Promise<HmacKey> {
        // require the prefix and decode the key bytes
        if (!secret.startsWith(SECRET_PREFIX)) {
            throw ObjectError.of(
                "SECRET_BLOCKED",
                "cannot verify standard webhooks: the secret must start with whsec_",
            );
        }
        const bytes = decode(secret.slice(SECRET_PREFIX.length));
        if (bytes === undefined) {
            throw ObjectError.of(
                "SECRET_BLOCKED",
                "cannot verify standard webhooks: the secret must be base64",
            );
        }

        return HmacKey.of(bytes, crypto);
    }

    /** Sign a message with every secret, answering its three headers. */
    async sign(message: StandardMessage): Promise<Headers> {
        // sign the identifier, timestamp and body with each key, space-separated
        const timestamp = String(message.timestamp);
        const signed = content(message.id, timestamp, new TextEncoder().encode(message.body));
        const digests = await Promise.all(this.#keys.map(async (key) => key.sign(signed)));
        const signatures = digests.map((digest) => `${SIGNATURE_VERSION},${digest.toBase64()}`);

        return new Headers({
            "webhook-id": message.id,
            "webhook-timestamp": timestamp,
            "webhook-signature": signatures.join(" "),
        });
    }

    /** Verify a request's v1 signature by any key within the tolerance, answering its message. */
    async verify(request: Request, now: number): Promise<StandardMessage> {
        // require the headers and a timestamp within the tolerance
        const id = request.headers.get("webhook-id");
        const timestamp = request.headers.get("webhook-timestamp");
        const signatures = request.headers.get("webhook-signature");
        if (id === null || timestamp === null || signatures === null) {
            throw ObjectError.of(
                "SIGNATURE_UNAUTHENTICATED",
                "authentication required: standard webhook headers are missing",
            );
        }
        const sentAt = Number(timestamp) * 1000;
        if (!/^\d+$/u.test(timestamp) || Math.abs(now - sentAt) > TOLERANCE_MILLISECONDS) {
            throw ObjectError.of(
                "TIMESTAMP_OUTSIDE_TOLERANCE_UNAUTHENTICATED",
                "authentication required: webhook timestamp is outside the tolerance",
            );
        }

        // match any v1 signature over the body's exact bytes among the space-separated ones, by any key
        const body = await RequestBody.bytes(request, "a webhook delivery");
        const signed = content(id, timestamp, body);
        let isSigned = false;
        for (const entry of signatures.split(" ")) {
            const [version, signature] = entry.split(",");
            const digest = version === SIGNATURE_VERSION ? decode(signature) : undefined;
            for (const key of this.#keys) {
                isSigned ||= digest !== undefined && (await key.verify(digest, signed));
            }
        }
        if (!isSigned) {
            throw ObjectError.of(
                "SIGNATURE_UNAUTHENTICATED",
                "authentication required: webhook signature does not match",
            );
        }

        return { id, timestamp: Number(timestamp), body: new TextDecoder().decode(body) };
    }
}

/** Encode the signed content: the identifier, timestamp and body bytes joined by dots. */
function content(id: string, timestamp: string, body: Uint8Array): Uint8Array<ArrayBuffer> {
    // join the prefix and the body
    const prefix = new TextEncoder().encode(`${id}.${timestamp}.`);
    const signed = new Uint8Array(prefix.length + body.length);
    signed.set(prefix);
    signed.set(body, prefix.length);

    return signed;
}

/** Decode base64, absent when malformed. */
function decode(value: string | undefined): Uint8Array<ArrayBuffer> | undefined {
    try {
        return value === undefined ? undefined : Uint8Array.fromBase64(value);
    } catch {
        return undefined;
    }
}
