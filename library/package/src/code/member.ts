import { defineSchema, schema } from "@destack/schema";
import { DeclarationDescription } from "./declaration.ts";
import { TypeDescription } from "./type.ts";
import { SignatureDescription } from "./signature.ts";
import { Documentation } from "./documentation.ts";

/** A declared class, interface, enum, or namespace member. */
export const MemberDescription = defineSchema(
    schema.object({
        /** The member name. */
        name: schema.string(),
        /** Individual source declarations, including accessors and overloads. */
        declarations: schema.array(DeclarationDescription).min(1),
        /** Whether the member can be absent. */
        isOptional: schema.boolean(),
        /** The member type. */
        type: TypeDescription,
        /** Call and construct signatures. */
        signatures: schema.array(SignatureDescription),
        /** Documentation for this member. */
        documentation: Documentation,
    }),
);
