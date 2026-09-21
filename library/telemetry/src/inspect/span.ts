import { defineSchema, schema } from "@destack/schema";
import { describeSource, SourceDescription } from "./source.ts";
import type { SpanContext } from "@opentelemetry/api";
import type { ReadableSpan } from "@opentelemetry/sdk-trace";
import { AttributeDescription, describeAttributes } from "./value.ts";
import {
    describeResource,
    describeScope,
    ResourceDescription,
    ScopeDescription,
    TimeDescription,
} from "./resource.ts";

/** Trace identity and propagation flags. */
export const ContextDescription = defineSchema(
    schema.object({
        traceId: schema.string(),
        spanId: schema.string(),
        traceFlags: schema.number().int(),
        traceState: schema.string().optional(),
        isRemote: schema.boolean().optional(),
    }),
);
/** A completed span and its recorded relationships. */
export const SpanDescription = defineSchema(
    schema.object({
        /** The exact build and declaration, when source attribution is present. */
        source: SourceDescription.optional(),
        name: schema.string(),
        kind: schema.number().int().min(0).max(4),
        context: ContextDescription,
        parent: ContextDescription.optional(),
        startTime: TimeDescription,
        endTime: TimeDescription,
        duration: TimeDescription,
        status: schema.object({
            code: schema.number().int().min(0).max(2),
            message: schema.string().optional(),
        }),
        attributes: AttributeDescription,
        links: schema.array(
            schema.object({
                context: ContextDescription,
                attributes: AttributeDescription,
                droppedAttributesCount: schema.number().int().optional(),
            }),
        ),
        events: schema.array(
            schema.object({
                name: schema.string(),
                time: TimeDescription,
                attributes: AttributeDescription,
                droppedAttributesCount: schema.number().int().optional(),
            }),
        ),
        resource: ResourceDescription,
        scope: ScopeDescription,
        ended: schema.boolean(),
        droppedAttributesCount: schema.number().int(),
        droppedEventsCount: schema.number().int(),
        droppedLinksCount: schema.number().int(),
    }),
);
/** A span's portable description. */
export type SpanDescription = schema.Infer<typeof SpanDescription>;

/** Describe a trace identity without retaining its SDK methods. */
export function describeContext(context: SpanContext): schema.Infer<typeof ContextDescription> {
    return {
        traceId: context.traceId,
        spanId: context.spanId,
        traceFlags: context.traceFlags,
        traceState: context.traceState?.serialize(),
        isRemote: context.isRemote,
    };
}

/** Describe a completed span for transport or storage. */
export function describeSpan(span: ReadableSpan): SpanDescription {
    return SpanDescription.parse({
        source: describeSource(span.attributes),
        name: span.name,
        kind: span.kind,
        context: describeContext(span.spanContext()),
        parent: span.parentSpanContext && describeContext(span.parentSpanContext),
        startTime: span.startTime,
        endTime: span.endTime,
        duration: span.duration,
        status: span.status,
        attributes: describeAttributes(span.attributes),
        links: span.links.map((link) => ({
            context: describeContext(link.context),
            attributes: describeAttributes(link.attributes ?? {}),
            droppedAttributesCount: link.droppedAttributesCount,
        })),
        events: span.events.map((event) => ({
            name: event.name,
            time: event.time,
            attributes: describeAttributes(event.attributes ?? {}),
            droppedAttributesCount: event.droppedAttributesCount,
        })),
        resource: describeResource(span.resource),
        scope: describeScope(span.instrumentationScope),
        ended: span.ended,
        droppedAttributesCount: span.droppedAttributesCount,
        droppedEventsCount: span.droppedEventsCount,
        droppedLinksCount: span.droppedLinksCount,
    });
}
