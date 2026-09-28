import type { Policy } from "../policy/policy.ts";
import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Condition } from "@destack/db/query";
import { AccessName, type AccessExpression } from "../policy/expression.ts";
import { SubjectType } from "../policy/subject.ts";
import { Elevation } from "../context/elevation.ts";

/** Serialized permission expressions with no executable callbacks. */
export const AccessExpressionDescription: schema.Schema<AccessExpression> = schema.lazy(() =>
    schema.union([
        schema.object({ kind: schema.literal("none") }),
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
        schema.object({ kind: schema.literal("condition"), condition: Condition.schema }),
        schema.object({
            kind: schema.literal("through"),
            relation: AccessName,
            permission: AccessName,
            transitive: schema.boolean(),
        }),
        schema.object({ kind: schema.literal("grants"), reference: AccessName }),
    ]),
);

/** A type's relations and permissions as data. */
export const PolicyDescription = defineSchema(
    schema.object({
        /** The stable declaring package identity. */
        packageId: PackageId,
        /** The declaration-local object type name. */
        name: AccessName,
        /** The scalar type of each object attribute that permission expressions read. */
        attributes: schema.record(AccessName, schema.enum(["string", "number", "boolean"])),
        /** The relations to subjects and to other objects. */
        relations: schema.record(
            AccessName,
            schema.object({
                /** The subject types the relation accepts itself, beside those other types contribute to an open relation. */
                subjects: schema.array(SubjectType),
                /** The permission whose holders grant and revoke the relation. */
                grantedBy: AccessName.optional(),
                /** Whether other types contribute themselves as subject types. */
                open: schema.literal(true).optional(),
            }),
        ),
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
            .optional(),
        /** The named permission expressions. */
        permissions: schema.record(AccessName, AccessExpressionDescription),
        /** The permission required to bind roles on an object. */
        grantedBy: AccessName.optional(),
        /** The permissions only their expressions grant. */
        reserved: schema.array(AccessName).optional(),
        /** The permissions that apply only after the authentication each names. */
        elevated: schema.record(AccessName, Elevation.schema).optional(),
        /** Permissions that stay available while the scope is suspended. */
        administration: schema.array(AccessName).optional(),
        /** Whether the objects are scopes. */
        scope: schema.literal(true).optional(),
    }),
);
/** A type's relations and permissions as data. */
export type PolicyDescription = schema.Infer<typeof PolicyDescription>;

/** Describe a policy for inspection. */
export function describePolicy(policy: Policy): PolicyDescription {
    return PolicyDescription.parse(policy.definition);
}
