import * as access from "@destack/access";
import { CallError } from "@destack/runtime/error";
import {
    AccessContext,
    type AccessExpression,
    Authorizer,
    Caller,
    Policy,
    principal,
    type RelationInput,
    relationsOf,
} from "@destack/access";
import type { Package, PackageId } from "@destack/package";
import { Field } from "../field/field.ts";
import {
    GrantInput,
    GrantShape,
    InvitationShape,
    InviteInput,
    InviteShape,
    RelationshipShape,
    Roles,
} from "../trait/shareable.ts";
import { Page } from "../method/page.ts";
import { Empty, ListPage, PageShape } from "../method/procedure.ts";
import type { TraitObject, TraitPolicy } from "../trait/trait.ts";
import { Casing } from "./name.ts";
import { AccessName, ObjectReference, Scope, Subject } from "@destack/sync";
import { TABLE, type Select, type Table } from "@destack/db";
import {
    ACCESS_MAPPINGS,
    accessGrant,
    accessInvitation,
    GrantRequest,
    INVITATION_PAGE,
    accessRelationship,
    accessRole,
    RoleRequest,
    type TableMapping,
} from "@destack/access";
import { present, schema } from "@destack/schema";
import { type Call } from "../method/call.ts";
import { defineMethod, type Method } from "../method/method.ts";
import type { MethodBuilder } from "../method/record.ts";
import { INTRINSIC } from "./intrinsic.ts";
import {
    ObjectPermissions,
    ObjectType,
    SCOPE,
    type ObjectDefinition,
    type ObjectRelationInput,
    type ObjectScope,
} from "./object.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

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
        const relations = ObjectAccess.declaredRelations(type, definition, owner, permissions);

        // collect the traits' policies and the permissions they derive
        const names = Object.keys(permissions);
        const object = ObjectAccess.traitObject(type, (definition.represents?.package ?? owner).id);
        const policies = type.traits.flatMap(({ trait, options }) =>
            trait.policy === undefined ? [] : [trait.policy(options, object, names)],
        );
        const derived = Object.fromEntries(
            policies.flatMap((policy) => Object.entries(policy.permissions ?? {})),
        );

        // register the policy over declared and derived permissions
        const policy = ObjectAccess.objectPolicy(owner, definition, {
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

        // reuse an intrinsic mapping
        if (type.intrinsic !== undefined) {
            return { ...type.intrinsic, policy: type.policy, ...inherited };
        }

        // map each declared relation to its field and add trait mappings
        const relations = ObjectAccess.fieldRelations(type);
        const placed: Partial<Pick<TableMapping, "parent" | "trees">> = {};
        for (const { relations: traitRelations, ...rest } of ObjectAccess.traitMappings(type)) {
            Object.assign(relations, traitRelations);
            Object.assign(placed, rest);
        }

        // map the self relation to the principal each object stands for under its own identifier, in the universe for a global principal
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
            scope: "scope",
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

    /** Build the authorizer of object types, their enclosing scope types, and related types. */
    authorizer(
        objects: readonly ObjectType[],
        others: readonly (Policy | ObjectType | TableMapping)[],
        replicates: (table: Table) => boolean,
        partition?: string,
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
            { replicates, ...(partition === undefined ? {} : { partition }) },
        );
    },

    /** Decide a subject's permission on an object the call's scope decides. */
    async check(
        call: Call,
        { authorization }: StoreExecution,
    ): Promise<schema.Infer<typeof CheckResult>> {
        // require an object the call's scope decides, and a permission its type declares
        const { subject, permission, object } = CheckInput.parse(call.input);
        const { authorizer, snapshot } = authorization;
        const policy = authorizer.policy(object);
        const scope = authorizer.governingScope(object);
        if (scope !== call.scope) {
            throw new CallError("BAD_REQUEST", {
                message: `${object.type} ${object.id} is decided in ${scope}, not ${call.scope}`,
            });
        } else if (!Object.hasOwn(policy.definition.permissions, permission)) {
            throw new CallError("BAD_REQUEST", {
                message: `${object.type} has no permission ${permission}`,
            });
        }
        const asked = { packageId: object.packageId, type: object.type, name: permission };
        const asking = await authorization.in(scope);

        // decide the caller's own access under its delegates and grant
        const own = call.caller.subject;
        if (own !== undefined && Subject.same(own, subject)) {
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
            throw new CallError("FORBIDDEN", {
                message: `checking another subject's access to ${object.type} needs ${gate ?? "a grant permission"}`,
            });
        }
        const checked = await authorizer.resolve(
            snapshot,
            scope,
            AccessContext.of(Caller.of(subject), call.now),
        );

        return { isAllowed: (await authorizer.check(snapshot, asked, object, checked)).isAllowed };
    },

    /** Find the scope object of the call's scope. */
    scopeObject(call: Call, execution: Execution): Promise<ObjectReference> {
        return Scope.object(Execution.snapshot(execution), call.scope);
    },

    /** Read one role row as the method's result. */
    async readRow(
        execution: StoreExecution,
        id: Select<typeof accessRole>["id"],
    ): Promise<Select<typeof accessRole>> {
        return present(
            await execution.transaction.read(accessRole).findFirst({ where: { id } }),
            `role ${id}`,
        );
    },

    /** Read how access maps one of its own tables. */
    readMapping(table: Table): TableMapping {
        return present(
            ACCESS_MAPPINGS.find((mapping) => mapping.table === table),
            `the access mapping of ${table[TABLE].name}`,
        );
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
                    ObjectAccess.resolveSubject(subject, owner),
                ),
                ...(relation.grantedBy === undefined ? {} : { grantedBy: relation.grantedBy }),
                ...(relation.concealed === true ? { concealed: true } : {}),
            };
        }

        // derive relations from the fields permissions read, refusing them on a scope
        const decided = new Set(Object.values(permissions).flatMap(relationsOf));
        for (const [property, field] of Object.entries(definition.fields ?? {})) {
            const name = Casing.kebab(field.relationAt(property));
            const relation =
                decided.has(name) && relations[name] === undefined
                    ? ObjectAccess.fieldRelation(field)
                    : undefined;
            if (relation !== undefined && definition.isScope === true) {
                throw new TypeError(
                    `scope ${definition.name} reads relation ${name} from a field: its enclosed objects cannot use it`,
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
        // keep any other subject
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
                ...ObjectAccess.representedRelations(represented),
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
            ...(definition.partition === true ? { partition: true } : {}),
            ...(represented?.definition.isGlobal === true ? { isGlobal: true } : {}),
            contributes: policies.flatMap((policy) => policy.contributes ?? []),
        });
    },

    /** Map each declared relation a type's fields keep to its column. */
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
                const isBeside = type.scopes.some((scope) =>
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
    },

    /** List the mappings a type's traits add. */
    traitMappings(type: ObjectType) {
        const object = ObjectAccess.traitObject(type, type.policy.package.id);

        return type.traits.flatMap(({ trait, options }) =>
            trait.mapping === undefined ? [] : [trait.mapping(options, object)],
        );
    },

    /** Read a represented policy's relations as input. */
    representedRelations(represented: Policy | undefined): Record<string, RelationInput> {
        return Object.fromEntries(
            Object.entries(represented?.definition.relations ?? {}).map(([name, relation]) => [
                name,
                // keep the whole relation, granted by none without a permission
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
        };
    },
};

/** A scope type's roles as objects. */
export const role = {
    /** Define a scope type's roles over the access role table. */
    definition<const Scope extends ObjectScope | readonly ObjectScope[]>(scope: Scope) {
        return {
            name: "role",
            plural: "roles",
            [INTRINSIC]: { table: accessRole, mapping: ObjectAccess.readMapping(accessRole) },
            scope,
            represents: access.role,
            permissions: access.role.definition.permissions,
        } as const;
    },

    /** Declare the roles' methods, which change roles through the access role API. */
    methods: (method: MethodBuilder<typeof accessRole>) => ({
        create: Execution.handle(
            method.create("create", {
                fields: ["name", "description"],
                input: RoleRequest.pick({ permissions: true }),
                isPredicted: false,
            }),
            async (call, execution) => {
                // create the role through the access role API
                const { name, description, permissions } = call.input;
                const store = Execution.store(execution, call);
                const created = await store.authorization.createRole(
                    await ObjectAccess.scopeObject(call, execution),
                    { name, description, permissions },
                );

                return ObjectAccess.readRow(store, created.id);
            },
        ),
        update: Execution.handle(
            method.update("update", {
                fields: ["name", "description"],
                input: schema.object({
                    permissions: RoleRequest.shape.permissions.exactOptional(),
                }),
                isPredicted: false,
            }),
            async (call, execution) => {
                // change the given values at the loaded revision
                const { name, description, permissions } = call.input;
                const store = Execution.store(execution, call);
                await store.authorization.updateRole(
                    await ObjectAccess.scopeObject(call, execution),
                    call.target.id,
                    {
                        ...(name === undefined ? {} : { name }),
                        ...(description === undefined ? {} : { description }),
                        ...(permissions === undefined ? {} : { permissions }),
                        revision: call.target.revision,
                    },
                );

                return ObjectAccess.readRow(store, call.target.id);
            },
        ),
        keep: defineMethod<{
            kind: "custom";
            permission: "update";
            input: typeof RoleBinding;
            output: typeof Empty;
            mutates: true;
            target: "scope";
            table: typeof accessRole;
        }>({
            kind: "custom",
            permission: "update",
            input: RoleBinding,
            output: Empty,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "/keep" },
                input: shapes.scope.extend({ ...shapes.replay, ...RoleBinding.shape }),
                output: Empty,
            }),
            handler: async (call, execution) => {
                // keep the role and its binding under the manager, dropping both without permissions
                const { manager, ...binding } = RoleBinding.parse(call.input);
                await Execution.store(execution, call)
                    .authorization.applying(manager)
                    .keepBinding(await ObjectAccess.scopeObject(call, execution), binding);

                return {};
            },
        }),
        delete: Execution.handle(
            method.delete("delete", { isPredicted: false }),
            async (call, execution) => {
                // delete the unbound role at the loaded revision
                await Execution.store(execution, call).authorization.deleteRole(
                    await ObjectAccess.scopeObject(call, execution),
                    call.target.id,
                    call.target.revision,
                );

                return {};
            },
        ),
    }),
};

