import { defineSchema, schema } from "@destack/schema";
import { PackageId, type Package } from "@destack/package";
import { AccessName, type AccessExpression } from "./expression.ts";
import { type RelationDefinition, type Subject, SubjectType } from "./subject.ts";
import { AccessError } from "../error/index.ts";
import { PolicyDescription } from "../inspect/policy.ts";
import type { Elevation } from "../context/elevation.ts";

/** A protected object: its type, the scope containing it, and its identifier there. */
export const ObjectReference = defineSchema(
    schema.object({
        /** The package that declares the object type. */
        packageId: PackageId,
        /** The declaration-local object type name. */
        type: AccessName,
        /** The scope containing the object, the container for a scope object. */
        scope: schema.string().min(1),
        /** The stable application record identity. */
        id: schema.string().min(1),
    }),
);
/** A protected object: its type, the scope containing it, and its identifier there. */
export type ObjectReference = schema.Infer<typeof ObjectReference>;

/** A package-qualified object type. */
export const TypeReference = defineSchema(ObjectReference.pick({ packageId: true, type: true }));
/** A package-qualified object type. */
export type TypeReference = schema.Infer<typeof TypeReference>;

/** The relations and permissions of one type of object, as data access evaluates. */
export interface PolicyDefinition {
    /** The stable declaring package identity. */
    readonly packageId: PackageId;
    /** The declaration-local object type name. */
    readonly name: string;
    /** Required object attributes used by permission expressions. */
    readonly attributes: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Relations to subjects and to other objects. */
    readonly relations: Readonly<Record<string, RelationDefinition>>;
    /** Named permission expressions; roles bound on the object grant every permission as well. */
    readonly permissions: Readonly<Record<string, AccessExpression>>;
    /** The permission whose holders bind roles on an object. */
    readonly grantedBy?: string;
    /** Permissions only their expressions grant, never roles, not even owners'. */
    readonly reserved?: readonly string[];
    /** Sensitive permissions that apply only after the authentication each names, whoever holds them. */
    readonly elevated?: Readonly<Record<string, Elevation>>;
    /** Permissions that stay available while the scope is suspended, such as reading and resuming it. */
    readonly administration?: readonly string[];
    /** Whether the type's objects are scopes, holding other objects while living in their own container like any object. */
    readonly scope?: true;
    /** The open relations of other types this type's objects may be subjects of. */
    readonly contributes?: readonly {
        readonly packageId: PackageId;
        readonly type: string;
        readonly relation: string;
    }[];
}

/** The policy of one type of object, with typed references to its permissions and relations. */
export class Policy<Name extends string = string> {
    /** The declaring package, supplied by the module transform. */
    readonly package: Package;
    /** The package-local object type name. */
    readonly name: string;
    /** The immutable serializable definition. */
    readonly definition: PolicyDefinition;
    /** The other policies the relations name as subjects. */
    readonly references: readonly Policy[];

