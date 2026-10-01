import { defineSchema, Digest, schema } from "@destack/schema";
import { SourceLocation } from "../source/location.ts";
import { DependencySymbol } from "../code/reference.ts";
import { DeclarationName, Package } from "../definition/package.ts";

/** A declaration described by its domain inspector. */
export const DeclarationDescription = defineSchema(
    schema.object({
        /** The declaration name assigned by the domain inspector. */
        name: schema.string().min(1),
        /** The description kind. */
        kind: DeclarationName,
        /** The package declaring the kind. */
        package: Package,
        /** The declaration constructor in its exact package. */
        constructor: DependencySymbol.extend({ package: Package }),
        /** The original declaration in its exact source package. */
        symbol: DependencySymbol.extend({ package: Package }),
        /** The declaring module and source position. */
        source: SourceLocation,
        /** The description validated by its domain inspector. */
        description: schema.record(schema.string(), schema.json()),
        /** The declaration's terms, such as `object/note/relation/editor`, with the digest of each definition. */
        vocabulary: schema.record(schema.string(), Digest).optional(),
    }),
);
/** A declaration described by its domain inspector. */
export type DeclarationDescription = schema.Infer<typeof DeclarationDescription>;
