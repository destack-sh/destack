import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package/package";
import { AccessName, type AccessExpression } from "./expression.ts";
import type { Attribute, Subject } from "./subject.ts";
import { AccessError } from "../error/index.ts";
import { AccessDeclaration } from "../inspect/declaration.ts";

/** A protected object within an explicit authority and installation scope. */
export const ObjectReference = defineSchema(
    schema.object({
        /** The package that declares the object type. */
        packageId: PackageId,
        /** The declaration-local object type name. */
        type: AccessName,
        /** The authority scope containing the object. */
        scope: schema.string().min(1),
        /** The stable application record identity. */
        id: schema.string().min(1),
    }),
);
/** A protected object within an explicit authority and installation scope. */
export type ObjectReference = schema.Infer<typeof ObjectReference>;

/** Membership, ownership, or an application object relationship. */
export type RelationDefinition =
    | {
          /** An explicitly persisted sharing relationship. */
          readonly kind: "grant";
          /** Identities accepted as recipients. */
          readonly subjects: readonly (Subject["kind"] | "everyone")[];
          /** The permission required to change grants. */
          readonly permission: string;
      }
    | {
          /** An identity relationship supplied by application records. */
          readonly kind: "subject";
          /** Identities accepted by this relationship. */
          readonly subjects: readonly Subject["kind"][];
      }
    | {
          /** A relationship to another protected object. */
          readonly kind: "object";
          /** The target object type in the declaring package. */
          readonly type: string;
      };

/** The declared relations and computed permissions for one protected object type. */
export interface ObjectDefinition {
    /** The stable declaring package identity. */
    readonly packageId: PackageId;
    /** The declaration-local object type name. */
    readonly name: string;
    /** Required object attributes used by permission expressions. */
    readonly attributes: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Application relationships and explicitly grantable relationships. */
    readonly relations: Readonly<Record<string, RelationDefinition>>;
    /** Named permission expressions. */
    readonly permissions: Readonly<Record<string, AccessExpression>>;
}

/** A declaration with typed references to its permissions and relations. */
export class ObjectType<Definition extends ObjectDefinition = ObjectDefinition> {
    /** The immutable serializable declaration. */
    readonly definition: Definition;

    /** Copy a declaration so later caller mutations cannot change access rules. */
    constructor(definition: Definition) {
        this.definition = freeze(AccessDeclaration.parse(definition) as Definition);
    }

    /** Identify an object without inferring its installation or authority. */
    ref(scope: string, id: string): ObjectReference {
        return ObjectReference.parse({
            packageId: this.definition.packageId,
            type: this.definition.name,
            scope,
            id,
        });
    }

    /** Reference one of this type's declared permissions. */
    permission(name: keyof Definition["permissions"] & string): PermissionReference {
        if (!Object.hasOwn(this.definition.permissions, name)) {
            throw new AccessError("INVALID_DECLARATION", `unknown permission: ${name}`);
        }

        return { packageId: this.definition.packageId, type: this.definition.name, name };
    }
}

/** A stable reference to a declared permission, independent of a package version. */
export const PermissionReference = defineSchema(
    schema.object({
        /** The package that declares the permission. */
        packageId: PackageId,
        /** The declaration-local object type name. */
        type: AccessName,
        /** The permission name within that object type. */
        name: AccessName,
    }),
);
/** A stable reference to a declared permission, independent of a package version. */
export type PermissionReference = schema.Infer<typeof PermissionReference>;

/** Authoritative application records supplied to the in-memory evaluator. */
export interface AccessObject {
    /** The complete authority-qualified object identity. */
    readonly reference: ObjectReference;
    /** Current values of declared scalar attributes. */
    readonly attributes: Readonly<Record<string, Attribute>>;
    /** Current direct subjects indexed by declared relation name. */
    readonly subjects: Readonly<Record<string, readonly Subject[]>>;
    /** Current related objects indexed by declared relation name. */
    readonly objects: Readonly<Record<string, readonly ObjectReference[]>>;
}

/** Declare a protected type while retaining literal permission names. */
export function defineObject<const Definition extends ObjectDefinition>(
    definition: Definition,
): ObjectType<Definition> {
    return new ObjectType(definition);
}

/** Construct a collision-free reference key. */
export function objectKey(reference: ObjectReference): string {
    return JSON.stringify([reference.packageId, reference.type, reference.scope, reference.id]);
}

/** Freeze every declaration node after copying it. */
function freeze<Value>(value: Value): Value {
    if (value !== null && typeof value === "object") {
        for (const child of Object.values(value)) {
            freeze(child);
        }
        Object.freeze(value);
    }

    return value;
}
