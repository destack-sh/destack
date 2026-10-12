import * as access from "@destack/access";
import {
    Authorizer,
    Policy,
    principal,
    relationsOf,
    type AccessExpression,
    type RelationInput,
} from "@destack/access";
import type { Package, PackageId } from "@destack/package";
import type { Field } from "../field/field.ts";
import { Roles } from "../trait/shareable.ts";
import type { TraitObject, TraitPolicy } from "../trait/trait.ts";
import { camelCase, kebabCase } from "./name.ts";
import { Scope, type ObjectReference } from "@destack/sync";
import { TABLE, type Select, type Table } from "@destack/db";
import {
    ACCESS_MAPPINGS,
    accessInvitation,
    accessRelationship,
    accessRole,
    RoleRequest,
    type TableMapping,
} from "@destack/access";
import { present, schema } from "@destack/schema";
import { type Call } from "../method/call.ts";
import type { Method, MethodBuilder } from "../method/method.ts";
import { INTRINSIC } from "./intrinsic.ts";
import {
    ObjectPermissions,
    ObjectType,
    SCOPE,
    type ObjectDefinition,
    type ObjectRelationInput,
    type ObjectScope,
} from "./object.ts";

/** The access of a type's objects: deciding policy, row mapping, and multi-type authorizer. */
export const ObjectAccess = {
    /** Build a type's policy of declared permissions and relations with its traits' policies. */
    policy(
        type: ObjectType,
        definition: ObjectDefinition,
        owner: Package,
    ): { permissions: string[]; policy: Policy } {
        // resolve the relations declared, read from fields and enclosing scopes
        const permissions = ObjectPermissions.expressions(definition.permissions ?? []);
        const relations = declaredRelations(type, definition, owner, permissions);

        // collect the traits' policies and the permissions they derive
        const names = Object.keys(permissions);
        const object = traitObject(type, (definition.represents?.package ?? owner).id);
        const policies = type.traits.flatMap(({ trait, options }) =>
            trait.policy === undefined ? [] : [trait.policy(options, object, names)],
        );
        const derived = Object.fromEntries(
            policies.flatMap((policy) => Object.entries(policy.permissions ?? {})),
        );

        // register the policy over declared and derived permissions
        const policy = objectPolicy(owner, definition, {
            relations,
            permissions,
            derived,
            policies,
        });

        return { permissions: [...names, ...Object.keys(derived)], policy };
    },

    /** Map a type's rows for access: its table, relations and attributes. */
    mapping(type: ObjectType): TableMapping {
        // copy every inherited row the condition matches
        const inherited =
            type.inherited === undefined ? {} : { inherited: type.inherited.where ?? {} };

        // reuse an intrinsic mapping
        if (type.intrinsic !== undefined) {
            return { ...type.intrinsic, policy: type.policy, ...inherited };
        }

        // map each declared relation to its field and add trait mappings
        const relations = fieldRelations(type);
        const placed: Partial<Pick<TableMapping, "parent" | "trees">> = {};
        for (const { relations: traitRelations, ...rest } of traitMappings(type)) {
            Object.assign(relations, traitRelations);
            Object.assign(placed, rest);
        }

        // map the self relation to the principal each object stands for under its own identifier, in the universe for a global principal
        if (Object.hasOwn(type.policy.definition.relations, "self")) {
            relations["self"] =
                type.represents?.definition.isGlobal === true
                    ? { column: "id", scope: Scope.universe.id }
                    : { column: "id" };
        }

        // assemble the mapping
        return {
            policy: type.policy,
            table: type.table,
            id: "id",
            scope: "scope",
            attributes: Object.fromEntries(
                Object.keys(type.policy.definition.attributes).map((name) => [
                    name,
                    camelCase(name),
                ]),
            ),
            relations,
            ...(type.owner === undefined ? {} : { owner: type.owner }),
            ...inherited,
            ...placed,
        };
    },

    /** Build the authorizer of object types, their enclosing scope types, and related types. */
    authorizer(
        objects: readonly ObjectType[],
        others: readonly (Policy | ObjectType | TableMapping)[],
        copies: (table: Table) => boolean,
    ): Authorizer {
        // collect the object types and the types they relate to
        const types = [...objects, ...others.filter((other) => other instanceof ObjectType)];
        const related = [
            ...new Set(
                types
                    .flatMap((object) => object.related)
                    .filter((target) => !types.some((type) => type.same(target))),
            ),
        ];

        return new Authorizer(
            [
                ...objects.flatMap((object) => [
                    object.policy,
                    ...object.scopes
                        .flatMap((scope) => [scope, ...scope.ancestors])
                        .map((scope) => scope.policy),
                ]),
                ...others.map((other) => (other instanceof Policy ? other : other.policy)),
                ...related.map((object) => object.policy),
            ],
            [
                ...[...types, ...related].map((object) => object.mapping),
                ...others.flatMap((other) =>
                    other instanceof Policy || other instanceof ObjectType ? [] : [other],
                ),
            ],
            { copies },
        );
    },
};

