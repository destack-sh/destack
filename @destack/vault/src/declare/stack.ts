import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import type {} from "@destack/stack/declare";

/** A secret a stack declares, without its value. */
export const SecretDefinition = defineSchema(
    schema.object({
        /** The stack's vault resource containing the secret. */
        vault: DeclarationName,
        /** The name within the vault. */
        name: DeclarationName,
    }),
);
/** A secret a stack declares, without its value. */
export type SecretDefinition = schema.Infer<typeof SecretDefinition>;

/** Add the secrets collection to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The secrets the stack declares. */
        readonly secrets?: Readonly<Record<string, schema.Input<typeof SecretDefinition>>>;
    }
}
