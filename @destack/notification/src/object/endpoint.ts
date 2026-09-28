import { unique, type Select } from "@destack/db";
import { user } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";
import { defineSchema, identifier, schema } from "@destack/schema";

/** The longest push resource URL. */
const URL_LENGTH = 2048;

/** A Web Push subscription's keys, base64url encoded. */
export const PushKeys = defineSchema(
    schema.object({
        /** The browser's uncompressed P-256 public key. */
        p256dh: schema.string().min(1),
        /** The authentication secret. */
        auth: schema.string().min(1),
    }),
);
/** A Web Push subscription's keys. */
export type PushKeys = schema.Infer<typeof PushKeys>;

/** A browser's Web Push endpoint. */
export const pushEndpoint = defineObject({
    name: "push-endpoint",
    plural: "pushEndpoints",
    scope: user,
    fields: {
        /** The push resource URL. */
        url: field.string(schema.string().url().max(URL_LENGTH)).sensitive(),
        /** The encryption keys. */
        keys: field.json(PushKeys).sensitive(),
        /** The browser's device, absent without one. */
        device: field.string(identifier("device")).optional(),
        /** The display name, such as "Pixel 9 (Chrome)". */
        name: field.string(schema.string().min(1).max(200)),
    },
    constraints: (endpoint) => [unique("push_endpoint_url").on(endpoint.scope, endpoint.url)],
    permissions: ["read", "create", "delete"],
    methods: {
        list: method.list("read"),
        create: method.create("create", {
            fields: ["url", "keys", "device", "name"],
            isPredicted: false,
        }),
        delete: method.delete("delete"),
    },
});

/** A push endpoint as its table holds it. */
export type PushEndpointRow = Select<typeof pushEndpoint.table>;
