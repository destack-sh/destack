import { defineSchema, schema } from "@destack/schema";
import { describeSource, SourceDescription } from "./source.ts";
import type { ReadableLogRecord } from "@opentelemetry/sdk-logs";
import {
    AttributeDescription,
    describeAttributes,
    describeValue,
    ValueDescription,
} from "./value.ts";
import {
    describeResource,
    describeScope,
    ResourceDescription,
    ScopeDescription,
    TimeDescription,
} from "./resource.ts";
import { ContextDescription, describeContext } from "./span.ts";

/** A structured log record with explicit trace correlation. */
export const LogDescription = defineSchema(schema.object({
    /** The exact build and declaration, when source attribution is present. */
    source: SourceDescription.optional(),
    time: TimeDescription,
    observedTime: TimeDescription,
    context: ContextDescription.optional(),
    severityText: schema.string().optional(),
    severityNumber: schema.number().int().optional(),
    body: ValueDescription.optional(),
    eventName: schema.string().optional(),
    attributes: AttributeDescription,
    resource: ResourceDescription,
    scope: ScopeDescription.extend({
        attributes: AttributeDescription,
        droppedAttributesCount: schema.number().int().optional(),
    }),
    droppedAttributesCount: schema.number().int(),
}));
/** A log record's portable description. */
export type LogDescription = schema.Infer<typeof LogDescription>;

/** Describe a log record for transport or storage. */
export function describeLog(record: ReadableLogRecord): LogDescription {
    return LogDescription.parse({
        source: describeSource(record.attributes),
        time: record.hrTime,
        observedTime: record.hrTimeObserved,
        context: record.spanContext && describeContext(record.spanContext),
        severityText: record.severityText,
        severityNumber: record.severityNumber,
        body: record.body === undefined ? undefined : describeValue(record.body),
        eventName: record.eventName,
        attributes: describeAttributes(record.attributes),
        resource: describeResource(record.resource),
        scope: {
            ...describeScope(record.instrumentationScope),
            attributes: describeAttributes(record.instrumentationScope.attributes ?? {}),
            droppedAttributesCount: record.instrumentationScope.droppedAttributesCount,
        },
        droppedAttributesCount: record.droppedAttributesCount,
    });
}
