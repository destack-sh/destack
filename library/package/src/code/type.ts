import { defineSchema, schema } from "@destack/schema";
import { SymbolReference } from "./reference.ts";

/** A compiler-rendered type and its named declaration references. */
export const TypeDescription = defineSchema(schema.object({
    /** The complete type expression in the package's language. */
    text: schema.string(),
    /** Named declarations used by the type expression. */
    references: schema.array(SymbolReference),
}));
/** A compiler-rendered type and its named declaration references. */
export type TypeDescription = schema.Infer<typeof TypeDescription>;

/** A generic type parameter. */
export const TypeParameterDescription = defineSchema(schema.object({
    /** The parameter name. */
    name: schema.string(),
    /** The declared constraint. */
    constraint: TypeDescription.optional(),
    /** The declared default. */
    default: TypeDescription.optional(),
}));