/** A role kept by name under a manager, granting exactly some permissions to one subject. */
const RoleBinding = RoleRequest.extend({
    /** The one subject the role is bound to. */
    subject: Subject,
    /** The object in the scope the role is bound on, the scope itself when absent. */
    object: ObjectReference.exactOptional(),
    /** What a request must meet for the binding to apply. */
    conditions: access.RelationshipCondition.exactOptional(),
    /** The stack declaration managing the role and its binding. */
    manager: access.Manager.schema,
});

/** A relationship to create: its object, and its subject through one relation or role. */
const RelationshipInput = schema.object({ object: ObjectReference, ...GrantShape });

/** One relationship of an object, by its identifier. */
const RelationshipKey = schema.object({ object: ObjectReference, ...RelationshipShape });

/** An invitation to create: its object, the relationship it offers, and why. */
const InvitationInput = schema.object({ object: ObjectReference, ...InviteShape });

/** One invitation to an object, by its identifier. */
const InvitationKey = schema.object({ object: ObjectReference, ...InvitationShape });

/** A page of an object's pending invitations, at most access's page. */
const InvitationListing = schema.object({
    object: ObjectReference,
    ...PageShape,
    limit: schema.number().int().min(1).max(INVITATION_PAGE).exactOptional(),
});

/** A scope type's relationships as objects, created and deleted under the grant rules. */
export const relationship = {
    /** Define a scope type's relationships over the access relationship table. */
    definition<const Scope extends ObjectScope | readonly ObjectScope[]>(scope: Scope) {
        return {
            name: "relationship",
            plural: "relationships",
            [INTRINSIC]: {
                table: accessRelationship,
                mapping: ObjectAccess.readMapping(accessRelationship),
            },
            scope,
            represents: access.relationship,
            permissions: access.relationship.definition.permissions,
        } as const;
    },

    /** Declare the relationships' methods: decide a subject's permission on an object. */
    // decide in the handler, which asks for other subjects too, instead of a scoped query's stub check
    methods: (_method: MethodBuilder<typeof accessRelationship>) => ({
        check: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof CheckInput;
            output: typeof CheckResult;
            mutates: false;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "read",
            input: CheckInput,
            output: CheckResult,
            mutates: false,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "/check" },
                input: shapes.scope.extend(CheckInput.shape),
                output: CheckResult,
            }),
            handler: (call, execution) =>
                ObjectAccess.check(call, Execution.store(execution, call)),
        }),
        create: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof RelationshipInput;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "grant",
            input: RelationshipInput,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "" },
                input: shapes.scope.extend({ ...shapes.replay, ...RelationshipInput.shape }),
                output: access.Relationship.schema,
            }),
            handler: (call, execution) => {
                // grant under the relation's grant rules
                const { object, ...granted } = RelationshipInput.parse(call.input);

                return Execution.store(execution, call).authorization.grant({
                    ...GrantInput.parse(granted),
                    object,
                });
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof RelationshipKey;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessRelationship;
        }>({
            kind: "custom",
            permission: "grant",
            input: RelationshipKey,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "DELETE", path: "/{relationshipId}" },
                input: shapes.scope.extend({ ...shapes.replay, ...RelationshipKey.shape }),
                output: access.Relationship.schema,
            }),
            handler: (call, execution) => {
                // revoke under the relation's grant rules
                const { object, relationshipId } = RelationshipKey.parse(call.input);

                return Execution.store(execution, call).authorization.revoke(
                    object,
                    relationshipId,
                );
            },
        }),
    }),
};

