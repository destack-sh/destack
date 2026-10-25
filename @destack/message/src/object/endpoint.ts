import { AuditActionName } from "@destack/object";
import { type Select } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { ConditionSet, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { WEBHOOK_FORMATS, WebhookAuthentication } from "../webhook/message.ts";

/** The most event types one endpoint subscribes to, a screen of event names. */
const MAX_ENDPOINT_EVENTS = 100;

/** A scope's webhook subscription to its audit history. */
export const endpoint = defineObject(
    {
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
            /** How each message encodes the calls: one event per call, or a JSON or NDJSON page. */
            format: field.enum(WEBHOOK_FORMATS).default("event"),
            /** How the endpoint authenticates each message with the secret. */
            authentication: field.json(WebhookAuthentication),
            /** The vault secret authenticating each message, which the installation consumes. */
            secret: field.json(ObjectReference),
            /** Whether the calls go to the endpoint. */
            status: field.enum(["enabled", "disabled"]).default("enabled"),
        },
        permissions: ["read", "manage"],
        constraints: {
            scope: { kind: "index", on: ["scope", "status"] },
        },
        // read each endpoint's calls from its creation on, kept a week past its longest retry
        readers: { endpoint: { by: "id", start: "latest", hold: { days: 7 } } },
    },
    (method) => ({
        create: method.create("manage", {
            fields: ["url", "events", "format", "authentication", "secret", "status"],
        }),
        update: method.update("manage", {
            fields: ["url", "events", "format", "authentication", "secret", "status"],
        }),
        delete: method.delete("manage"),
    }),
);
/** A persisted endpoint. */
export type Endpoint = Select<typeof endpoint.table>;

/** The endpoints webhook messages go to. */
export const Endpoint = {
    /** Read the endpoint a webhook message's source names, as the code serving the endpoints. */
    async read(store: Pick<ObjectStore, "query" | "as">, source: ObjectReference) {
        // require an endpoint as the source
        if (!endpoint.is(source)) {
            throw new TypeError(
                `a webhook message's source ${source.type} ${source.id} is not an endpoint`,
            );
        }

        // read it through the store, as the installation delivering it
        const target = endpoint.target({ scope: source.scope, id: endpoint.identifier(source.id) });

        return store.query(store.as(endpoint), endpoint.call("get", target));
    },
};

/** The conditions the endpoint controller reports: Delivered, by the last settled message. */
export const endpointConditions = new ConditionSet(["Delivered"]);
