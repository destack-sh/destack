import type { Policy } from "../declare/policy.ts";
import { AccessName, ObjectTypeReference } from "@destack/sync";
import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Condition } from "@destack/db";
import { AttributeType, type AccessExpression } from "../declare/expression.ts";
import { SubjectType } from "../declare/subject.ts";
import { Elevation } from "../context/elevation.ts";

/** Serialized permission expressions with no executable callbacks. */
export const AccessExpressionDescription: schema.Schema<AccessExpression> = schema.lazy(() =>
    schema.union([
        schema.object({ kind: schema.literal("none") }),
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
export const PolicyDescription = defineSchema(
    schema.object({
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
                /** The subject types the relation accepts itself, beside those other types contribute to an open relation. */
                subjects: schema.array(SubjectType),
                /** The permission whose holders grant and revoke the relation. */
                grantedBy: AccessName.exactOptional(),
                /** Whether other types contribute themselves as subject types. */
                open: schema.literal(true).exactOptional(),
                /** Whether the relation is to the scope containing each object, which the scope chain decides. */
                isScope: schema.literal(true).exactOptional(),
                /** Whether the relation's relationships show only to holders of the permission granting it. */
                concealed: schema.literal(true).exactOptional(),
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
        /** The permissions only their expressions grant. */
        reserved: schema.array(AccessName).exactOptional(),
        /** The permissions that apply only after the authentication each names. */
        elevated: schema.record(AccessName, Elevation.schema).exactOptional(),
        /** Permissions that stay available while the scope is suspended. */
        administration: schema.array(AccessName).exactOptional(),
        /** Whether the objects are scopes. */
        scope: schema.literal(true).exactOptional(),
        /** Whether the type's objects live in the universe and an identifier alone refers to each. */
        isGlobal: schema.literal(true).exactOptional(),
    }),
);
/** A type's relations and permissions in JSON form. */
export type PolicyDescription = schema.Infer<typeof PolicyDescription>;

/** Describe a policy for inspection. */
export function describePolicy(policy: Policy): PolicyDescription {
    return PolicyDescription.parse(policy.definition);
}
