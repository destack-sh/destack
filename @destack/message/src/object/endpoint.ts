import { principal } from "@destack/access";
import { AuditActionName } from "@destack/audit";
import { index, type Select } from "@destack/db";
import { Conditions, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { WEBHOOK_FORMATS, WebhookDestination } from "../webhook/message.ts";

/** The most event types one endpoint subscribes to, a screen of event names. */
const MAX_ENDPOINT_EVENTS = 100;

/** A scope's subscription to its audit history and the history of the scopes inside it, delivered as webhook messages to an HTTPS URL. */
export const endpoint = defineObject({
    name: "endpoint",
    plural: "endpoints",
    controlled: true,
    fields: {
        /** The URL the messages go to. */
        url: field.string(schema.url({ protocol: /^https$/u })),
        /** The audit actions the endpoint receives, such as member.create, or all of them. */
        events: field.json(
            schema.union([
                schema.literal("all"),
                schema.array(AuditActionName).min(1).max(MAX_ENDPOINT_EVENTS),
            ]),
        ),
        /** How each message encodes the calls: one event per call, or a page of calls as JSON or NDJSON. */
        format: field.enum(WEBHOOK_FORMATS).default("event"),
        /** How the endpoint authenticates each message with the secret. */
        authentication: field.json(WebhookDestination.shape.authentication),
        /** The vault secret authenticating each message. */
        secret: field.json(ObjectReference),
        /** Whether the calls go to the endpoint. */
        status: field.enum(["enabled", "disabled"]).default("enabled"),
        /** The user who created the endpoint, as whom its secret is read. */
        user: field.reference(principal.user).caller(),
    },
    permissions: ["read", "manage"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage", {
            fields: ["url", "events", "format", "authentication", "secret", "status"],
        }),
        update: method.update("manage", {
            fields: ["url", "events", "format", "authentication", "secret", "status"],
        }),
        delete: method.delete("manage"),
    }),
    constraints: (entry) => [index("endpoint_scope").on(entry.scope, entry.status)],
});
/** A persisted endpoint. */
export type Endpoint = Select<typeof endpoint.table>;

/** The conditions the endpoint controller reports: Delivered, by an endpoint's last settled message. */
export const endpointConditions = new Conditions(["Delivered"]);
