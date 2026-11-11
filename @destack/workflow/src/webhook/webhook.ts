import type { MountFetch, ObjectStore } from "@destack/object/server";
import type { ServerContext } from "@destack/object/workload";
import { DeclarationName, defineKind, ModuleMetadata } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import type { MountDeclaration } from "@destack/object";
import type { ResourceHandle, Secret } from "@destack/resource";
import { Instant, nth, schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";
import type { WebhookSignature } from "./signature.ts";
import { WebhookServer } from "./server.ts";

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

/** A webhook in the manifest: its path template, its signature scheme and its secret. */
export const WebhookDescription = schema.object({
    /** The package-local webhook name. */
    name: DeclarationName,
    /** The path template below the webhook's mount path, such as `/{repository}`. */
    template: schema.string().regex(PATH_PATTERN),
    /** The name of the signature scheme, such as standard. */
    verification: schema.string().min(1),
    /** The secret the deliveries are signed with. */
    secret: DeclarationReference,
});
/** A webhook in the manifest. */
export type WebhookDescription = schema.Infer<typeof WebhookDescription>;

/** The webhook declaration kind, which workflow defines. */
export const WEBHOOK_KIND = defineKind({ name: "webhook", description: WebhookDescription });

/** Take a webhook's verified delivery in the installation's store. */
export type WebhookReceive = (delivery: WebhookDelivery, store: ObjectStore) => Promise<void>;

/** A webhook other services deliver to: a mount verifying each delivery before taking it. */
export interface Webhook extends MountDeclaration {
    /** The declaration kind. */
    readonly kind: "webhook";
    /** The mount's path below the package's path: `/webhooks/<name>`. */
    readonly path: `/webhooks/${string}`;
    /** The path template below the mount's path. */
    readonly template: WebhookPath;
    /** The scheme verifying each delivery's signature. */
    readonly signature: WebhookSignature;
    /** The secret the deliveries are signed with, which the installation binds. */
    readonly secret: ResourceHandle<Secret>;
    /** Answer the deliveries of an installation, verifying each before handing it over. */
    readonly serve: (
        receive: WebhookReceive,
        server: Pick<ServerContext, "resources">,
    ) => MountFetch;
}

/** What a webhook declares. */
export interface WebhookDefinition {
    /** The webhook's name within its package. */
    readonly name: string;
    /** The path template below the webhook's mount path, `/` by default. */
    readonly template?: string;
    /** The scheme verifying each delivery's signature. */
    readonly signature: WebhookSignature;
    /** The secret the deliveries are signed with, which the installation binds. */
    readonly secret: ResourceHandle<Secret>;
}

/** A webhook path template below a webhook, of literal and `{name}` segments, which paths match. */
export class WebhookPath {
    /** The path template, such as `/{repository}`. */
    readonly template: string;
    /** The template's parameter names, in order. */
    readonly parameters: readonly string[];

    /** Check a path template, refusing a malformed one or a repeated parameter. */
    constructor(template: string) {
        // read the parameter names, refusing a repeated one
        const checked = WebhookDescription.shape.template.parse(template);
        const names = segments(checked).filter((segment) => segment.startsWith("{"));
        if (new Set(names).size !== names.length) {
            throw new TypeError(`webhook path repeats a parameter: '${checked}'`);
        }
        this.template = checked;
        this.parameters = names;
    }

    /** Read a path's parameters, refusing a path the template does not match. */
    match(path: string): WebhookParameters {
        // require a path of as many segments
        const expected = segments(this.template);
        const actual = segments(path);
        if (!path.startsWith("/") || actual.length !== expected.length) {
            throw WorkflowError.of("PATH_NOT_FOUND", `webhook path '${path}' not found`);
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
                throw WorkflowError.of("PATH_NOT_FOUND", `webhook path '${path}' not found`);
            }
            // bind a parameter
            else if (segment.startsWith("{")) {
                parameters[segment.slice(1, -1)] = value;
            }
        }

        return parameters;
    }
}

/** Declare a webhook of the calling package, verified with the secret its installations bind. */
export function defineWebhook(definition: WebhookDefinition, module?: ModuleMetadata): Webhook {
    // answer each installation's deliveries through the webhook's server
    const owner = ModuleMetadata.require(module, "defineWebhook").package;
    const checked = DeclarationName.parse(definition.name);
    const { signature, secret } = definition;
    const webhook: Webhook = Object.freeze({
        kind: "webhook" as const,
        package: owner,
        name: checked,
        path: `/webhooks/${checked}`,
        template: new WebhookPath(definition.template ?? "/"),
        signature,
        secret,
        serve: (receive: WebhookReceive, server: Pick<ServerContext, "resources">) =>
            new WebhookServer(webhook, receive, server).fetch,
    });

    return webhook;
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
