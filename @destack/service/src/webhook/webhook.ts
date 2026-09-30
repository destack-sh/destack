import type { Call } from "@destack/sync";
import type { ResourceContext } from "@destack/resource/context";
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

/** A declared webhook: requests a sender signs, each delivery running one object method call. */
export interface Webhook extends Declaration {
    /** The trigger kind. */
    readonly kind: "webhook";
    /** The signature scheme. */
    readonly verification: WebhookDescription["verification"];
    /** The path template below the webhook. */
    readonly route: string;
    /** Read the signing secret of a delivery's route parameters from the installation's resources. */
    secret(parameters: WebhookParameters, resources: ResourceContext): Promise<string>;
    /** Build the call a verified delivery runs. */
    call(delivery: WebhookDelivery): Call;
    /** Verify a request to a path below the webhook and read its delivery. */
    receive(
        request: Request,
        path: string,
        now: number,
        resources: ResourceContext,
    ): Promise<WebhookDelivery>;
}

/** Declare a webhook. */
export function defineWebhook(
    definition: Pick<Webhook, "name" | "verification" | "route" | "secret" | "call">,
    module?: ModuleMetadata,
): Webhook {
    // stamp the declaring package
    const owner = declaringModule(module, "defineWebhook").package;
    const { secret, call, ...fields } = definition;
    const description = WebhookDescription.parse(fields);

    // require distinct parameter names
    const names = segments(description.route).filter((segment) => segment.startsWith("{"));
    if (new Set(names).size !== names.length) {
        throw new TypeError(`webhook route repeats a parameter: ${description.route}`);
    }

    return Object.freeze({
        ...description,
        kind: "webhook",
        package: owner,
        secret,
        call,
        receive,
    });
}

/** Verify a request to a path below a webhook with its route's secret, and read its delivery. */
async function receive(
    this: Webhook,
    request: Request,
    path: string,
    now: number,
    resources: ResourceContext,
): Promise<WebhookDelivery> {
    const parameters = match(this.route, path);
    const secret = await this.secret(parameters, resources);

    return WEBHOOK_SIGNATURES[this.verification].verify(request, secret, parameters, now);
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