/** A scope type's invitations as objects, answered through sharing methods. */
export const invitation = {
    /** Define a scope type's invitations over the access invitation table. */
    definition<const Scope extends ObjectScope | readonly ObjectScope[]>(scope: Scope) {
        return {
            name: "invitation",
            plural: "invitations",
            [INTRINSIC]: {
                table: accessInvitation,
                mapping: ObjectAccess.readMapping(accessInvitation),
            },
            scope,
            represents: access.invitation,
            permissions: access.invitation.definition.permissions,
        } as const;
    },

    /** Declare the invitations' methods: invite, accept, withdraw and list. */
    methods: (_method: MethodBuilder<typeof accessInvitation>) => ({
        create: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof InvitationInput;
            output: typeof access.Invitation.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "grant",
            input: InvitationInput,
            output: access.Invitation.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationInput.shape }),
                output: access.Invitation.schema,
            }),
            handler: (call, execution) => {
                // invite to the relationship on the object
                const { object, ...invited } = InvitationInput.parse(call.input);
                const { relationship: offered, ...details } = InviteInput.parse(invited);

                return Execution.store(execution, call).authorization.invite({
                    ...details,
                    relationship: { ...offered, object },
                });
            },
        }),
        accept: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof InvitationKey;
            output: typeof access.Relationship.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "read",
            input: InvitationKey,
            output: access.Relationship.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "/{invitationId}/accept" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationKey.shape }),
                output: access.Relationship.schema,
            }),
            handler: (call, execution) => {
                // accept as the caller
                const { object, invitationId } = InvitationKey.parse(call.input);

                return Execution.store(execution, call).authorization.accept(object, invitationId);
            },
        }),
        delete: defineMethod<{
            kind: "custom";
            permission: "read";
            input: typeof InvitationKey;
            output: typeof access.Invitation.schema;
            mutates: true;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "read",
            input: InvitationKey,
            output: access.Invitation.schema,
            mutates: true,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "DELETE", path: "/{invitationId}" },
                input: shapes.scope.extend({ ...shapes.replay, ...InvitationKey.shape }),
                output: access.Invitation.schema,
            }),
            handler: (call, execution) => {
                // withdraw as its inviter, its addressee or a granter
                const { object, invitationId } = InvitationKey.parse(call.input);

                return Execution.store(execution, call).authorization.withdraw(
                    object,
                    invitationId,
                );
            },
        }),
        list: defineMethod<{
            kind: "custom";
            permission: "grant";
            input: typeof InvitationListing;
            output: ReturnType<typeof ListPage.schema<typeof access.Invitation.schema>>;
            mutates: false;
            target: "scope";
            table: typeof accessInvitation;
        }>({
            kind: "custom",
            permission: "grant",
            input: InvitationListing,
            output: ListPage.schema(access.Invitation.schema),
            mutates: false,
            target: "scope",
            result: "value",
            isPredicted: false,
            procedure: (_name, shapes) => ({
                route: { method: "GET", path: "" },
                input: shapes.scope.extend(InvitationListing.shape),
                output: ListPage.schema(access.Invitation.schema),
            }),
            handler: async (call, execution) => {
                // page the object's pending invitations by identifier
                const { object, ...paging } = InvitationListing.parse(call.input);
                const listing = new Page(
                    paging,
                    [call.object.name, call.scope, object.type, object.id],
                    schema.string(),
                );
                const { authorization } = Execution.store(execution, call);
                const invitations = await authorization.invitations(
                    { object },
                    {
                        ...(listing.after === undefined ? {} : { after: listing.after }),
                        limit: listing.limit,
                    },
                );

                return listing.result(invitations, (each) => each.id);
            },
        }),
    }),
};

