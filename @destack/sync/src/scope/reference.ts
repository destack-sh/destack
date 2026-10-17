import { schema } from "@destack/schema";

/** The lowercase kebab case of declared type names. */
const TYPE_NAME = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u;

/** The parts of an object's key in order: package, type, scope and identifier. */
const OBJECT_KEY = schema.tuple([
    schema.string(),
    schema.string(),
    schema.string(),
    schema.string(),
]);

/** The schema of an object reference. */
const objectReference = schema.object({
    /** The package that declares the object type. */
    packageId: schema.identifier("package"),
    /** The declaration-local object type name. */
    type: schema.string().regex(TYPE_NAME),
    /** The scope containing the object, the container for a scope object. */
    scope: schema.string().min(1),
    /** The stable application record identity. */
    id: schema.string().min(1),
});
/** An object: its type, the scope containing it, and its identifier there. */
export type ObjectReference = schema.Infer<typeof objectReference>;

/** An object: its type, the scope containing it, and its identifier there, and its key. */
export const ObjectReference = Object.assign(objectReference, {
    /** Key an object without collisions. */
    key(reference: ObjectReference): string {
        return JSON.stringify([reference.packageId, reference.type, reference.scope, reference.id]);
    },

    /** Read the object a key refers to, refusing a malformed key. */
    read(key: string): ObjectReference {
        const [packageId, type, scope, id] = OBJECT_KEY.parse(JSON.parse(key));

        return objectReference.parse({ packageId, type, scope, id });
    },

    /** Spread a reference, or its absence, into the columns keeping it under a field name. */
    columns: referenceColumns,

    /** Read the reference a row keeps under a field name, refusing an absent one. */
    at<Name extends string>(
        name: Name,
        row: Readonly<
            Partial<
                Record<`${Name}PackageId` | `${Name}Type` | `${Name}Scope` | `${Name}Id`, unknown>
            >
        >,
    ): ObjectReference {
        const columns: { readonly [Column in string]?: unknown } = row;

        return objectReference.parse({
            packageId: columns[`${name}PackageId`],
            type: columns[`${name}Type`],
            scope: columns[`${name}Scope`],
            id: columns[`${name}Id`],
        });
    },

    /** Read the parent a nested row refers to, in the row's scope. */
    parent(row: {
        readonly scope: string;
        readonly parentPackageId: ObjectReference["packageId"];
        readonly parentType: string;
        readonly parentId: string;
    }): ObjectReference {
        return {
            packageId: row.parentPackageId,
            type: row.parentType,
            scope: row.scope,
            id: row.parentId,
        };
    },
});

/** The columns keeping an object reference under a field name. */
export type ReferenceColumns<Name extends string> = Readonly<
    Record<`${Name}PackageId`, ObjectReference["packageId"]> &
        Record<`${Name}Type` | `${Name}Scope` | `${Name}Id`, string>
>;

/** The columns of an absent object reference under a field name. */
export type AbsentColumns<Name extends string> = Readonly<
    Record<`${Name}PackageId` | `${Name}Type` | `${Name}Scope` | `${Name}Id`, null>
>;

/** Spread a reference, or its absence, into the columns keeping it under a field name. */
function referenceColumns<Name extends string, Reference extends ObjectReference | undefined>(
    name: Name,
    reference: Reference,
): Reference extends ObjectReference ? ReferenceColumns<Name> : AbsentColumns<Name>;
/**
 * Spread a reference into the columns keeping it under a field name, typed by the signature above.
 *
 * NOTE #Architecture: the one stated type of reference columns, whose keys a template computes.
 */
function referenceColumns(
    name: string,
    reference: ObjectReference | undefined,
): { readonly [Column in string]: string | null } {
    return {
        [`${name}PackageId`]: reference?.packageId ?? null,
        [`${name}Type`]: reference?.type ?? null,
        [`${name}Scope`]: reference?.scope ?? null,
        [`${name}Id`]: reference?.id ?? null,
    };
}

/** A package-qualified object type, and its key. */
export const ObjectTypeReference = Object.assign(
    objectReference.pick({ packageId: true, type: true }),
    {
        /** Key an object type without collisions. */
        key(reference: Pick<ObjectReference, "packageId" | "type">): string {
            return JSON.stringify([reference.packageId, reference.type]);
        },
    },
);
/** A package-qualified object type. */
export type ObjectTypeReference = schema.Infer<typeof ObjectTypeReference>;
