import { check, dialectSQL, sql, type Column, type Condition } from "@destack/db";
import { schema } from "@destack/schema";
import { AccessName, type ObjectTypeReference } from "@destack/sync";
import type { Policy } from "./policy.ts";

/** A scalar request attribute that permission conditions compare. */
export const Attribute = schema.union([schema.string(), schema.number(), schema.boolean()]);
/** A scalar request attribute that permission conditions compare. */
export type Attribute = schema.Infer<typeof Attribute>;

/** The scalar type of an attribute that permission conditions compare. */
export const AttributeType = schema.enum(["string", "number", "boolean"]);
/** The scalar type of an attribute that permission conditions compare. */
export type AttributeType = schema.Infer<typeof AttributeType>;

/** Constrain a column with access names to their lowercase kebab case. */
export function nameCheck(name: string, column: Column) {
    return check(
        name,
        dialectSQL({
            sqlite: sql`${column} IS NULL OR (length(${column}) > 0 AND substr(${column}, 1, 1) GLOB '[a-z]' AND ${column} NOT GLOB '*[^a-z0-9-]*' AND ${column} NOT LIKE '%--%' AND ${column} NOT LIKE '%-')`,
            postgresql: sql`${column} IS NULL OR ${column} ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'`,
        }),
    );
}

/** A permission rule deciding by a condition over the object's attributes, or the request's. */
export type ConditionExpression = Extract<
    AccessExpression,
    { readonly kind: "resource" | "context" }
>;

/** A serializable permission rule. */
export type AccessExpression =
    | { readonly kind: "none" }
    | { readonly kind: "relation"; readonly name: string }
    | { readonly kind: "permission"; readonly name: string }
    | {
          readonly kind: "union" | "intersection";
          readonly expressions: readonly AccessExpression[];
      }
    | {
          readonly kind: "exclusion";
          readonly include: AccessExpression;
          readonly exclude: AccessExpression;
      }
    | { readonly kind: "resource" | "context"; readonly condition: Condition }
    | {
          readonly kind: "through";
          readonly relation: string;
          readonly permission: string;
          readonly transitive: boolean;
      }
    | { readonly kind: "granters"; readonly reference: string }
    | { readonly kind: "readers"; readonly reference: string; readonly relation: string }
    | { readonly kind: "contained"; readonly principals?: readonly ObjectTypeReference[] };

/** The relation of the subject set of the principals inside a scope. */
export const CONTAINED = "contained";

/** Match no relation, so only roles grant the permission. */
export function none(): AccessExpression {
    return { kind: "none" };
}

/** Construct a relation membership expression. */
export function relation(name: string): AccessExpression {
    return { kind: "relation", name: AccessName.parse(name) };
}

/** Reference another permission on the same object. */
export function permission(name: string): AccessExpression {
    return { kind: "permission", name: AccessName.parse(name) };
}

/** Permit subjects accepted by any expression. */
export function union(...expressions: AccessExpression[]): AccessExpression {
    return { kind: "union", expressions };
}

/** Require every expression to permit access. */
export function intersection(...expressions: AccessExpression[]): AccessExpression {
    return { kind: "intersection", expressions };
}

/** Remove subjects accepted by the excluded expression. */
export function exclusion(include: AccessExpression, exclude: AccessExpression): AccessExpression {
    return { kind: "exclusion", include, exclude };
}

/** Follow an object relation once or through its ancestor closure. */
export function through(
    relationName: string,
    permissionName: string,
    transitive = false,
): AccessExpression {
    return {
        kind: "through",
        relation: AccessName.parse(relationName),
        permission: AccessName.parse(permissionName),
        transitive,
    };
}

/** Permit whoever may grant on the object a row references, through the grant permission of that object's own type. */
export function grantersOf(reference: string): AccessExpression {
    return { kind: "granters", reference: AccessName.parse(reference) };
}

/**
 * Permit whoever sees the relationships of the object a row references: holders of its type's relationship read permission or its grant permission.
 *
 * A row of a concealed relation, named by the row's relation field, is permitted only to holders of the permission granting that relation.
 */
export function readersOf(reference: string, field: string): AccessExpression {
    return {
        kind: "readers",
        reference: AccessName.parse(reference),
        relation: AccessName.parse(field),
    };
}

/**
 * Permit the principals inside a scope: those living in it or in a scope it encloses, and the scope's own principal.
 *
 * A scope object decides for itself, any other object for the scope it lives in.
 * Named principal types narrow the principals to those types.
 */
export function contained(...principals: readonly Policy[]): AccessExpression {
    return principals.length === 0
        ? { kind: "contained" }
        : {
              kind: "contained",
              principals: principals.map((type) => ({
                  packageId: type.definition.packageId,
                  type: type.definition.name,
              })),
          };
}

/** Permit objects whose attributes meet a condition, reading request attributes through placeholders. */
export function resource(where: Condition): AccessExpression {
    return { kind: "resource", condition: where };
}

/** Permit requests whose attributes meet a condition. */
export function context(where: Condition): AccessExpression {
    return { kind: "context", condition: where };
}
