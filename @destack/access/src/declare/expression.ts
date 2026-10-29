import { check, type Condition } from "@destack/db";
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

/** Constrain a column property to lowercase kebab case access names. */
export function nameCheck(name: string, property: string) {
    return check(name, {
        [property]: {
            OR: [
                { isNull: true },
                {
                    length: { gt: 0 },
                    characters: "a-z0-9-",
                    NOT: {
                        OR: [
                            ..."-0123456789".split("").map((first) => ({ startsWith: first })),
                            { endsWith: "-" },
                            { like: "%--%" },
                        ],
                    },
                },
            ],
        },
    });
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
    | { readonly kind: "contained"; readonly principals?: readonly ObjectTypeReference[] }
    | {
          readonly kind: "subjectSet";
          readonly object: ObjectTypeReference;
          readonly id: string;
          readonly relation: string;
          readonly scope: SubjectSetScope;
      }
    | { readonly kind: "self" };

/** Where a subject set counts: in the row's scope alone, or in any scope of the row's chain. */
export type SubjectSetScope = "row" | "chain";

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

/** Permit who may grant on a row's referenced object, by that object type's grant permission. */
export function grantersOf(reference: string): AccessExpression {
    return { kind: "granters", reference: AccessName.parse(reference) };
}

/** Permit who sees the relationships of a row's referenced object. */
export function readersOf(reference: string, field: string): AccessExpression {
    return {
        kind: "readers",
        reference: AccessName.parse(reference),
        relation: AccessName.parse(field),
    };
}

/** Permit the principals inside a scope, its enclosed scopes, and the scope's principal. */
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

/** Permit the members of an object's subject set in the row's scope or chain. */
export function subjectSet(
    type: Policy,
    id: string,
    set: string,
    scope: SubjectSetScope,
): AccessExpression {
    return {
        kind: "subjectSet",
        object: type.typeReference,
        id,
        relation: AccessName.parse(set),
        scope,
    };
}

/** Permit the principal the object represents, acting on its row. */
export function self(): AccessExpression {
    return { kind: "self" };
}

/** Permit objects whose attributes meet a condition, reading request attributes as placeholders. */
export function resource(where: Condition): AccessExpression {
    return { kind: "resource", condition: where };
}

/** Permit requests whose attributes meet a condition. */
export function context(where: Condition): AccessExpression {
    return { kind: "context", condition: where };
}
