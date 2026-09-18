import { defineSchema, schema } from "@destack/schema";
import type { Resource } from "@opentelemetry/resources";
import type { InstrumentationScope } from "@opentelemetry/core";
import { AttributeDescription, describeAttributes } from "./value.ts";

/** The emitting process or frontend identity. */
export const ResourceDescription = defineSchema(schema.object({
    attributes: AttributeDescription,
    schemaUrl: schema.string().optional(),
}));
/** A resource's portable description. */
export type ResourceDescription = schema.Infer<typeof ResourceDescription>;

/** The emitting library identity. */
export const ScopeDescription = defineSchema(schema.object({
    name: schema.string(),
    version: schema.string().optional(),
    schemaUrl: schema.string().optional(),
}));
/** An instrumentation scope's portable description. */
export type ScopeDescription = schema.Infer<typeof ScopeDescription>;

/** Describe resolved resource attributes. */
export function describeResource(resource: Resource): ResourceDescription {
    if (resource.asyncAttributesPending) throw new Error("Resource attributes are unresolved.");

    return { attributes: describeAttributes(resource.attributes), schemaUrl: resource.schemaUrl };
}

/** Describe the emitting library. */
export function describeScope(scope: InstrumentationScope): ScopeDescription {
    return { name: scope.name, version: scope.version, schemaUrl: scope.schemaUrl };
}

/** Seconds and nanoseconds, preserving the SDK's timestamp precision. */
export const TimeDescription = defineSchema(
    schema.tuple([schema.number().int(), schema.number().int().min(0).max(999999999)]),
);
