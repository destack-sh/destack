import { defineSchema, schema } from "@destack/schema";
import { SourceRange } from "../source/location.ts";
import { TypeDescription, TypeParameterDescription } from "./type.ts";

/** One source declaration contributing to a symbol. */
export const SourceDeclaration = defineSchema(
    schema.object({
        /** The language's declaration kind. */
        kind: schema.string().min(1),
        /** The declaration's source range. */
        source: SourceRange,
        /** Modifiers on this declaration, in source order. */
        modifiers: schema.array(schema.string()),
        /** Generic parameters on this declaration. */
        typeParameters: schema.array(TypeParameterDescription),
        /** Extends and implements clauses on this declaration. */
        heritage: schema.array(
            schema.object({
                /** The inheritance relation. */
                relation: schema.enum(["extends", "implements"]),
                /** The named base type. */
                type: TypeDescription,
            }),
        ),
    }),
);
/** One source declaration contributing to a symbol. */
export type SourceDeclaration = schema.Infer<typeof SourceDeclaration>;
