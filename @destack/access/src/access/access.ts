import { AccessError } from "../error/index.ts";
import { isActive } from "../grant/grant.ts";
import {
    type AccessObject,
    type ObjectReference,
    type PermissionReference,
    objectKey,
} from "./object.ts";
import type { AccessView } from "./view.ts";
import { type AccessContext, type Attribute, type Subject, sameSubject } from "./subject.ts";
import type { AccessOperand, AccessExpression } from "./expression.ts";
import { AccessModel } from "./model.ts";
import { applies, permitsDelegation } from "./policy.ts";

/** A reproducible result over one authoritative record snapshot. */
export interface Decision {
    /** The evaluated permission. */
    readonly permission: PermissionReference;
    /** The protected object. */
    readonly object: ObjectReference;
    /** Whether the complete authorization rule permitted access. */
    readonly allowed: boolean;
    /** Matching grants visited while evaluating the rule. */
    readonly grants: readonly string[];
    /** Authoritative read-view revision used for this decision. */
    readonly revision: string | number;
}

/** Evaluate a fixed application snapshot without network or database calls. */
export class Access {
    /** Retain authoritative state by reference; the host supplies its consistency lifetime. */
    constructor(
        readonly model: AccessModel,
        readonly view: AccessView,
    ) {}

    /** Check a permission against a trusted request context. */
    check(
        permission: PermissionReference,
        object: ObjectReference,
        context: AccessContext,
    ): boolean {
        return this.#decide(permission, object, context);
    }

    /** Reject a request whose permission is not satisfied. */
    require(
        permission: PermissionReference,
        object: ObjectReference,
        context: AccessContext,
    ): void {
        if (!this.check(permission, object, context)) {
            throw new AccessError("FORBIDDEN", `permission denied: ${permission.name}`);
        }
    }

    /** Explain evaluated grants to a caller authorized to inspect access. */
    explain(
        permission: PermissionReference,
        object: ObjectReference,
        context: AccessContext,
    ): Decision {
        // evaluate only records reachable from this object and permission
        const grants = new Set<string>();
        const allowed = this.#decide(permission, object, context, grants);

        return {
            permission,
            object,
            allowed,
            grants: [...grants].sort(),
            revision: this.view.revision,
        };
    }

