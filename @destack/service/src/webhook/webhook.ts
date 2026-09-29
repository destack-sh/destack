import type { TriggerHandler } from "../trigger/trigger.ts";
import { ServiceError } from "../error/index.ts";
import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import type { WebhookDelivery, WebhookParameters } from "./delivery.ts";
import { WEBHOOK_SIGNATURES } from "./signature.ts";

/** The webhook signature schemes. */
export const WEBHOOK_VERIFICATIONS = ["standard", "github"] as const;

/** A route of literal and `{name}` segments, or `/` alone. */
const ROUTE_PATTERN = /^\/$|^(?:\/(?:[\w.~-]+|\{[a-z][A-Za-z0-9]*\}))+$/;

/** A webhook, as the manifest describes it. */
export const WebhookDescription = defineSchema(
    schema.object({
        /** The package-local webhook name. */
        name: DeclarationName,
        /** The signature scheme. */
        verification: schema.enum(WEBHOOK_VERIFICATIONS),
        /** The path template below the webhook, such as `/{repository}`. */
        route: schema.string().regex(ROUTE_PATTERN),
    }),
);
/** A webhook, as the manifest describes it. */
export type WebhookDescription = schema.Infer<typeof WebhookDescription>;

/** A declared webhook. */
export interface Webhook extends Declaration {
    /** The trigger kind. */
    readonly kind: "webhook";
    /** The signature scheme. */
    readonly verification: WebhookDescription["verification"];
    /** The path template below the webhook. */
    readonly route: string;

    /** Pair the webhook with its handler and secret. */
    handle(
        handle: (event: WebhookDelivery, signal: AbortSignal) => Promise<void>,
        options: Pick<WebhookHandler, "secret">,
    ): WebhookHandler;
}

/** A workload's handler of one webhook. */
export interface WebhookHandler extends TriggerHandler<Webhook> {
    /** Read the signing secret of a delivery's route parameters. */
    secret(parameters: WebhookParameters): Promise<string>;
    /** Verify a request to a path below the webhook and read its delivery. */
    receive(request: Request, path: string, now: number): Promise<WebhookDelivery>;
}

/** Declare a webhook. */
export function defineWebhook(
    definition: Pick<Webhook, "name" | "verification" | "route">,
    module?: ModuleMetadata,
): Webhook {
    // stamp the declaring package
    const owner = declaringModule(module, "defineWebhook").package;
    const description = WebhookDescription.parse(definition);

    // require distinct parameter names
    const names = segments(description.route).filter((segment) => segment.startsWith("{"));
    if (new Set(names).size !== names.length) {
        throw new TypeError(`webhook route repeats a parameter: ${description.route}`);
    }

    return Object.freeze({
        ...description,
        kind: "webhook",
        package: owner,
        handle: handleWebhook,
    });
}

/** Pair a webhook with its handler and secret. */
function handleWebhook(
    this: Webhook,
    handle: WebhookHandler["handle"],
    options: Pick<WebhookHandler, "secret">,
): WebhookHandler {
    return {
        trigger: this,
        handle,
        secret: options.secret,
        receive: async (request, path, now) => {
            // verify the delivery with its route's secret
            const parameters = match(this.route, path);
            const secret = await options.secret(parameters);

            return WEBHOOK_SIGNATURES[this.verification].verify(request, secret, parameters, now);
        },
    };
}

/** Read a path's parameters from a route. */
function match(route: string, path: string): WebhookParameters {
    // require a path of as many segments
    const expected = segments(route);
    const actual = segments(path);
    if (!path.startsWith("/") || actual.length !== expected.length) {
        throw new ServiceError("NOT_FOUND", { message: `webhook route does not match: ${path}` });
    }

    // bind parameters and compare literals
    const parameters: Record<string, string> = {};
    for (const [index, segment] of expected.entries()) {
        const value = segment.startsWith("{") ? decode(actual[index]!) : undefined;
        // bind a parameter
        if (value) {
            parameters[segment.slice(1, -1)] = value;
        }
        // require the literal
        else if (segment !== actual[index]) {
            throw new ServiceError("NOT_FOUND", {
                message: `webhook route does not match: ${path}`,
            });
        }
    }

    return parameters;
}

/** Split a route or path into its segments. */
function segments(route: string): string[] {
    return route === "/" ? [] : route.slice(1).split("/");
}

/** Decode a path segment. */
function decode(segment: string): string | undefined {
    try {
        return decodeURIComponent(segment);
    } catch {
        return undefined;
    }
}
