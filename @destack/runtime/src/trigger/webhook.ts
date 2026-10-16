import type { ServerContext } from "../workload/workload.ts";
import { nth, defineSchema, Instant, schema, type JsonValue } from "@destack/schema";
import { CallError } from "../error/index.ts";
import { RequestBody } from "../request/body.ts";

/** A path template of literal and `{name}` segments, or only `/`. */
const PATH_PATTERN = /^\/$|^(?:\/(?:[\w.~-]+|\{[a-z][A-Za-z0-9]*\}))+$/u;

/** The Standard Webhooks timestamp tolerance, five minutes as in the reference libraries. */
const STANDARD_TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The prefix of a serialized Standard Webhooks symmetric secret. */
const STANDARD_SECRET_PREFIX = "whsec_";

/** The version tag of a Standard Webhooks HMAC-SHA256 signature. */
const STANDARD_SIGNATURE_VERSION = "v1";

/** The values of a webhook path's parameters, by name. */
export const WebhookParameters = defineSchema(
    schema.record(schema.string(), schema.string().min(1)),
);
/** The values of a webhook path's parameters, by name. */
export type WebhookParameters = schema.Infer<typeof WebhookParameters>;

/** One verified webhook delivery. */
export const WebhookDelivery = defineSchema(
    schema.object({
        /** The sender's delivery identifier. */
        id: schema.string().min(1),
        /** The event type, such as push. */
        event: schema.string().min(1),
        /** The decoded body. */
        payload: schema.json(),
        /** The path's parameters. */
        parameters: WebhookParameters,
        /** The receiving time, in UTC epoch milliseconds. */
        receivedAt: Instant,
    }),
);
/** One verified webhook delivery. */
export type WebhookDelivery = schema.Infer<typeof WebhookDelivery>;

/** The signed webhook deliveries a trigger fires on, as the manifest describes them. */
const shape = defineSchema(
    schema.object({
        /** The name of the signature scheme, such as standard. */
        verification: schema.string().min(1),
        /** The path template below the trigger, such as `/{repository}`. */
        path: schema.string().regex(PATH_PATTERN),
    }),
);
/** The signed webhook deliveries a trigger fires on, as the manifest describes them. */
export type WebhookOn = schema.Infer<typeof shape>;

/** The signed webhook deliveries a trigger fires on, and how it verifies them. */
export const WebhookOn = Object.assign(shape, {
    /** Read a path template's parameter names, refusing a repeated one. */
    parameters(path: string): readonly string[] {
        const names = segments(path).filter((segment) => segment.startsWith("{"));
        if (new Set(names).size !== names.length) {
            throw new TypeError(`webhook path repeats a parameter: ${path}`);
        }

        return names;
    },

    /** Verify a request to a path below a trigger that matches its path template. */
    async receive(
        on: Pick<WebhookOn, "path"> & {
            readonly signature: WebhookSignature;
            readonly secret?: (
                parameters: WebhookParameters,
                context: Pick<ServerContext, "resources" | "keys">,
            ) => Promise<string>;
        },
        request: Request,
        pathname: string,
        now: number,
        context: Pick<ServerContext, "resources" | "keys">,
    ): Promise<WebhookDelivery> {
        const parameters = match(on.path, pathname);
        const secret = on.secret === undefined ? undefined : await on.secret(parameters, context);

        return on.signature.verify(request, secret, parameters, now, context);
    },
});

/** Read a path's parameters from a path template. */
function match(template: string, path: string): WebhookParameters {
    // require a path of as many segments
    const expected = segments(template);
    const actual = segments(path);
    if (!path.startsWith("/") || actual.length !== expected.length) {
        throw new CallError("NOT_FOUND", { message: `webhook path does not match: ${path}` });
    }

    // bind parameters and compare literals
    const parameters: Record<string, string> = {};
    for (const [index, segment] of expected.entries()) {
        const value = segment.startsWith("{")
            ? decode(nth(actual, index))
            : nth(actual, index);
        // refuse an empty or undecodable parameter or another literal
        if (
            value === undefined ||
            value === "" ||
            (!segment.startsWith("{") && segment !== value)
        ) {
            throw new CallError("NOT_FOUND", {
                message: `webhook path does not match: ${path}`,
            });
        }
        // bind a parameter
        else if (segment.startsWith("{")) {
            parameters[segment.slice(1, -1)] = value;
        }
    }

    return parameters;
}