    /** Reject a decision if application code changes the read view during evaluation. */
    #decide(
        permission: PermissionReference,
        object: ObjectReference,
        context: AccessContext,
        grants?: Set<string>,
    ): boolean {
        // require the requested permission to describe this object and a valid request time
        if (
            permission.packageId !== object.packageId ||
            permission.type !== object.type ||
            !Number.isFinite(context.now)
        ) {
            throw new AccessError("INVALID_CONTEXT", "permission type or request time is invalid");
        }

        // retain the read revision while evaluating the represented subject
        const revision = this.view.revision;
        const record = this.#object(object);
        let allowed =
            permitsDelegation(permission, object, context) &&
            this.#evaluate(this.model.expression(permission), record, context, grants);

        // every software actor must also hold current permission on the target
        for (const delegation of context.delegations ?? []) {
            allowed =
                allowed &&
                this.#evaluate(
                    this.model.expression(permission),
                    record,
                    { ...context, subjects: [delegation.actor] },
                    grants,
                );
        }

        // apply mandatory host restrictions to the combined authority
        for (const policy of this.model.policies) {
            if (applies(policy, permission, object.scope)) {
                const matches = this.#evaluate(policy.condition, record, context, grants);
                allowed = allowed && (policy.effect === "restrict" ? matches : !matches);
            }
        }

        // reject changes to the authoritative view during the decision
        if (revision !== this.view.revision) {
            throw new AccessError("CONFLICT", "authorization records changed during evaluation");
        }

        return allowed;
    }

    /** Resolve an object from the supplied authoritative snapshot. */
    #object(reference: ObjectReference): AccessObject {
        const object = this.view.object(reference);
        if (!object) {
            throw new AccessError("NOT_FOUND", `missing access object: ${objectKey(reference)}`);
        }
        if (objectKey(object.reference) !== objectKey(reference)) {
            throw new AccessError("INVALID_CONTEXT", "read view returned a different object");
        }

        return object;
    }

    /** Interpret the same expression tree used by SQL compilation. */
    #evaluate(
        expression: AccessExpression,
        object: AccessObject,
        context: AccessContext,
        used?: Set<string>,
    ): boolean {
        switch (expression.kind) {
            case "union":
                return expression.expressions.some((child) =>
                    this.#evaluate(child, object, context, used),
                );
            case "intersection":
                return expression.expressions.every((child) =>
                    this.#evaluate(child, object, context, used),
                );
            case "exclusion":
                return (
                    this.#evaluate(expression.include, object, context, used) &&
                    !this.#evaluate(expression.exclude, object, context, used)
                );
            case "permission":
                return this.#evaluate(
                    this.model.expression({ ...object.reference, name: expression.name }),
                    object,
                    context,
                    used,
                );
            case "compare":
                return compareValues(
                    expression.operator,
                    readOperand(expression.left, object.attributes, context),
                    readOperand(expression.right, object.attributes, context),
                );
            case "relation":
                return this.#relation(expression.name, object, context, used);
            case "through":
                return this.#through(expression, object, context, used);
        }
    }

    /** Match application ownership or an active grant for one relation. */
    #relation(
        name: string,
        object: AccessObject,
        context: AccessContext,
        used?: Set<string>,
    ): boolean {
        // resolve direct ownership from the supplied application record
        const definition = this.model.type(object.reference).definition.relations[name];
        if (definition.kind === "subject") {
            const subjects = object.subjects[name];
            if (!subjects) {
                throw new AccessError("INVALID_CONTEXT", `missing subject relation: ${name}`);
            }

            return subjects.some(
                (subject) =>
                    definition.subjects.includes(subject.kind) &&
                    (subject.kind !== "share-token" ||
                        this.#activeToken(subject, object.reference.scope, context.now)) &&
                    context.subjects.some((candidate) => sameSubject(subject, candidate)),
            );
        }

        // match active grants only against subject kinds allowed by the declaration
        const grants = this.view.grants(object.reference);

        return grants.some((grant) => {
            // require the view to return grants for this exact object
            if (objectKey(grant.object) !== objectKey(object.reference)) {
                throw new AccessError(
                    "INVALID_CONTEXT",
                    "read view returned a grant for a different object",
                );
            }

            // skip grants outside the relation, subject kinds or active lifetime
            if (
                definition.kind !== "grant" ||
                grant.relation !== name ||
                !isActive(grant, context.now) ||
                !definition.subjects.includes(grant.subject.kind)
            ) {
                return false;
            }

            // require the current bearer token to remain active in the grant's scope
            if (grant.subject.kind === "share-token") {
                if (!this.#activeToken(grant.subject, grant.object.scope, context.now)) {
                    return false;
                }
            }

            // record the matching grant when an explanation was requested
            const subject = grant.subject;
            const allowed =
                subject.kind === "everyone" ||
                context.subjects.some((candidate) => sameSubject(candidate, subject));
            if (allowed) {
                used?.add(grant.id);
            }

            return allowed;
        });
    }

    /** Require a bearer subject to retain a current credential in this object's scope. */
    #activeToken(subject: Subject, scope: string, now: number): boolean {
        const token = this.view.token(scope, subject.id);
        if (token && (token.id !== subject.id || token.scope !== scope)) {
            throw new AccessError("INVALID_CONTEXT", "read view returned a different share token");
        }

        return subject.authority === scope && token !== undefined && isActive(token, now);
    }

    /** Traverse related objects once each while retaining the original subject context. */
    #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        object: AccessObject,
        context: AccessContext,
        used?: Set<string>,
    ): boolean {
        // seed the traversal with direct parents and exclude the starting object
        const pending = [...this.#related(object, expression.relation)];
        const visited = new Set<string>([objectKey(object.reference)]);

        // evaluate each reachable parent at most once
        for (let position = 0; position < pending.length; position++) {
            const reference = pending[position];
            const key = objectKey(reference);
            if (visited.has(key)) {
                continue;
            }
            visited.add(key);

            // retain the original scope while following application relationships
            const parent = this.#object(reference);
            const permission = this.model.expression({
                ...reference,
                name: expression.permission,
            });
            if (this.#evaluate(permission, parent, context, used)) {
                return true;
            }

            // continue through ancestors only when explicitly declared
            if (expression.transitive) {
                pending.push(...this.#related(parent, expression.relation));
            }
        }

        return false;
    }

    /** Require declared relationship records and reject cross-scope traversal. */
    #related(object: AccessObject, name: string): readonly ObjectReference[] {
        const references = object.objects[name];
        const relation = this.model.type(object.reference).definition.relations[name];
        if (!references || relation.kind !== "object") {
            throw new AccessError("INVALID_CONTEXT", `missing object relation: ${name}`);
        }

        // require every related object to match the declared type and original scope
        for (const reference of references) {
            if (
                reference.scope !== object.reference.scope ||
                reference.packageId !== object.reference.packageId ||
                reference.type !== relation.type
            ) {
                throw new AccessError(
                    "INVALID_CONTEXT",
                    "object relation crosses its declared type or scope",
                );
            }
        }

        return references;
    }
}

/** Read a required scalar without inventing a value for missing attributes. */
export function readOperand(
    operand: AccessOperand,
    attributes: Readonly<Record<string, Attribute>>,
    context: AccessContext,
): Attribute {
    if (operand.kind === "literal") {
        return operand.value;
    }

    // resolve a typed scalar from the application record or trusted context
    const source = operand.kind === "object" ? attributes : context.attributes;
    const value = source[operand.name];
    if (
        !Object.hasOwn(source, operand.name) ||
        typeof value !== operand.type ||
        (typeof value === "number" && !Number.isFinite(value))
    ) {
        throw new AccessError(
            "INVALID_CONTEXT",
            `missing or invalid ${operand.kind} attribute: ${operand.name}`,
        );
    }

    return value;
}

/** Compare values using SQL-compatible scalar equality and numeric ordering. */
function compareValues(
    operator: Extract<AccessExpression, { kind: "compare" }>["operator"],
    left: Attribute,
    right: Attribute,
): boolean {
    switch (operator) {
        case "eq":
            return left === right;
        case "ne":
            return left !== right;
        case "lt":
            return (left as number) < (right as number);
        case "lte":
            return (left as number) <= (right as number);
        case "gt":
            return (left as number) > (right as number);
        case "gte":
            return (left as number) >= (right as number);
    }
}
