import { through, union } from "@destack/access";
import { machine, space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { installation } from "@destack/stack/object";
import { AttributeKey, Instrumentation, PersonalAttributes } from "../event/attribute.ts";
import { TELEMETRY_FLUSH, TELEMETRY_RETENTION } from "../event/policy.ts";

/** What a log record keeps beside its keys. */
export const LogData = schema.object({
    /** The body, as text or JSON. */
    body: schema.string().exactOptional(),
    /** The span it was emitted in, as 16 hexadecimal digits. */
    span: schema.string().exactOptional(),
    /** The workload instance that emitted it. */
    instance: schema.string().exactOptional(),
    /** The library that recorded it. */
    instrumentation: Instrumentation,
    /** When the collector observed it, in Unix microseconds. */
    observed: schema.number().int().exactOptional(),
    /** The attributes the emitter marked personal. */
    personal: PersonalAttributes.exactOptional(),
});

/** A log record an installation or a machine emitted. */
export const log = defineObject(
    {
        name: "log",
        plural: "logs",
        scope: [space, machine],
        storage: {
            kind: "events",
            flush: TELEMETRY_FLUSH,
            retention: TELEMETRY_RETENTION,
            delivery: "at-most-once",
            subject: "person",
        },
        fields: {
            /** The record, its personal attributes masked unless the reader may unmask them. */
            data: field.json(LogData).guard({ read: "unmask" }),
            /** The installation that emitted it, absent for the machine's own records. */
            installation: field.reference(installation).optional(),
            /** The digest of the emitting build's manifest. */
            build: field.string().optional(),
            /** The event name, or a text body naming none. */
            name: field.string(),
            /** The severity, 1 to 24 as OpenTelemetry numbers it, 0 when unspecified. */
            severity: field.integer(),
            /** The trace it was emitted in, as 32 hexadecimal digits. */
            trace: field.string().optional(),
            /** The issue its exception is grouped into. */
            issue: field.string().optional(),
            /** The session it was recorded in. */
            session: field.string().optional(),
            /** The person it acted for, whose key seals its personal attributes. */
            person: field.string().optional(),
            /** The attributes, the personal ones left out. */
            attributes: field.json(AttributeKey),
        },
        permissions: {
            read: union(through("installation", "observe"), through("scope", "observe")),
            unmask: union(through("installation", "unmask"), through("scope", "unmask")),
            // let the machine serving the scope append what it forwards
            append: through("scope", "serve"),
        },
    },
    (method) => ({
        /** Append the records an export carries. */
        append: method.createMany("append", {
            fields: [
                "installationId",
                "build",
                "name",
                "severity",
                "trace",
                "issue",
                "session",
                "person",
                "attributes",
                "data",
            ],
        }),
    }),
);
