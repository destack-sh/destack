import { defineSchema, schema } from "@destack/schema";

/** The lowercase kebab case of declared type names. */
const TYPE_NAME = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/u;

/** The schema of an object reference. */
const objectReference = defineSchema(
    schema.object({
        /** The package that declares the object type. */
        packageId: schema.identifier("package"),
        /** The declaration-local object type name. */
        type: schema.string().regex(TYPE_NAME),
        /** The scope containing the object, the container for a scope object. */
        scope: schema.string().min(1),
        /** The stable application record identity. */
        id: schema.string().min(1),
    }),
);
/** An object: its type, the scope containing it, and its identifier there. */
export type ObjectReference = schema.Infer<typeof objectReference>;

/** An object: its type, the scope containing it, and its identifier there, and its key. */
export const ObjectReference = Object.assign(objectReference, {
    /** Key an object without collisions. */
    key(reference: ObjectReference): string {
        return JSON.stringify([reference.packageId, reference.type, reference.scope, reference.id]);
    },
});

/** A package-qualified object type, and its key. */
export const ObjectTypeReference = Object.assign(
    defineSchema(objectReference.pick({ packageId: true, type: true })),
    {
        /** Key an object type without collisions. */
        key(reference: Pick<ObjectReference, "packageId" | "type">): string {
            return JSON.stringify([reference.packageId, reference.type]);
        },
    },
);
/** A package-qualified object type. */
export type ObjectTypeReference = schema.Infer<typeof ObjectTypeReference>;