/** A scope type's grants as objects: what its principal lets delegates do for it. */
export const grant = {
    /** Define a scope type's grants over the access grant table. */
    definition<const Scope extends ObjectScope | readonly ObjectScope[]>(scope: Scope) {
        return {
            name: "grant",
            plural: "grants",
            [INTRINSIC]: { table: accessGrant, mapping: ObjectAccess.readMapping(accessGrant) },
            scope,
            represents: access.grant,
            permissions: access.grant.definition.permissions,
        } as const;
    },

    /** Declare the grants' methods, which create and revoke grants through the access grant API. */
    methods: (method: MethodBuilder<typeof accessGrant>) => ({
        create: Execution.handle(
            method.create("create", { fields: [], input: GrantRequest, isPredicted: false }),
            async (call, execution) =>
                Execution.store(execution, call).authorization.createGrant(
                    await ObjectAccess.scopeObject(call, execution),
                    call.input,
                ),
        ),
        revoke: Execution.handle(
            method.delete("revoke", { isPredicted: false }),
            async (call, execution) => {
                // refuse a personal access token's grant: revoking the token revokes it
                const { id } = call.target;
                const delegate = ObjectReference.at("delegate", call.target);
                if (access.principal.personalAccessToken.is(delegate)) {
                    throw new CallError("CONFLICT", {
                        message: `grant ${id} belongs to personal access token ${delegate.id}: revoke the token`,
                    });
                }

                // revoke the grant as its delegator
                await Execution.store(execution, call).authorization.revokeGrant(
                    await ObjectAccess.scopeObject(call, execution),
                    id,
                );

                return {};
            },
        ),
    }),
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