    /** Qualify the declared subject types by package and freeze a copy of the rules. */
    constructor(owner: Package, input: PolicyInput<Name>) {
        // qualify subject types named by the declaring package, and grant relations as the policy does unless they say otherwise
        const relations = Object.fromEntries(
            Object.entries(input.relations ?? {}).map(([name, relation]) => {
                const grantedBy =
                    relation.grantedBy === undefined ? input.grantedBy : relation.grantedBy;

                return [
                    name,
                    {
                        subjects: relation.subjects.map((subject) =>
                            subjectType(owner.id, subject),
                        ),
                        ...(grantedBy === undefined || grantedBy === null ? {} : { grantedBy }),
                        ...(relation.open ? { open: true } : {}),
                    },
                ];
            }),
        );

        // require every relation to accept a subject type unless other types contribute them
        for (const [name, relation] of Object.entries(relations)) {
            if (relation.subjects.length === 0 && !relation.open) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `relation ${name} accepts no subject type and is not open`,
                );
            }
        }

        // remember the policies named as subjects and the policies contributed to
        this.references = [
            ...new Set([
                ...Object.values(input.relations ?? {})
                    .flatMap((relation) => relation.subjects)
                    .flatMap((subject) =>
                        subject instanceof Policy
                            ? [subject]
                            : subject instanceof PolicySubject
                              ? [subject.policy]
                              : [],
                    ),
                ...(input.contributes ?? []).map((entry) => entry.policy),
            ]),
        ];

        // retain the declaring package and a frozen copy of the rules
        this.package = owner;
        this.name = input.name;
        this.definition = freeze(
            PolicyDescription.parse({
                packageId: owner.id,
                name: input.name,
                attributes: input.attributes ?? {},
                relations,
                permissions: input.permissions,
                ...(input.grantedBy === undefined ? {} : { grantedBy: input.grantedBy }),
                ...(input.reserved === undefined ? {} : { reserved: [...input.reserved] }),
                ...(input.elevated === undefined ? {} : { elevated: { ...input.elevated } }),
                ...(input.administration === undefined
                    ? {}
                    : { administration: [...input.administration] }),
                ...(input.scope ? { scope: true } : {}),
                ...(input.contributes === undefined || input.contributes.length === 0
                    ? {}
                    : {
                          contributes: input.contributes.map((entry) => ({
                              packageId: entry.policy.definition.packageId,
                              type: entry.policy.definition.name,
                              relation: entry.relation,
                          })),
                      }),
            }) as PolicyDefinition,
        );
    }

    /** Identify an object of this type in the scope containing it. */
    reference(scope: string, id: string): ObjectReference {
        return ObjectReference.parse({
            packageId: this.definition.packageId,
            type: this.definition.name,
            scope,
            id,
        });
    }

    /** Reference one of this type's declared permissions. */
    permission(name: Name): Permission {
        if (!Object.hasOwn(this.definition.permissions, name)) {
            throw new AccessError("INVALID_DECLARATION", `unknown permission: ${name}`);
        }

        const reference: PermissionReference = {
            packageId: this.definition.packageId,
            type: this.definition.name,
            name,
        };

        return reference as Permission;
    }

    /** Determine whether a subject is one object of this type, rather than a set or another type's object. */
    is(subject: Subject): boolean {
        return (
            subject.relation === undefined &&
            subject.packageId === this.definition.packageId &&
            subject.type === this.definition.name
        );
    }

    /** Accept the members of one of this type's relations as subjects. */
    members(relation: string): PolicySubject {
        return new PolicySubject(this, AccessName.parse(relation), false);
    }

    /** Accept every object of this type through one relationship. */
    all(): PolicySubject {
        return new PolicySubject(this, undefined, true);
    }
}

/** A subject type taken from a policy: the members of one of its relations, or all of its objects. */
export class PolicySubject {
    /** The policy whose objects are the subjects. */
    readonly policy: Policy;
    /** The relation whose members are the subjects, for a subject set. */
    readonly relation: string | undefined;
    /** Whether one relationship relates every object of the type. */
    readonly isWildcard: boolean;

    /** Retain the policy and the form of its subjects. */
    constructor(policy: Policy, relation: string | undefined, isWildcard: boolean) {
        this.policy = policy;
        this.relation = relation;
        this.isWildcard = isWildcard;
    }

    /** Qualify the subject type by the policy's package. */
    type(): SubjectType {
        return {
            packageId: this.policy.definition.packageId,
            type: this.policy.definition.name,
            ...(this.relation === undefined ? {} : { relation: this.relation }),
            ...(this.isWildcard ? { wildcard: true as const } : {}),
        };
    }
}

/** A stable reference to a declared permission, independent of a package version. */
export const PermissionReference = defineSchema(
    schema.object({
        /** The package that declares the permission. */
        packageId: PackageId,
        /** The declaration-local object type name. */
        type: AccessName,
        /** The permission name within that object type. */
        name: AccessName,
    }),
);
/** A stable reference to a declared permission, independent of a package version. */
export type PermissionReference = schema.Infer<typeof PermissionReference>;

/** Marks the permission references a policy produced. */
declare const DECLARED: unique symbol;

/** A permission of a policy, which only `Policy.permission` produces. */
export type Permission = PermissionReference & { readonly [DECLARED]: true };

