import { defineSchema, identifier, schema } from "@destack/schema";

/** The lowercase kebab case of declared type names. */
const TYPE_NAME = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/;

/** An object: its type, the scope containing it, and its identifier there. */
export const ObjectReference = defineSchema(
    schema.object({
        /** The package that declares the object type. */
        packageId: identifier("package"),
        /** The declaration-local object type name. */
        type: schema.string().regex(TYPE_NAME),
        /** The scope containing the object, the container for a scope object. */
        scope: schema.string().min(1),
        /** The stable application record identity. */
        id: schema.string().min(1),
    }),
);
/** An object: its type, the scope containing it, and its identifier there. */
export type ObjectReference = schema.Infer<typeof ObjectReference>;

/** A package-qualified object type. */
export const ObjectTypeReference = defineSchema(
    ObjectReference.pick({ packageId: true, type: true }),
);
/** A package-qualified object type. */
export type ObjectTypeReference = schema.Infer<typeof ObjectTypeReference>;
