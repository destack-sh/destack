import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** How a stream encodes a batch of calls: one JSON array, or one JSON object per line. */
export const STREAM_FORMATS = ["json", "ndjson"] as const;

/** An HTTPS endpoint a scope's audited calls stream to in batches as webhook messages, such as Datadog's log intake, after GitHub's audit log streaming. */
export const auditStream = defineObject({
    name: "audit-stream",
    plural: "auditStreams",
    controlled: true,
    fields: {
        /** The URL the batches go to. */
        url: field.string(schema.url({ protocol: /^https$/u })),
        /** How a batch encodes its calls. */
        format: field.enum(STREAM_FORMATS).default("ndjson"),
        /** The headers each batch carries, such as an intake's API key. */
        headers: field.json(schema.record(schema.string().min(1), schema.string())).sensitive(),
        /** Whether the calls stream to the endpoint. */
        status: field.enum(["active", "inactive"]).default("inactive"),
    },
    permissions: ["read", "manage"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage", { fields: ["url", "format", "headers", "status"] }),
        update: method.update("manage", { fields: ["url", "format", "headers", "status"] }),
        delete: method.delete("manage"),
    }),
});
/** A persisted audit stream. */
export type AuditStream = Select<typeof auditStream.table>;