/** Define a scope type's roles as objects. */
export function role<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "role",
        plural: "roles",
        [INTRINSIC]: { table: accessRole, mapping: mappingOf(accessRole) },
        scope,
        represents: access.role,
        permissions: access.role.definition.permissions,
        methods: (method: MethodBuilder<typeof accessRole>) => ({
            get: method.get("read"),
            list: method.list("read"),
            create: unpredicted(
                method.create("create", {
                    fields: ["name", "description"],
                    input: RoleRequest.pick({ permissions: true }),
                }),
            ).handle(async (call) => {
                // create the role through the access role API
                const { name, description, permissions } = call.input;
                const created = await call
                    .requireAuthorization()
                    .createRole(await scopeObject(call), { name, description, permissions });

                return readRow(call, created.id);
            }),
            update: unpredicted(
                method.update("update", {
                    fields: ["name", "description"],
                    input: schema.object({
                        permissions: RoleRequest.shape.permissions.exactOptional(),
                    }),
                }),
            ).handle(async (call) => {
                // change the given values at the loaded revision
                const { name, description, permissions } = call.input;
                await call
                    .requireAuthorization()
                    .updateRole(await scopeObject(call), call.target.id, {
                        ...(name === undefined ? {} : { name }),
                        ...(description === undefined ? {} : { description }),
                        ...(permissions === undefined ? {} : { permissions }),
                        revision: call.target.revision,
                    });

                return readRow(call, call.target.id);
            }),
            delete: unpredicted(method.delete("delete")).handle(async (call) => {
                // delete the unbound role at the loaded revision
                await call
                    .requireAuthorization()
                    .deleteRole(await scopeObject(call), call.target.id, call.target.revision);

                return {};
            }),
        }),
    } as const;
}

