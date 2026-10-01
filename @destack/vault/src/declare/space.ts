import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import type {} from "@destack/space/declare";

/** A secret a stack declares, without its value. */
export const SpaceSecret = defineSchema(
    schema.object({
        /** The stack's vault resource containing the secret. */
        vault: DeclarationName,
        /** The name within the vault. */
        name: DeclarationName,
    }),
);
/** A secret a stack declares, without its value. */
export type SpaceSecret = schema.Infer<typeof SpaceSecret>;

/** Add the secrets collection to stacks. */
declare module "@destack/space/declare" {
    interface SpaceDocument {
        /** The secrets the stack declares. */
        readonly secrets?: Readonly<Record<string, schema.Input<typeof SpaceSecret>>>;
    }
}
