import type { ObjectType } from "../access/object.ts";
import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { AccessName } from "../access/expression.ts";
import { Attribute } from "../access/subject.ts";
import type { AccessExpression, AccessOperand } from "../access/expression.ts";

/** Serialized scalar references used by access inspection. */
export const AccessOperandDescription: schema.Schema<AccessOperand> = schema.union([
    schema.object({ kind: schema.literal("literal"), value: Attribute }),
    schema.object({
        kind: schema.enum(["object", "context"]),
        name: AccessName,
        type: schema.enum(["string", "number", "boolean"]),
    }),
]);

/** Serialized permission expressions with no executable callbacks. */
export const AccessExpressionDescription: schema.Schema<AccessExpression> = schema.lazy(() =>
    schema.union([
        schema.object({ kind: schema.enum(["relation", "permission"]), name: AccessName }),
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
            kind: schema.literal("compare"),
            operator: schema.enum(["eq", "ne", "lt", "lte", "gt", "gte"]),
            left: AccessOperandDescription,
            right: AccessOperandDescription,
        }),
        schema.object({
            kind: schema.literal("through"),
            relation: AccessName,
            permission: AccessName,
            transitive: schema.boolean(),
        }),
    ]),
);

/** A package's inspectable object types, relationships, and permissions. */
export const ObjectDescription = defineSchema(
    schema.object({
        packageId: PackageId,
        name: AccessName,
        attributes: schema.record(AccessName, schema.enum(["string", "number", "boolean"])),
        relations: schema.record(
            AccessName,
            schema.discriminatedUnion("kind", [
                schema.object({
                    kind: schema.literal("grant"),
                    subjects: schema
                        .array(
                            schema.enum([
                                "user",
                                "group",
                                "service-account",
                                "share-token",
                                "everyone",
                            ]),
                        )
                        .min(1),
                    permission: AccessName,
                }),
                schema.object({
                    kind: schema.literal("subject"),
                    subjects: schema
                        .array(schema.enum(["user", "group", "service-account", "share-token"]))
                        .min(1),
                }),
                schema.object({ kind: schema.literal("object"), type: AccessName }),
            ]),
        ),
        permissions: schema.record(AccessName, AccessExpressionDescription),
    }),
);
/** A package's inspectable object types, relationships, and permissions. */
export type ObjectDescription = schema.Infer<typeof ObjectDescription>;

/** Describe a declared object type for the package manifest. */
export function describeObject(object: ObjectType): ObjectDescription {
    return ObjectDescription.parse(object.definition);
}
