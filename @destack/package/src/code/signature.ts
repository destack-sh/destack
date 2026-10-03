import { defineSchema, schema } from "@destack/schema";
import { SymbolReference } from "./reference.ts";
import { TypeDescription, TypeParameterDescription } from "./type.ts";

/** A function or constructor parameter. */
export const ParameterDescription = defineSchema(
    schema.object({
        /** The parameter name or binding pattern. */
        name: schema.string(),
        /** Named declarations the parameter type uses, printed in the signature text. */
        references: schema.array(SymbolReference),
        /** Whether callers can omit this argument. */
        isOptional: schema.boolean(),
        /** Whether this parameter collects remaining arguments. */
        isRest: schema.boolean(),
    }),
);

/** A callable signature, including individual overloads. */
export const SignatureDescription = defineSchema(
    schema.object({
        /** Whether the signature is called or constructed. */
        kind: schema.enum(["call", "construct"]),
        /** The signature with its predicates, truncated as the compiler prints it for display. */
        text: schema.string(),
        /** Generic parameters in declaration order. */
        typeParameters: schema.array(TypeParameterDescription),
        /** Explicit receiver type. */
        receiver: TypeDescription.exactOptional(),
        /** Parameters in declaration order. */
        parameters: schema.array(ParameterDescription),
        /** The return type. */
        returns: TypeDescription,
    }),
);
/** A callable signature. */
export type SignatureDescription = schema.Infer<typeof SignatureDescription>;
