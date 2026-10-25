import type { InstalledServerContext } from "@destack/object/workload";
import { nth, Instant, present, schema, type JsonValue } from "@destack/schema";
import { StandardWebhook } from "@destack/object";
import type { Crypto } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";

/** A path template of literal and `{name}` segments, or only `/`. */
const PATH_PATTERN = /^\/$|^(?:\/(?:[\w.~-]+|\{[a-z][A-Za-z0-9]*\}))+$/u;

/** The values of a webhook path's parameters, by name. */
export const WebhookParameters = schema.record(schema.string(), schema.string().min(1));
/** The values of a webhook path's parameters, by name. */
export type WebhookParameters = schema.Infer<typeof WebhookParameters>;

/** One verified webhook delivery. */
export const WebhookDelivery = schema.object({
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
});
/** One verified webhook delivery. */
export type WebhookDelivery = schema.Infer<typeof WebhookDelivery>;

/** The signed webhook deliveries a trigger fires on, as the manifest describes them. */
export const WebhookOn = schema.object({
    /** The name of the signature scheme, such as standard. */
    verification: schema.string().min(1),
    /** The path template below the trigger, such as `/{repository}`. */
    path: schema.string().regex(PATH_PATTERN),
});
/** The signed webhook deliveries a trigger fires on, as the manifest describes them. */
export type WebhookOn = schema.Infer<typeof WebhookOn>;

/** A webhook path template below a trigger, of literal and `{name}` segments, which paths match. */
export class WebhookPath {
    /** The path template, such as `/{repository}`. */
    readonly template: string;
    /** The template's parameter names, in order. */
    readonly parameters: readonly string[];

    /** Check a path template, refusing a repeated parameter. */
    constructor(template: string) {
        // read the parameter names, refusing a repeated one
        const names = segments(template).filter((segment) => segment.startsWith("{"));
        if (new Set(names).size !== names.length) {
            throw new TypeError(`webhook path repeats a parameter: ${template}`);
        }
        this.template = template;
        this.parameters = names;
    }

    /** Read a path's parameters, refusing a path the template does not match. */
    match(path: string): WebhookParameters {
        // require a path of as many segments
        const expected = segments(this.template);
        const actual = segments(path);
        if (!path.startsWith("/") || actual.length !== expected.length) {
            throw WorkflowError.of("PATH_MISMATCH", `webhook path does not match: ${path}`);
        }

        // bind parameters and compare literals
        const parameters: Record<string, string> = {};
        for (const [index, segment] of expected.entries()) {
            const value = segment.startsWith("{") ? decode(nth(actual, index)) : nth(actual, index);
            // refuse an empty or undecodable parameter or another literal
            if (
                value === undefined ||
                value === "" ||
                (!segment.startsWith("{") && segment !== value)
            ) {
                throw WorkflowError.of("PATH_MISMATCH", `webhook path does not match: ${path}`);
            }
            // bind a parameter
            else if (segment.startsWith("{")) {
                parameters[segment.slice(1, -1)] = value;
            }
        }

        return parameters;
    }
}

/** Read a webhook trigger's secret for a delivery's path parameters. */
export type WebhookTriggerSecret = (
    parameters: WebhookParameters,
    context: Pick<InstalledServerContext, "resources" | "keys">,
) => Promise<string>;

/** A webhook trigger's deliveries as one installation receives them. */
export class WebhookReceiver {
    /** The path template below the trigger. */
    readonly path: WebhookPath;
    /** The source of the trigger's secret, absent for a scheme without one. */
    readonly #secret: WebhookTriggerSecret | undefined;
    /** The verifier of the installation's deliveries. */
    readonly #verify: WebhookVerifier;
    /** The installation's resources and keys, the source of the secret. */
    readonly #context: Pick<InstalledServerContext, "resources" | "keys">;

    /** Receive a trigger's deliveries for an installation. */
    constructor(
        on: { readonly path: string; readonly secret?: WebhookTriggerSecret },
        verify: WebhookVerifier,
        context: Pick<InstalledServerContext, "resources" | "keys">,
    ) {
        // check the path template and take the secret, the verifier and the context
        this.path = new WebhookPath(on.path);
        this.#secret = on.secret;
        this.#verify = verify;
        this.#context = context;
    }

    /** Verify a request to a path below the trigger that matches its path template. */
    async receive(request: Request, pathname: string, now: number): Promise<WebhookDelivery> {
        // read the path's parameters and the secret they select, then verify the request
        const parameters = this.path.match(pathname);
        const secret =
            this.#secret === undefined ? undefined : await this.#secret(parameters, this.#context);

        return this.#verify(request, secret, parameters, now);
    }
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

/** Verify a request's signature for one installation, and read its delivery. */
export type WebhookVerifier = (
    request: Request,
    secret: string | undefined,
    parameters: WebhookParameters,
    now: number,
) => Promise<WebhookDelivery>;

/** A webhook signature scheme. */
export interface WebhookSignature {
    /** The scheme's name, which the manifest lists as a trigger's verification. */
    readonly name: string;
    /** Whether the scheme verifies with a secret its triggers share with the sender. */
    readonly isKeyed: boolean;
    /** Sign a message with the shared secret or the private key, returning the headers. */
    readonly sign: (message: WebhookMessage, secret: string, crypto: Crypto) => Promise<Headers>;
    /** Build the verifier of an installation's deliveries in the installation's environment. */
    readonly verifier: (
        context: Pick<InstalledServerContext, "resources" | "keys" | "environment">,
    ) => WebhookVerifier;
}

/** The payloads of the signature schemes. */
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
    /** Whether the scheme verifies with a shared secret, which it does. */
    readonly isKeyed: boolean;

    /** Create the Standard Webhooks scheme. */
    constructor() {
        this.name = "standard";
        this.isKeyed = true;
    }

    /** Sign the identifier, sending time in whole seconds and body. */
    async sign(message: WebhookMessage, secret: string, crypto: Crypto): Promise<Headers> {
        const timestamp = Math.floor(message.sentAt / 1000);
        const webhook = await StandardWebhook.of(secret, crypto);

        return webhook.sign({ id: message.id, timestamp, body: message.body });
    }

    /** Verify deliveries alike for every installation, on its environment's crypto. */
    verifier(
        context: Pick<InstalledServerContext, "resources" | "keys" | "environment">,
    ): WebhookVerifier {
        return (request, secret, parameters, now) =>
            this.verify(request, secret, parameters, now, context.environment.crypto);
    }

    /** Verify a delivery's signature, and read its event type. */
    async verify(
        request: Request,
        secret: string | undefined,
        parameters: WebhookParameters,
        now: number,
        crypto: Crypto,
    ): Promise<WebhookDelivery> {
        // verify the signature with the trigger's secret
        const webhook = await StandardWebhook.of(present(secret, "the trigger's secret"), crypto);
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
