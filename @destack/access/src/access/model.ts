import { AccessError } from "../error/index.ts";
import { ObjectType, type ObjectReference, type PermissionReference } from "./object.ts";
import type { AccessExpression } from "./expression.ts";
import type { AccessPolicy } from "./policy.ts";
import { AccessPolicyDescription } from "../inspect/policy.ts";

/** Validated declarations shared by query compilation and memory evaluation. */
export class AccessModel {
    /** Object declarations indexed by package and type. */
    readonly #types = new Map<string, ObjectType>();
    /** Mandatory restrictions supplied by the hosting authority. */
    readonly policies: readonly AccessPolicy[];

    /** Validate declarations and reject ambiguous references or permission cycles. */
    constructor(types: readonly ObjectType[], policies: readonly AccessPolicy[] = []) {
        // parse and retain immutable host policies
        this.policies = Object.freeze(
            policies.map((policy) => freezePolicy(AccessPolicyDescription.parse(policy))),
        );

        // register the complete model before resolving cross-type relationships
        for (const type of types) {
            const key = JSON.stringify([type.definition.packageId, type.definition.name]);
            if (this.#types.has(key)) {
                throw new AccessError("INVALID_DECLARATION", `duplicate object type: ${key}`);
            }

            this.#types.set(key, type);
        }

        // validate every named permission, including unused administrative permissions
        for (const type of types) {
            const definition = type.definition;
            for (const relation of Object.values(definition.relations)) {
                if (relation.kind === "grant") {
                    this.expression(type.permission(relation.permission));
                } else if (relation.kind === "object") {
                    this.type({ packageId: definition.packageId, type: relation.type });
                }
            }

            // validate named permissions after resolving the declared relations
            for (const name of Object.keys(definition.permissions)) {
                this.#validate(type.permission(name), new Set());
            }
        }

        // validate host restrictions against the same declarations
        for (const policy of this.policies) {
            for (const name of policy.permissions) {
                const reference = { packageId: policy.packageId, type: policy.type, name };
                this.expression(reference);
            }

            this.#validateExpression(this.type(policy), policy.condition, new Set());
        }
    }

    /** Resolve the declaration for a package-qualified object type. */
    type(reference: Pick<ObjectReference, "packageId" | "type">): ObjectType {
        const type = this.#types.get(JSON.stringify([reference.packageId, reference.type]));
        if (!type) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `unknown object type: ${reference.packageId}/${reference.type}`,
            );
        }

        return type;
    }

    /** Resolve a permission without accepting inherited JavaScript properties. */
    expression(reference: PermissionReference): AccessExpression {
        const definition = this.type(reference).definition;
        if (!Object.hasOwn(definition.permissions, reference.name)) {
            throw new AccessError("INVALID_DECLARATION", `unknown permission: ${reference.name}`);
        }

        return definition.permissions[reference.name];
    }

    /** Return serializable definitions in registration order. */
    describe() {
        return [...this.#types.values()].map((type) => type.definition);
    }

    /** Resolve all expressions and keep recursion in explicit ancestor traversal. */
    #validate(reference: PermissionReference, parents: ReadonlySet<string>): void {
        // detect recursion by the complete permission reference
        const key = JSON.stringify([reference.packageId, reference.type, reference.name]);
        if (parents.has(key)) {
            throw new AccessError("INVALID_DECLARATION", `cyclic permission: ${reference.name}`);
        }

        // retain the named permission path while checking its expression
        const path = new Set(parents).add(key);
        this.#validateExpression(this.type(reference), this.expression(reference), path);
    }

    /** Resolve references and scalar comparisons within a declared expression. */
    #validateExpression(type: ObjectType, root: AccessExpression, path: ReadonlySet<string>): void {
        // inspect each expression against its enclosing object declaration
        const definition = type.definition;
        const pending = [root];

        // resolve references in the already parsed expression tree
        while (pending.length > 0) {
            const expression = pending.pop()!;
            switch (expression.kind) {
                case "union":
                case "intersection":
                    pending.push(...expression.expressions);
                    break;
                case "exclusion":
                    pending.push(expression.include, expression.exclude);
                    break;
                case "relation": {
                    const relation = definition.relations[expression.name];
                    if (
                        !Object.hasOwn(definition.relations, expression.name) ||
                        relation.kind === "object"
                    ) {
                        throw new AccessError(
                            "INVALID_DECLARATION",
                            `unknown subject relation: ${expression.name}`,
                        );
                    }
                    break;
                }
                case "permission":
                    this.#validate(type.permission(expression.name), path);
                    break;
                case "through": {
                    const relation = definition.relations[expression.relation];
                    if (
                        !Object.hasOwn(definition.relations, expression.relation) ||
                        relation.kind !== "object"
                    ) {
                        throw new AccessError(
                            "INVALID_DECLARATION",
                            `unknown object relation: ${expression.relation}`,
                        );
                    }

                    // require ancestor traversal to stay within a single object type
                    if (expression.transitive && relation.type !== definition.name) {
                        throw new AccessError(
                            "INVALID_DECLARATION",
                            "ancestor traversal requires a same-type relation",
                        );
                    }

                    // resolve the referenced permission on the related type
                    this.#validate(
                        {
                            packageId: definition.packageId,
                            type: relation.type,
                            name: expression.permission,
                        },
                        path,
                    );
                    break;
                }
                case "compare":
                    validateComparison(expression, type);
                    break;
            }
        }
    }
}

/** Require declared attributes and compatible scalar operands. */
function validateComparison(
    expression: Extract<AccessExpression, { kind: "compare" }>,
    type: ObjectType,
): void {
    // resolve object operands against the declared attribute types
    for (const operand of [expression.left, expression.right]) {
        if (
            operand.kind === "object" &&
            type.definition.attributes[operand.name] !== operand.type
        ) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `undeclared object attribute: ${operand.name}`,
            );
        }
    }

    // require matching scalar types and numeric ordering
    const left =
        expression.left.kind === "literal" ? typeof expression.left.value : expression.left.type;
    const right =
        expression.right.kind === "literal" ? typeof expression.right.value : expression.right.type;
    if (left !== right || (!["eq", "ne"].includes(expression.operator) && left !== "number")) {
        throw new AccessError(
            "INVALID_DECLARATION",
            "ordered comparisons require numbers and equality requires matching types",
        );
    }
}

/** Freeze parsed policy expressions before publishing a reusable model. */
function freezePolicy<Value>(value: Value): Value {
    if (value !== null && typeof value === "object") {
        for (const child of Object.values(value)) {
            freezePolicy(child);
        }
        Object.freeze(value);
    }

    return value;
}
