import { defineSchema, schema } from "@destack/schema";
import { DeclarationDescription } from "./declaration.ts";
import { SymbolReference } from "./reference.ts";
import { TypeDescription } from "./type.ts";
import { SignatureDescription } from "./signature.ts";
import { Documentation } from "./documentation.ts";
import { MemberDescription } from "./member.ts";

/** A named symbol and its combined type and value descriptions. */
export const SymbolDescription = defineSchema(
    schema.object({
        /** The symbol's qualified module-local name. */
        name: schema.string().min(1),
        /** Individual declarations, including overloads and declaration merging. */
        declarations: schema.array(DeclarationDescription).min(1),
        /** Combined documentation for this symbol. */
        documentation: Documentation,
        /** The declared type, including the instance type of a class. */
        declaredType: TypeDescription.optional(),
        /** Callable and constructable signatures of the declared type. */
        typeSignatures: schema.array(SignatureDescription),
        /** The value type, including class constructors and static members. */
        valueType: TypeDescription.optional(),
        /** Callable and constructable signatures of the value. */
        signatures: schema.array(SignatureDescription),
        /** Declared members, including static members. */
        members: schema.array(MemberDescription),
        /** Exports of a namespace or module, resolved to their original symbols. */
        exports: schema.array(
            schema.object({
                /** The exported name. */
                name: schema.string(),
                /** The original symbol. */
                symbol: SymbolReference,
            }),
        ),
        /** Index signatures of the declared type. */
        indexes: schema.array(
            schema.object({
                /** The accepted key type. */
                key: TypeDescription,
                /** The indexed value type. */
                value: TypeDescription,
                /** Whether indexed assignments are forbidden. */
                isReadonly: schema.boolean(),
            }),
        ),
    }),
);
/** A named symbol and its combined type and value descriptions. */
export type SymbolDescription = schema.Infer<typeof SymbolDescription>;
