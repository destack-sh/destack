import * as access from "@destack/access";
import { ObjectError } from "../error/error.ts";

import {
    ACCESS_MAPPINGS,
    type AccessExpression,
    Authorizer,
    Policy,
    principal,
    type RelationInput,
    relationsOf,
    type TableMapping,
} from "@destack/access";
import type { Package, PackageId } from "@destack/package";
import { Field } from "../field/field.ts";
import { Roles } from "../trait/shareable.ts";
import type { TraitObject, TraitPolicy } from "../trait/trait.ts";
import { Casing } from "./name.ts";
import { AccessName, ObjectReference, Scope, Subject } from "@destack/sync";
import { TABLE, type Table } from "@destack/db";
import { present, schema } from "@destack/schema";
import { type MethodContext } from "../method/context.ts";
import { type Method } from "../method/method.ts";
import {
    ObjectPermissions,
    ObjectScope,
    ObjectType,
    SCOPE,
    type ObjectDefinition,
    type ObjectRelationInput,
} from "./object.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

import type {} from "@destack/package/import-meta";

/** The access policy of a type's objects, with their row mapping and multi-type authorizer. */
export const ObjectPolicy = {
    /** Build a type's policy of declared permissions and relations with its traits' policies. */
    policy(
        type: ObjectType,
        definition: ObjectDefinition,
        owner: Package,
    ): { permissions: string[]; policy: Policy } {
        // collect the declared permissions, the traits' policies and the permissions they derive
        const permissions = ObjectPermissions.expressions(definition.permissions ?? []);
        const names = Object.keys(permissions);
        const object = ObjectPolicy.traitObject(type, (definition.represents?.package ?? owner).id);
        const policies = type.traits.flatMap(({ trait, options }) =>
            trait.policy === undefined ? [] : [trait.policy(options, object, names)],
        );
        const derived = Object.fromEntries(
            policies.flatMap((policy) => Object.entries(policy.permissions ?? {})),
        );

        // resolve the relations declared, read from fields and enclosing scopes by either kind of permission
        const read = { ...derived, ...permissions };
        const relations = ObjectPolicy.declaredRelations(type, definition, owner, read);

        // register the policy over declared and derived permissions
        const policy = ObjectPolicy.objectPolicy(owner, definition, {
            relations,
            permissions,
            derived,
            policies,
        });

        return { permissions: [...names, ...Object.keys(derived)], policy };
    },

    /** Map a type's rows for access: its table, relations and attributes. */
    mapping(type: ObjectType): TableMapping {
        // replicate every inherited row the condition matches
        const inherited =
            type.inherited === undefined ? {} : { inherited: type.inherited.where ?? {} };

        // reuse the mapping of another package's table
        const foreign = type.source.mapping;
        if (foreign !== undefined) {
            return { ...foreign, policy: type.policy, ...inherited };
        }

        // map each declared relation to its field and add trait mappings
        const relations = ObjectPolicy.fieldRelations(type);
        const placed: Partial<Pick<TableMapping, "parent" | "trees">> = {};
        for (const { relations: traitRelations, ...rest } of ObjectPolicy.traitMappings(type)) {
            Object.assign(relations, traitRelations);
            Object.assign(placed, rest);
        }

        // map the self relation to the principal each object stands for under its identifier, in the universe for a global principal
        const self = type.definition.relations?.["self"];
        if (Object.hasOwn(type.policy.definition.relations, "self")) {
            const isGlobal =
                type.represents?.definition.isGlobal === true ||
                (self !== undefined &&
                    self.subjects.every(
                        (subject) =>
                            subject instanceof Policy && subject.definition.isGlobal === true,
                    ));
            relations["self"] = isGlobal
                ? { column: "id", scope: Scope.universe.id }
                : { column: "id" };
        }

        // assemble the mapping
        return {
            policy: type.policy,
            table: type.table,
            id: "id",
            ...(type.isUniverseOnly ? {} : { scope: "scope" }),
            attributes: Object.fromEntries(
                Object.keys(type.policy.definition.attributes).map((name) => [
                    name,
                    Casing.camel(name),
                ]),
            ),
            relations,
            ...(type.owner === undefined ? {} : { owner: type.owner }),
            ...inherited,
            ...placed,
        };
    },

    /** Build the authorizer of stored object types, their scope types and related types. */
    authorizer(
        objects: readonly ObjectType[],
        others: readonly (Policy | ObjectType | TableMapping)[],
        replicated: ReadonlySet<Table>,
        options: {
            /** The one partition whose scopes' access the database stores, else none. */
            readonly partition?: string | undefined;
            /** The mappings of subject set types, registered once a policy grants their sets. */
            readonly sets?: readonly TableMapping[];
        } = {},
    ): Authorizer {
        // collect each object type once and the types they relate to
        const types: ObjectType[] = [];
        for (const type of [...objects, ...others.filter((other) => other instanceof ObjectType)]) {
            if (!types.some((each) => each.same(type))) {
                types.push(type);
            }
        }
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
            {
                replicated,
                tables: new Set([...replicated, ...objects.map((object) => object.table)]),
                ...(options.partition === undefined ? {} : { partition: options.partition }),
                ...(options.sets === undefined ? {} : { sets: options.sets }),
            },
        );
    },

    /** Decide a subject's permission on an object the call's scope decides. */
    async check(
        context: MethodContext,
        { authorization }: StoreExecution,
    ): Promise<schema.Infer<typeof CheckResult>> {
        // require an object the call's scope decides, and a permission its type declares
        const { subject, permission, object } = CheckInput.parse(context.input);
        const { authorizer, snapshot } = authorization;
        const policy = authorizer.policy(object);
        const scope = authorizer.governingScope(object);
        if (scope !== context.scope) {
            throw ObjectError.of(
                "SCOPE_MISMATCH",
                `${object.type} ${object.id} is checked in ${scope}, not ${context.scope}`,
            );
        } else if (!Object.hasOwn(policy.definition.permissions, permission)) {
            throw ObjectError.of(
                "PERMISSION_UNKNOWN",
                `${object.type} has no permission ${permission}`,
            );
        }
        const asked = { packageId: object.packageId, type: object.type, name: permission };
        const asking = await authorization.in(scope);

        // decide the caller's access under its delegates and grant
        const caller = context.caller.subject;
        if (caller !== undefined && Subject.same(caller, subject)) {
            return {
                isAllowed: (await authorizer.check(snapshot, asked, object, asking)).isAllowed,
            };
        }

        // decide another subject's access only for the object's grantors, as explanations do
        const gate = policy.definition.grantedBy;
        const granting =
            gate === undefined
                ? undefined
                : await authorizer.check(snapshot, { ...asked, name: gate }, object, asking);
        if (granting?.isAllowed !== true) {
            throw ObjectError.of(
                "CHECK_REFUSED",
                `checking another subject's access to ${object.type} needs ${gate ?? "a grant permission"}`,
            );
        }

        return { isAllowed: await context.permits(subject, asked, object) };
    },

    /** Refuse a relationship or invitation outside the call's scope, the scope it lives in. */
    async requireScope(
        context: MethodContext,
        request: { readonly object: ObjectReference } & ({ relation: string } | { role: string }),
        { authorization }: StoreExecution,
    ): Promise<void> {
        const scope = await access.Relationship.scope(authorization.snapshot, request);
        if (scope !== context.scope) {
            throw ObjectError.of(
                "SCOPE_MISMATCH",
                `the relationship on ${request.object.type} ${request.object.id} is in ${scope}, not ${context.scope}`,
            );
        }
    },

    /** Find the scope object of the call's scope. */
    scopeObject(context: MethodContext, execution: Execution): Promise<ObjectReference> {
        return Scope.object(Execution.snapshot(execution), context.scope);
    },

    /** Read how access maps one of its tables, typed by that table. */
    readMapping<Mapped extends Table>(
        table: Mapped,
    ): Omit<TableMapping, "table" | "policy"> & { readonly table: Mapped } {
        const { policy: _policy, ...mapping } = present(
            ACCESS_MAPPINGS.find((each) => each.table === table),
            `the access mapping of ${table[TABLE].name}`,
        );

        return { ...mapping, table };
    },

    /** Resolve a definition's declared relations, its permissions' field relations, and scopes. */
    declaredRelations(
        type: ObjectType,
        definition: ObjectDefinition,
        owner: Package,
        permissions: Readonly<Record<string, AccessExpression>>,
    ): Record<string, RelationInput> {
        // resolve declared relations
        const relations: Record<string, RelationInput> = {};
        for (const [name, relation] of Object.entries(definition.relations ?? {})) {
            relations[name] = {
                subjects: relation.subjects.map((subject) =>
                    ObjectPolicy.resolveSubject(subject, owner),
                ),
                ...(relation.grantedBy === undefined ? {} : { grantedBy: relation.grantedBy }),
                ...(relation.concealed === true ? { concealed: true } : {}),
                ...(relation.isRequired === true ? { isRequired: true } : {}),
            };
        }

        // derive relations from the fields permissions read, refusing them on a scope
        const decided = new Set(Object.values(permissions).flatMap(relationsOf));
        for (const [property, field] of Object.entries(definition.fields ?? {})) {
            const name = Casing.kebab(field.relationAt(property));
            const relation =
                decided.has(name) && relations[name] === undefined
                    ? ObjectPolicy.fieldRelation(field)
                    : undefined;
            if (relation !== undefined && definition.isScope === true) {
                throw new TypeError(
                    `scope ${definition.name} reads relation ${name} from a field, so its enclosed objects cannot use it`,
                );
            } else if (relation !== undefined) {
                relations[name] = relation;
            }
        }

        // relate the objects to their scope when a permission reads through it
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
    },

    /** Resolve a declared relation subject: a type's policy, a package subject type, or as is. */
    resolveSubject(
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
        // pass any other subject
        else {
            return subject;
        }
    },

    /** Relate a field to the subjects it refers to, absent for a field referring to none. */
    fieldRelation(field: Field): RelationInput | undefined {
        // relate a principal reference to its kind
        if (field.type === "reference" && field.principals !== undefined) {
            return { subjects: field.principals, grantedBy: null };
        }
        // relate a plain reference to its target type
        else if (field.type === "reference" && field.target) {
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
        // relate an object field to its types, or openly to any type
        else if (field.type === "object") {
            const { source } = field;
            const types = source.builder === "object" ? source.arguments : [];

            return types.length === 0
                ? { subjects: [], grantedBy: null, open: true }
                : {
                      subjects: types.map((type) =>
                          type instanceof ObjectType ? type.policy : type,
                      ),
                      grantedBy: null,
                  };
        }
        // relate nothing else
        else {
            return undefined;
        }
    },

    /** Assemble a type's policy from its declared and represented rules and its traits'. */
    objectPolicy(
        owner: Package,
        definition: ObjectDefinition<string, Readonly<Record<string, Method>>>,
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
                ...ObjectPolicy.representedRelations(represented),
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
            ...(definition.moving === undefined ? {} : { moving: definition.moving }),
            ...(definition.isScope === true || represented?.definition.scope === true
                ? { scope: true }
                : {}),
            ...(definition.partition === true || represented?.definition.partition === true
                ? { partition: true }
                : {}),
            ...(represented?.definition.isGlobal === true ? { isGlobal: true } : {}),
            contributes: policies.flatMap((policy) => policy.contributes ?? []),
            sets: policies.flatMap((policy) => policy.sets ?? []),
        });
    },

    /** Map each declared relation a type's fields have to its column. */
    fieldRelations(type: ObjectType): Record<string, TableMapping["relations"][string]> {
        const relations: Record<string, TableMapping["relations"][string]> = {};
        for (const [property, field] of Object.entries(type.fields)) {
            const name = Casing.kebab(field.relationAt(property));
            const isRelation = Object.hasOwn(type.policy.definition.relations, name);

            // find a subject or object field's subject by its package, type and scope
            if (isRelation && field.reference === "object") {
                const columns = Field.columnsOf(field.relationAt(property));
                relations[name] = {
                    column: property,
                    subject: {
                        packageId: columns.packageId,
                        type: columns.type,
                        scope: columns.scope,
                    },
                };
            }
            // map a principal reference in the universe
            else if (isRelation && field.type === "reference" && field.principals !== undefined) {
                relations[name] = { column: property, scope: Scope.universe.id };
            }
            // map a plain reference by identifier, in the universe, the row's scope or across scopes
            else if (isRelation && field.type === "reference" && field.target) {
                const target = field.target();
                const isBeside = ObjectScope.isBeside(
                    type.definition.scope,
                    target.definition.scope,
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
    },

    /** List the mappings a type's traits add. */
    traitMappings(type: ObjectType) {
        const object = ObjectPolicy.traitObject(type, type.policy.package.id);

        return type.traits.flatMap(({ trait, options }) =>
            trait.mapping === undefined ? [] : [trait.mapping(options, object)],
        );
    },

    /** Read a represented policy's relations as input. */
    representedRelations(represented: Policy | undefined): Record<string, RelationInput> {
        return Object.fromEntries(
            Object.entries(represented?.definition.relations ?? {}).map(([name, relation]) => [
                name,
                // take the whole relation, granted by none without a permission
                { ...relation, grantedBy: relation.grantedBy ?? null },
            ]),
        );
    },

    /** Describe an object type to its traits. */
    traitObject(type: ObjectType, packageId: PackageId): TraitObject {
        return {
            name: type.name,
            identity: type.identity,
            packageId,
            table: () => type.table,
            key: type.keyed,
            storage: type.storage,
            isPrincipal: type.represents !== undefined,
            isUniverseOnly: type.isUniverseOnly,
        };
    },
};

/** What a check decides: a subject's permission on an object (Zanzibar Check). */
export const CheckInput = schema.object({
    /** The subject whose access the check decides. */
    subject: Subject,
    /** The permission of the object's type. */
    permission: AccessName,
    /** The object, with access decided in the call's scope. */
    object: ObjectReference,
});

/** The answer of a check. */
export const CheckResult = schema.object({
    /** Whether the subject has the permission on the object. */
    isAllowed: schema.boolean(),
});
