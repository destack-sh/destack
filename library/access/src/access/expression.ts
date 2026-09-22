import { schema } from "@destack/schema";
import { Attribute } from "./subject.ts";

/** A stable declaration-local name used by access rules. */
export const AccessName = schema.string().regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/);

/** A scalar literal or a typed attribute read. */
export type AccessOperand =
    | { readonly kind: "literal"; readonly value: Attribute }
    | {
          readonly kind: "object" | "context";
          readonly name: string;
          readonly type: "string" | "number" | "boolean";
      };

/** A serializable rule evaluated identically in memory and SQL. */
export type AccessExpression =
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
    | {
          readonly kind: "compare";
          readonly operator: "eq" | "ne" | "lt" | "lte" | "gt" | "gte";
          readonly left: AccessOperand;
          readonly right: AccessOperand;
      }
    | {
          readonly kind: "through";
          readonly relation: string;
          readonly permission: string;
          readonly transitive: boolean;
      };

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

/** Read a typed object or request attribute. */
export function attribute(
    kind: "object" | "context",
    name: string,
    type: "string" | "number" | "boolean",
): AccessOperand {
    return { kind, name: AccessName.parse(name), type };
}

/** Embed a scalar value in a permission expression. */
export function literal(value: Attribute): AccessOperand {
    return { kind: "literal", value: Attribute.parse(value) };
}

/** Compare attributes or literals without executing application callbacks. */
export function compare(
    operator: Extract<AccessExpression, { kind: "compare" }>["operator"],
    left: AccessOperand,
    right: AccessOperand,
): AccessExpression {
    return { kind: "compare", operator, left, right };
}
