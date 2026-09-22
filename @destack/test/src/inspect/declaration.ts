import { defineSchema, schema } from "@destack/schema";

/** A statically identified test or suite declaration. */
export const TestDeclaration = defineSchema(
    schema.object({
        /** Whether the call declares a case or a suite. */
        kind: schema.enum(["test", "suite"]),
        /** The literal title or parameterized title pattern. */
        name: schema.string(),
        /** The source file relative to the package root. */
        file: schema.string(),
        /** The inclusive UTF-16 source offset. */
        start: schema.number().int().min(0),
        /** The exclusive UTF-16 source offset. */
        end: schema.number().int().min(0),
        /** Chained options such as skip, only, concurrent, or each. */
        modifiers: schema.array(schema.string()),
    }),
);
/** A statically identified test or suite declaration. */
export type TestDeclaration = schema.Infer<typeof TestDeclaration>;
