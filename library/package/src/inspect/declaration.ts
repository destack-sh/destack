import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";
import { SourceLocation } from "../source/location.ts";
import { DependencySymbol } from "../code/reference.ts";

/** A declaration described by its domain inspector. */
export const DeclarationDescription = defineSchema(
    schema.object({
        /** The declaration name assigned by the domain inspector. */
        name: schema.string().min(1),
        /** The declaration domain. */
        kind: ResourceName,
        /** The original declaration in its exact source package. */
        symbol: DependencySymbol,
        /** The declaring module and source position. */
        source: SourceLocation,
        /** JSON Schema supplied by the domain library. */
        schema: schema.record(schema.string(), schema.json()),
        /** The description validated by its domain inspector. */
        description: schema.record(schema.string(), schema.json()),
    }),
);
/** A declaration described by its domain inspector. */
export type DeclarationDescription = schema.Infer<typeof DeclarationDescription>;