/**
 * A subject type accepted by a policy.
 *
 * It is a policy's objects or members, a qualified subject type, or a type of the declaring package written `type`, `type#relation` or `type:*`.
 */
export type SubjectTypeInput = string | Policy | PolicySubject | SubjectType;

/** A relation as declared, with subject types not yet qualified by package. */
export interface RelationInput {
    /** The subject types the relation accepts. */
    readonly subjects: readonly SubjectTypeInput[];
    /** The permission whose holders grant and revoke the relation, the policy's when absent, none when null. */
    readonly grantedBy?: string | null;
    /** Whether other types contribute themselves as subject types, as the hosts of an attachment do. */
    readonly open?: true;
}

/** A policy as declared. */
export interface PolicyInput<Name extends string> {
    /** The declaration-local object type name. */
    readonly name: string;
    /** Required object attributes used by permission expressions. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** Relations to subjects and to other objects. */
    readonly relations?: Readonly<Record<string, RelationInput>>;
    /** Named permission expressions. */
    readonly permissions: Readonly<Record<Name, AccessExpression>>;
    /** The permission whose holders bind roles on an object, and grant the relations naming no other. */
    readonly grantedBy?: NoInfer<Name>;
    /** Permissions only their expressions grant, never roles, not even owners'. */
    readonly reserved?: readonly NoInfer<Name>[];
    /** Sensitive permissions that apply only after the authentication each names, whoever holds them. */
    readonly elevated?: Readonly<Partial<Record<NoInfer<Name>, Elevation>>>;
    /** Permissions that stay available while the scope is suspended, such as reading and resuming it. */
    readonly administration?: readonly NoInfer<Name>[];
    /** Whether the type's objects are scopes, holding other objects while living in their own container like any object. */
    readonly scope?: boolean;
    /** The open relations of other types this type's objects may be subjects of, such as an attachment's parent. */
    readonly contributes?: readonly { readonly policy: Policy; readonly relation: string }[];
}

/** Qualify a declared subject type with the package declaring it. */
export function subjectType(packageId: PackageId, subject: SubjectTypeInput): SubjectType {
    // accept a policy's objects
    if (subject instanceof Policy) {
        return { packageId: subject.definition.packageId, type: subject.definition.name };
    }
    // accept a policy's members or all of its objects
    else if (subject instanceof PolicySubject) {
        return subject.type();
    }
    // accept a qualified subject type
    else if (typeof subject !== "string") {
        return SubjectType.parse(subject);
    }

    // parse `type`, `type#relation` and `type:*`
    const match = /^([a-z][a-z0-9-]*)(?:#([a-z][a-z0-9-]*)|(:\*))?$(?![\s\S])/.exec(subject);
    if (!match) {
        throw new AccessError("INVALID_DECLARATION", `invalid subject type: ${subject}`);
    }

    return SubjectType.parse({
        packageId,
        type: match[1],
        ...(match[2] === undefined ? {} : { relation: match[2] }),
        ...(match[3] === undefined ? {} : { wildcard: true }),
    });
}

/** Construct a collision-free permission key. */
export function permissionKey(
    permission: Pick<PermissionReference, "packageId" | "type" | "name">,
): string {
    return JSON.stringify([permission.packageId, permission.type, permission.name]);
}

/** Construct a collision-free reference key. */
export function objectKey(reference: ObjectReference): string {
    return JSON.stringify([reference.packageId, reference.type, reference.scope, reference.id]);
}

/** Freeze every declaration node after copying it. */
function freeze<Value>(value: Value): Value {
    // freeze the children before the node
    if (value !== null && typeof value === "object") {
        for (const child of Object.values(value)) {
            freeze(child);
        }
        Object.freeze(value);
    }

    return value;
}

/** List the relations an expression reads, directly or through an arrow. */
export function relationsOf(expression: AccessExpression): string[] {
    switch (expression.kind) {
        case "relation":
            return [expression.name];
        case "through":
            return [expression.relation];
        case "union":
        case "intersection":
            return expression.expressions.flatMap(relationsOf);
        case "exclusion":
            return [...relationsOf(expression.include), ...relationsOf(expression.exclude)];
        default:
            return [];
    }
}
