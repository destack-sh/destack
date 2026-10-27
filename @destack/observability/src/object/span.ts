import { through, union } from "@destack/access";
import { machine, space } from "@destack/account/object";
import { defineObject, field, installation } from "@destack/object";
import { schema } from "@destack/schema";
import {
    AttributeKey,
    Attributes,
    Instrumentation,
    PersonalAttributes,
} from "../event/attribute.ts";
import { EVENT_FLUSH } from "../event/policy.ts";
import { telemetryRetention } from "../setting/index.ts";

/** A span's outcome, as OpenTelemetry's status codes name it. */
export const SPAN_STATUSES = ["unset", "ok", "error"] as const;

/** Something that happened during a span, such as an exception it recorded. */
export const SpanAnnotation = schema.object({
    /** The event's name, such as exception. */
    name: schema.string(),
    /** When it happened, in Unix microseconds. */
    time: schema.number().int(),
    /** Its attributes. */
    attributes: Attributes,
});
/** Something that happened during a span. */
export type SpanAnnotation = schema.Infer<typeof SpanAnnotation>;

/** What a span keeps beside its keys. */
export const SpanData = schema.object({
    /** The span, as 16 hexadecimal digits. */
    span: schema.string(),
    /** The span's kind, such as server or client. */
    kind: schema.number().int().exactOptional(),
    /** The annotations it recorded, such as exceptions. */
    annotations: schema.array(SpanAnnotation),
    /** The spans it links to. */
    links: schema.array(schema.object({ trace: schema.string(), span: schema.string() })),
    /** The workload instance that emitted it. */
    instance: schema.string().exactOptional(),
    /** The library that recorded it. */
    instrumentation: Instrumentation,
    /** The attributes the emitter marked personal. */
    personal: PersonalAttributes.exactOptional(),
});

/** A span an installation or a machine emitted, one wide event each. */
export const span = defineObject(
    {
        name: "span",
        plural: "spans",
        scope: [space, machine],
        storage: {
            kind: "events",
            flush: EVENT_FLUSH,
            retention: { setting: telemetryRetention.definition },
            delivery: "at-most-once",
            subject: "person",
        },
        fields: {
            /** The span, its personal attributes masked unless the reader may unmask them. */
            data: field.json(SpanData).guard({ read: "unmask" }),
            /** The installation that emitted it, absent for the machine's own spans. */
            installation: field.reference(installation).optional(),
            /** The digest of the emitting build's manifest. */
            build: field.string().optional(),
            /** The span's name. */
            name: field.string(),
            /** The trace, as 32 hexadecimal digits. */
            trace: field.string(),
            /** The parent span, absent for a trace's root. */
            parent: field.string().optional(),
            /** The outcome: unset, ok or error. */
            status: field.enum(SPAN_STATUSES),
            /** The duration, in microseconds. */
            duration: field.integer(),
            /** The issue an exception it recorded is grouped into. */
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
            visibility: "package",
            fields: [
                "installationId",
                "build",
                "name",
                "trace",
                "parent",
                "status",
                "duration",
                "issue",
                "session",
                "person",
                "attributes",
                "data",
            ],
        }),
    }),
);