/** Define a scope type's relationships as objects, changed only through sharing methods. */
export function relationship<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "relationship",
        plural: "relationships",
        [INTRINSIC]: { table: accessRelationship, mapping: mappingOf(accessRelationship) },
        scope,
        represents: access.relationship,
        permissions: access.relationship.definition.permissions,
        methods: (method: MethodBuilder<typeof accessRelationship>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Define a scope type's invitations as objects, answered through sharing methods. */
export function invitation<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "invitation",
        plural: "invitations",
        [INTRINSIC]: { table: accessInvitation, mapping: mappingOf(accessInvitation) },
        scope,
        represents: access.invitation,
        permissions: access.invitation.definition.permissions,
        methods: (method: MethodBuilder<typeof accessInvitation>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Keep a method on the server. */
function unpredicted<Declared extends Method>(declared: Declared): Declared {
    return { ...declared, isPredicted: false };
}

/** Find the scope object of the call's scope. */
function scopeObject(call: Call): Promise<ObjectReference> {
    return Scope.object(call.view(), call.scope);
}

/** Read one role row as the method's result. */
async function readRow(
    call: Call,
    id: Select<typeof accessRole>["id"],
): Promise<Select<typeof accessRole>> {
    return present(await call.requireTransaction().row(accessRole, { id }), `role ${id}`);
}

/** Read how access maps one of its own tables. */
function mappingOf(table: Table): TableMapping {
    return present(
        ACCESS_MAPPINGS.find((mapping) => mapping.table === table),
        `the access mapping of ${table[TABLE].name}`,
    );
}

/** Resolve a definition's declared relations, its permissions' field relations, and scopes. */
function declaredRelations(
    type: ObjectType,
    definition: ObjectDefinition,
    owner: Package,
    permissions: Readonly<Record<string, AccessExpression>>,
): Record<string, RelationInput> {
    // resolve declared relations
    const relations: Record<string, RelationInput> = {};
    for (const [name, relation] of Object.entries(definition.relations ?? {})) {
        relations[name] = {
            subjects: relation.subjects.map((subject) => subjectOf(subject, owner)),
            ...(relation.grantedBy === undefined ? {} : { grantedBy: relation.grantedBy }),
            ...(relation.concealed === true ? { concealed: true } : {}),
        };
    }

    // derive relations from fields permissions read, refusing them on a scope, whose enclosed objects decide through relationships alone
    const decided = new Set(Object.values(permissions).flatMap(relationsOf));
    for (const [property, field] of Object.entries(definition.fields ?? {})) {
        const name = kebabCase(field.relationAt(property));
        const relation =
            decided.has(name) && relations[name] === undefined ? fieldRelation(field) : undefined;
        if (relation !== undefined && definition.isScope === true) {
            throw new TypeError(
                `scope ${definition.name} reads relation ${name} from a field, which its enclosed objects cannot decide by`,
            );
        } else if (relation !== undefined) {
            relations[name] = relation;
        }
    }

    // relate the objects to their scope when a permission reads through it, by its type or as any scope type it lives in
    const enclosing = type.scopes.filter(
        (scope) => decided.has(scope.name) && relations[scope.name] === undefined,
    );
    for (const scope of enclosing) {
        relations[scope.name] = { subjects: [scope.policy], grantedBy: null, isScope: true };
    }
    if (decided.has(SCOPE) && relations[SCOPE] === undefined) {
        const subjects = type.scopes.map((scope) => scope.policy);
        relations[SCOPE] = { subjects, grantedBy: null, isScope: true };
    }

    return relations;
}

/** Resolve a declared relation subject: a type's policy, a package subject type, or as is. */
function subjectOf(
    subject: ObjectRelationInput["subjects"][number],
    owner: Package,
): RelationInput["subjects"][number] {
    // take an object type's policy
    if (subject instanceof ObjectType) {
        return subject.policy;
    }
    // take a subject type of the package by name
    else if (typeof subject === "string") {
        return Policy.subjectType(owner.id, subject);
    }
    // keep any other subject
    else {
        return subject;
    }
}

/** Relate a field to the subjects it refers to, absent for a field referring to none. */
function fieldRelation(field: Field): RelationInput | undefined {
    // relate a principal reference to its kind
    if (field.type === "reference" && field.principals !== undefined) {
        return { subjects: field.principals, grantedBy: null };
    }
    // relate a plain reference to its target type
    else if (field.type === "reference" && field.target && !field.qualified) {
        return { subjects: [field.target().policy], grantedBy: null };
    }
    // relate a subject field to its principal kinds
    else if (field.type === "subject") {
        const subjects = field.principals ?? [
            principal.user,
            principal.machine,
            principal.installation,
        ];

        return { subjects, grantedBy: null };
    }
    // relate nothing else
    else {
        return undefined;
    }
}

/** Assemble a type's policy from its declared and represented rules and its traits' policies. */
function objectPolicy(
    owner: Package,
    definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
    rules: {
        /** The relations the definition declares, reads from fields and enclosing scopes. */
        readonly relations: Readonly<Record<string, RelationInput>>;
        /** The permission expressions the definition declares. */
        readonly permissions: Readonly<Record<string, AccessExpression>>;
        /** The permission expressions the traits derive. */
        readonly derived: Readonly<Record<string, AccessExpression>>;
        /** The traits' policies. */
        readonly policies: readonly TraitPolicy[];
    },
): Policy {
    // refuse to represent a policy of another name
    const represented = definition.represents;
    if (represented && represented.name !== definition.name) {
        throw new TypeError(`object ${definition.name} cannot represent ${represented.name}`);
    }

    // combine the represented, declared and trait rules
    const { relations, permissions, derived, policies } = rules;
    const gate = Roles.gate(definition);

    return new Policy(represented?.package ?? owner, {
        name: definition.name,
        attributes: Object.fromEntries([
            ...policies.flatMap((policy) => Object.entries(policy.attributes ?? {})),
            ...Object.entries(definition.attributes ?? {}),
        ]),
        relations: {
            ...representedRelations(represented),
            ...relations,
            ...Object.fromEntries(
                policies.flatMap((policy) => Object.entries(policy.relations ?? {})),
            ),
        },
        permissions: {
            ...represented?.definition.permissions,
            ...permissions,
            ...derived,
        },
        ...(gate === undefined ? {} : { grantedBy: gate }),
        ...(definition.shareable?.read === undefined
            ? {}
            : { relationships: { read: definition.shareable.read } }),
        ...(definition.reserved === undefined ? {} : { reserved: definition.reserved }),
        ...(definition.elevated === undefined ? {} : { elevated: definition.elevated }),
        ...(definition.administration === undefined
            ? {}
            : { administration: definition.administration }),
        ...(definition.isScope === true || represented?.definition.scope === true
            ? { scope: true }
            : {}),
        ...(definition.partition === true ? { partition: true } : {}),
        ...(represented?.definition.isGlobal === true ? { isGlobal: true } : {}),
        contributes: policies.flatMap((policy) => policy.contributes ?? []),
    });
}

/** Map each declared relation a type's fields keep to its column. */
function fieldRelations(type: ObjectType): Record<string, TableMapping["relations"][string]> {
    const relations: Record<string, TableMapping["relations"][string]> = {};
    for (const [property, field] of Object.entries(type.fields)) {
        const name = kebabCase(field.relationAt(property));
        const isRelation = Object.hasOwn(type.policy.definition.relations, name);

        // keep a subject field of a relation as a subject key
        if (isRelation && field.type === "subject") {
            relations[name] = { column: property, isKey: true };
        }
        // map a principal reference in the universe
        else if (isRelation && field.type === "reference" && field.principals !== undefined) {
            relations[name] = { column: property, scope: Scope.universe.id };
        }
        // map a plain reference by identifier: in the universe, in the object's scope, or across scopes
        else if (isRelation && field.type === "reference" && field.target && !field.qualified) {
            const target = field.target();
            const isBeside = type.scopes.every((scope) =>
                target.scopes.some((other) => other.same(scope)),
            );
            relations[name] =
                target.scope === Scope.universe.id
                    ? { column: property, scope: Scope.universe.id }
                    : isBeside
                      ? { column: property }
                      : { column: property, scope: access.ANY_SCOPE };
        }
    }

    return relations;
}

/** List the mappings a type's traits add. */
function traitMappings(type: ObjectType) {
    const object = traitObject(type, type.policy.package.id);

    return type.traits.flatMap(({ trait, options }) =>
        trait.mapping === undefined ? [] : [trait.mapping(options, object)],
    );
}

/** Read a represented policy's relations as input. */
function representedRelations(represented: Policy | undefined): Record<string, RelationInput> {
    return Object.fromEntries(
        Object.entries(represented?.definition.relations ?? {}).map(([name, relation]) => [
            name,
            { subjects: relation.subjects, grantedBy: relation.grantedBy ?? null },
        ]),
    );
}

/** Describe an object type to its traits. */
function traitObject(type: ObjectType, packageId: PackageId): TraitObject {
    return {
        name: type.name,
        identity: type.identity,
        packageId,
        table: () => type.table,
        key: type.keyed,
        storage: type.storage,
    };
}
