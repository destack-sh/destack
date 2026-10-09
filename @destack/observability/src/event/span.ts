import { defineEventKind } from "@destack/event/declare";
import { TELEMETRY_ACCESS } from "./access.ts";
import { defineSchema, schema } from "@destack/schema";
import { AttributeKey, Attributes, Instrumentation, PersonalAttributes } from "./attribute.ts";
import { DAY, TELEMETRY_FLUSH } from "./policy.ts";

/** A span's outcome, as OpenTelemetry's status codes name it. */
export const SPAN_STATUSES = ["unset", "ok", "error"] as const;

/** Something that happened during a span, such as an exception it recorded. */
export const SpanAnnotation = defineSchema(
    schema.object({
        /** The event's name, such as exception. */
        name: schema.string(),
        /** When it happened, in Unix microseconds. */
        time: schema.number().int(),
        /** Its attributes. */
        attributes: Attributes,
    }),
);
/** Something that happened during a span. */
export type SpanAnnotation = schema.Infer<typeof SpanAnnotation>;

/** A span an installation or a machine emitted, one wide event each, as OpenTelemetry's span model records it. */
export const span = defineEventKind({
    name: "span",
    description: "A span an installation or a machine emitted.",
    keys: schema.object({
        /** The installation that emitted it, null for the machine's own spans. */
        installation: schema.string().nullable(),
        /** The digest of the emitting build's manifest. */
        build: schema.string().nullable(),
        /** The span's name. */
        name: schema.string(),
        /** The trace, as 32 hexadecimal digits. */
        trace: schema.string(),
        /** The parent span, null for a trace's root. */
        parent: schema.string().nullable(),
        /** The outcome: unset, ok or error. */
        status: schema.string(),
        /** The duration, in microseconds. */
        duration: schema.number().int(),
        /** The issue an exception it recorded is grouped into. */
        issue: schema.string().nullable(),
        /** The person it acted for, whose key seals its personal attributes. */
        person: schema.string().nullable(),
        /** The attributes, the personal ones left out. */
        attributes: AttributeKey,
    }),
    data: schema.object({
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
    }),
    delivery: "at-most-once",
    policy: { flush: TELEMETRY_FLUSH, retention: 30 * DAY },
    access: TELEMETRY_ACCESS,
    subject: "person",
});
