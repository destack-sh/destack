import { AccessName, ObjectTypeReference } from "@destack/sync";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Condition } from "@destack/db";
import { AttributeType, type AccessExpression } from "./expression.ts";
import { SubjectType } from "./subject.ts";
import { Elevation } from "../context/elevation.ts";

/** Serialized permission expressions with no executable callbacks. */
export const AccessExpressionDescription: schema.Schema<AccessExpression> = schema.lazy(() =>
    schema.union([
        schema.object({ kind: schema.enum(["none", "self"]) }),
        schema.object({
            kind: schema.literal("contained"),
            principals: schema.array(ObjectTypeReference).exactOptional(),
        }),
        schema.object({
            kind: schema.enum(["relation", "permission"]),
            name: AccessName,
        }),
        schema.object({
            kind: schema.enum(["union", "intersection"]),
            expressions: schema.array(AccessExpressionDescription).min(1),
        }),
        schema.object({
            kind: schema.literal("exclusion"),
            include: AccessExpressionDescription,
            exclude: AccessExpressionDescription,
        }),
        schema.object({
            kind: schema.enum(["resource", "context"]),
            condition: Condition.schema,
        }),
        schema.object({
            kind: schema.literal("through"),
            relation: AccessName,
            permission: AccessName,
            transitive: schema.boolean(),
        }),
        schema.object({ kind: schema.literal("granters"), reference: AccessName }),
        schema.object({
            kind: schema.literal("readers"),
            reference: AccessName,
            relation: AccessName,
        }),
    ]),
);

/** A type's relations and permissions in JSON form. */
export const PolicyDescription = schema.object({
    /** The stable declaring package identity. */
    packageId: PackageId,
    /** The declaration-local object type name. */
    name: AccessName,
    /** The scalar type of each object attribute that permission expressions read. */
    attributes: schema.record(AccessName, AttributeType),
    /** The scalar type of each request attribute that permission expressions read. */
    context: schema.record(AccessName, AttributeType),
    /** The relations to subjects and to other objects. */
    relations: schema.record(
        AccessName,
        schema.object({
            /** The relation's declared subject types, beside an open relation's added ones. */
            subjects: schema.array(SubjectType),
            /** The permission whose holders grant and revoke the relation. */
            grantedBy: AccessName.exactOptional(),
            /** Whether other types contribute themselves as subject types. */
            open: schema.literal(true).exactOptional(),
            /** Whether the relation is to each object's scope, decided by the chain. */
            isScope: schema.literal(true).exactOptional(),
            /** Whether relationships show only to holders of the granting permission. */
            concealed: schema.literal(true).exactOptional(),
            /** Whether each object keeps at least one relationship of the relation. */
            isRequired: schema.literal(true).exactOptional(),
            /** Whether the rows with a deleted subject go with it. */
            onDelete: schema.literal("cascade").exactOptional(),
        }),
    ),
    /** The permission whose holders see the objects' relationships, `read` by default. */
    relationships: schema.object({ read: AccessName }).exactOptional(),
    /** The open relations of other types this type's objects may be subjects of. */
    contributes: schema
        .array(
            schema.object({
                /** The package declaring the type with the open relation. */
                packageId: PackageId,
                /** The type with the open relation. */
                type: AccessName,
                /** The open relation. */
                relation: AccessName,
            }),
        )
        .exactOptional(),
    /** The named permission expressions. */
    permissions: schema.record(AccessName, AccessExpressionDescription),
    /** The permission required to bind roles on an object. */
    grantedBy: AccessName.exactOptional(),
    /** Permissions owner roles leave out, granted only by expressions and listing roles. */
    reserved: schema.array(AccessName).exactOptional(),
    /** The permissions that apply only after the authentication each requires. */
    elevated: schema.record(AccessName, Elevation.schema).exactOptional(),
    /** Permissions that stay available while the scope is suspended. */
    administration: schema.array(AccessName).exactOptional(),
    /** Permissions that stay available while the scope moves to another machine. */
    moving: schema.array(AccessName).exactOptional(),
    /** Whether the objects are scopes. */
    scope: schema.literal(true).exactOptional(),
    /** Whether each object is a partition. */
    partition: schema.literal(true).exactOptional(),
    /** Whether objects live in the universe, each found by identifier only. */
    isGlobal: schema.literal(true).exactOptional(),
});
/** A type's relations and permissions in JSON form. */
export type PolicyDescription = schema.Infer<typeof PolicyDescription>;
