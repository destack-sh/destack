import { check, dialectSQL, sql, type Column } from "@destack/db";
import type { Condition } from "@destack/db/query";
import { schema } from "@destack/schema";

/** A scalar request attribute that permission conditions compare. */
export const Attribute = schema.union([
    schema.string(),
    schema.number().finite(),
    schema.boolean(),
]);
/** A scalar request attribute that permission conditions compare. */
export type Attribute = schema.Infer<typeof Attribute>;

/** A stable declaration-local name used by access rules. */
export const AccessName = schema.string().regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/);

/** Constrain a column holding access names to their lowercase kebab case. */
export function nameCheck(name: string, column: Column) {
    return check(
        name,
        dialectSQL({
            sqlite: sql`${column} IS NULL OR (length(${column}) > 0 AND substr(${column}, 1, 1) GLOB '[a-z]' AND ${column} NOT GLOB '*[^a-z0-9-]*' AND ${column} NOT LIKE '%--%' AND ${column} NOT LIKE '%-')`,
            postgresql: sql`${column} IS NULL OR (${column} COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'`,
        }),
    );
}

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
    | { readonly kind: "condition"; readonly condition: Condition }
    | {
          readonly kind: "through";
          readonly relation: string;
          readonly permission: string;
          readonly transitive: boolean;
      }
    | { readonly kind: "grants"; readonly reference: string };

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
    relation: string,
    permission: string,
    transitive = false,
): AccessExpression {
    return {
        kind: "through",
        relation: AccessName.parse(relation),
        permission: AccessName.parse(permission),
        transitive,
    };
}

/** Permit whoever may grant on the object a row references, through the grant permission of that object's own type. */
export function grants(reference: string): AccessExpression {
    return { kind: "grants", reference: AccessName.parse(reference) };
}

/** Permit rows with attributes that meet a condition over request attributes. */
export function condition(condition: Condition): AccessExpression {
    return { kind: "condition", condition };
}