/** Split a path or path template into its segments. */
function segments(path: string): string[] {
    return path === "/" ? [] : path.slice(1).split("/");
}

/** Decode a path segment. */
function decode(segment: string): string | undefined {
    try {
        return decodeURIComponent(segment);
    } catch {
        return undefined;
    }
}

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
    /** The scheme's name, which the manifest lists as a trigger's verification. */
    readonly name: string;
    /** Sign a message with the shared secret or the private key, returning the headers. */
    readonly sign: (message: WebhookMessage, secret: string) => Promise<Headers>;
    /** Verify a request's signature, and read its delivery. */
    readonly verify: (
        request: Request,
        secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
        context: Pick<ServerContext, "resources" | "keys">,
    ) => Promise<WebhookDelivery>;
}

/** The keys and payloads of the signature schemes. */
export const WebhookSignature = {
    /** Import raw bytes as an HMAC SHA-256 key. */
    key(bytes: Uint8Array<ArrayBuffer>, usage: "sign" | "verify"): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            usage,
        ]);
    },

    /** Require the shared secret a keyed scheme verifies with. */
    secret(secret: string | undefined, scheme: string): string {
        if (secret === undefined) {
            throw new CallError("UNAUTHORIZED", {
                message: `${scheme} webhooks need a secret on the trigger`,
            });
        }

        return secret;
    },

    /** Parse a verified body as JSON. */
    payload(body: string): JsonValue {
        try {
            return schema.json().parse(JSON.parse(body));
        } catch {
            throw new CallError("BAD_REQUEST", { message: "webhook body is not JSON" });
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
        secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
    ): Promise<WebhookDelivery> {
        // require the headers and a current timestamp
        const id = request.headers.get("webhook-id");
        const timestamp = request.headers.get("webhook-timestamp");
        const signatures = request.headers.get("webhook-signature");
        if (id === null || timestamp === null || signatures === null) {
            throw new CallError("UNAUTHORIZED", { message: "missing standard webhook headers" });
        }
        const sentAt = Number(timestamp) * 1000;
        if (!/^\d+$/u.test(timestamp) || Math.abs(now - sentAt) > STANDARD_TOLERANCE_MILLISECONDS) {
            throw new CallError("UNAUTHORIZED", {
                message: "webhook timestamp is outside the tolerance",
            });
        }

        // match any v1 signature
        const body = await RequestBody.text(request, "a webhook delivery");
        const key = await StandardSignature.#key(
            WebhookSignature.secret(secret, this.name),
            "verify",
        );
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
            throw new CallError("UNAUTHORIZED", { message: "webhook signature does not match" });
        }

        // read the event type
        const payload = WebhookSignature.payload(body);
        const event = schema
            .object({ type: schema.string().min(1) })
            .loose()
            .safeParse(payload);
        if (!event.success) {
            throw new CallError("BAD_REQUEST", {
                message: "standard webhook payload has no type",
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
            throw new TypeError("standard webhook secret must start with whsec_");
        }

        return WebhookSignature.key(
            Uint8Array.fromBase64(secret.slice(STANDARD_SECRET_PREFIX.length)),
            usage,
        );
    }
}

/** Decode a base64 signature. */
function decodeBase64(value: string | undefined): Uint8Array<ArrayBuffer> | undefined {
    try {
        return value === undefined ? undefined : Uint8Array.fromBase64(value);
    } catch {
        return undefined;
    }
}
