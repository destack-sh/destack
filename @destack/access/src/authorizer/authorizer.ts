import { v7 } from "uuid";
import {
    and,
    eq,
    inArray,
    sql,
    type DatabaseConnection,
    type Transaction,
    type SQL,
    type Table,
    Condition,
    Placeholder,
    Predicate,
    Snapshot,
    TABLE,
    type Row,
    combine,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { schema, zip } from "@destack/schema";
import {
    Replica,
    Scope,
    type Subscription,
    type ScopeLink,
    Watch,
    ObjectReference,
    ObjectTypeReference,
    Subject,
} from "@destack/sync";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { relationsOf, PermissionReference, type Policy } from "../declare/policy.ts";
import type {
    AccessExpression,
    AttributeType,
    ConditionExpression,
} from "../declare/expression.ts";
import { accepts, type RelationDescription, type SubjectType } from "../declare/subject.ts";
import { ACCESS_PACKAGE_ID, anyone, INTRINSIC_POLICIES } from "../declare/principal.ts";
import { ACCESS_MAPPINGS } from "../replica/replica.ts";
import { type AccessContext } from "../context/context.ts";
import { HIGHEST_ASSURANCE, type Elevation, type StepUp } from "../context/elevation.ts";
import { accessRelationship, type RelationshipRow } from "../relationship/table.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { Creation } from "./authorization.ts";
import { accessRole } from "../role/table.ts";
import { accessTables, CHAIN_SHAPE, decisionTables } from "../replica/replica.ts";
import { earliest, Access } from "./access.ts";
import { Compiler } from "./compiler.ts";
import { TupleCondition } from "./condition.ts";
import type { Decision, Explanation } from "./decision.ts";
import { TupleReader, TupleTree, type Tuple, type Lookup } from "./tuple.ts";
import { TableMapping, type FieldRelation } from "./mapping.ts";

/** The identifier of a wildcard subject, matching every identity of its type. */
const WILDCARD = "*";

/** How long an unheard access replica lasts before refusal: three 10-second beats, in ms. */
const LAG_MILLISECONDS = 30_000;

/** The name of the shape replicating the universe's rows a scope reads. */
export const UNIVERSE_SHAPE = "universe";

/** The parameters of a chain replica. */
export const ChainParameters = schema.object({
    /** The object types whose access rows live in the follower's own database. */
    local: schema.array(ObjectTypeReference),
    /** The object types the follower replicates. */
    replicated: schema.array(ObjectTypeReference),
    /** The scopes an outside follower replicates, with the scope above and those between. */
    via: schema.array(schema.string().min(1)),
    /** The object types whose universe relationships the follower's decisions read. */
    types: schema.array(ObjectTypeReference),
    /** The scopes between the replicated ones and the scope above, as the follower knows them. */
    between: schema.array(schema.string().min(1)),
});
/** The parameters of a chain replica. */
export type ChainParameters = schema.Infer<typeof ChainParameters>;

/** The parameters of a universe replica: each object type's rows, decided for their reader. */
export const UniverseParameters = schema.object({
    /** The rows of each object type. */
    rows: schema.array(
        schema.object({
            /** The object type. */
            type: ObjectTypeReference,
            /** The rows replicated. */
            where: Condition.schema,
            /** The requested types whose rows these rows scope, absent for the replicated scope. */
            within: schema.array(ObjectTypeReference).min(1).exactOptional(),
            /** Whether rows are replicated from their own scope instead of the replicated scope. */
            isEverywhere: schema.literal(true).exactOptional(),
        }),
    ),
});
/** The parameters of a replica of the universe's rows. */
export type UniverseParameters = schema.Infer<typeof UniverseParameters>;

/** A table a chain replica reads: its row condition, absent for all rows, and its scopes. */
type ChainTable = [Table, Condition | undefined, readonly string[]];

/** A caller's permitted rows of a set by position, and the next time they change. */
export interface Admission {
    /** The positions of the rows the caller has the permission on. */
    readonly permitted: ReadonlySet<number>;
    /** The next time the permitted rows may change. */
    readonly until?: number;
}

/** One scope type on a permission's scope chain and the relations granting the permission there. */
export interface ChainStep {
    /** The scope type. */
    readonly type: Pick<PermissionReference, "packageId" | "type">;
    /** The relations whose relationships on its object grant the permission. */
    readonly relations: readonly string[];
    /** The containment set the permission admits, limited to some principal types when given. */
    readonly contained?: { readonly principals?: readonly ObjectTypeReference[] };
}

/** The chain a scope's replicas cover: a home's above it, or a follower's from itself. */
export type ChainOptions =
    | {
          /** A home, replicating the chain above it. */
          readonly kind: "home";
          /** The scope the home lives in, whose chain it replicates before it is born. */
          readonly parent: string;
      }
    | {
          /** A follower, replicating its chain from itself. */
          readonly kind: "follower";
      };

/** Decide policies over their objects' tables, in SQL or in memory from grants. */
export class Authorizer {
    /** The permission keys owner roles leave out, granted only by expressions and listing roles. */
    readonly reserved: ReadonlySet<string>;
    /** The authentication each elevated permission asks for, by permission key. */
    readonly elevated: ReadonlyMap<string, Elevation>;
    /** The keys of permissions that stay available while a scope is suspended. */
    readonly administration: ReadonlySet<string>;
    /** The keys of permissions that stay available while a scope moves. */
    readonly moving: ReadonlySet<string>;
    /** Field relations that another relation uses as a subject set. */
    readonly fields: FieldRelation[];
    /** The subject sets that relations accept and roles may bind to. */
    readonly memberships: SubjectType[];
    /** The permissions accepted as subject sets, by the type and relation of each relationship. */
    readonly #implied: Map<string, string[]>;
    /** The permissions accepted as sets on enclosed scopes, by enclosing type and permission. */
    readonly #enclosed: Map<string, PermissionReference[]>;
    /** The scope types that take subject sets from the scopes enclosing them. */
    readonly #enclosedTypes: Set<string>;
    /** The scope chain steps of permissions read through enclosing scopes, by permission key. */
    readonly #inherited: Map<string, readonly ChainStep[]>;
    /** How long an access replica may go without its home before refusal, in milliseconds. */
    readonly lag: number;
    /** The one partition whose scopes' access this database keeps, every partition when absent. */
    readonly partition: string | undefined;
    /** The object types whose access rows live in this database, rather than as replicas. */
    readonly local: readonly ObjectTypeReference[];
    /** The object types replicated from scopes above: inherited rows and kept scope types. */
    readonly replicated: readonly ObjectTypeReference[];
    /** The replicated object types whose rows live in a scope. */
    readonly scoped: readonly ObjectTypeReference[];
    /** The global principal types, outside scope chains. */
    readonly global: readonly ObjectTypeReference[];
    /** The policies indexed by package and type. */
    readonly #policies: Map<string, Policy>;
    /** The subject types other types contribute to each open relation, by type and relation. */
    readonly #contributions: Map<string, SubjectType[]>;
    /** The table mappings indexed by package and type. */
    readonly #mappings: Map<string, TableMapping>;
    /** The compiler of the registered policies to SQL. */
    readonly #compiler: Compiler;

    /** Validate the policies, their referenced policies and their tables in this database. */
    constructor(
        policies: readonly Policy[],
        mappings: readonly TableMapping[] = [],
        options: {
            /** How long an access replica may go without hearing from its home, in milliseconds. */
            readonly lag?: number;
            /** Decide whether the database replicates a table's rows from their home. */
            readonly replicates?: (table: Table) => boolean;
            /** The one partition whose scopes' access the database keeps, every one when absent. */
            readonly partition?: string;
        } = {},
    ) {
        // start with empty state
        this.fields = [];
        this.memberships = [];
        this.#implied = new Map<string, string[]>();
        this.#enclosed = new Map<string, PermissionReference[]>();
        this.#enclosedTypes = new Set<string>();
        this.#inherited = new Map<string, readonly ChainStep[]>();

        // start the rest of the state empty
        this.#policies = new Map<string, Policy>();
        this.#contributions = new Map<string, SubjectType[]>();
        this.#mappings = new Map<string, TableMapping>();

        // register every policy the declared policies reference, by type
        const types = Authorizer.#collectPolicies(policies);
        for (const type of types) {
            this.#policies.set(Authorizer.#policyKey(type), type);
        }

        // add contributing types to open relations and validate every referenced permission
        for (const type of types) {
            this.#registerContributions(type);
        }
        for (const type of types) {
            this.#validatePermissions(type);
        }

        // index the permissions the policies list as reserved, elevated, administrative or moving
        this.reserved = Authorizer.#listedPermissions(types, "reserved");
        this.elevated = Authorizer.#elevatedPermissions(types);
        this.administration = Authorizer.#listedPermissions(types, "administration");
        this.moving = Authorizer.#listedPermissions(types, "moving");
        this.lag = options.lag ?? LAG_MILLISECONDS;
        this.partition = options.partition;
        this.#compiler = new Compiler(this);

        // register the declared mappings and the mappings every database has
        for (const source of [...mappings, ...this.#implicitMappings(mappings)]) {
            this.#registerMapping(source);
        }
        for (const type of types) {
            this.#validateScopeRelations(type);
        }

        // list the types whose access rows this database keeps, and the types it keeps replicas of
        const replicates = options.replicates ?? (() => false);
        this.local = this.#localTypes(replicates);
        this.replicated = this.#replicatedTypes(replicates);
        this.scoped = this.#scopedTypes(replicates);
        this.global = this.#globalTypes();

        // expand the subject sets roles may bind to
        const sets = this.#subjectSets();
        this.#expandMemberships(sets);
        this.#registerFieldSets(sets);

        // require every declared relation of a mapped type to decide a permission
        this.#requireDeciding(types, sets);
    }

    /** Add a contributing type to each open relation it references. */
    #registerContributions(type: Policy): void {
        for (const entry of type.definition.contributes ?? []) {
            // refuse a contribution to a relation that is not open
            const target = this.#policies.get(ObjectTypeReference.key(entry));
            if (target?.definition.relations[entry.relation]?.open !== true) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `${type.name} cannot contribute to ${entry.type}.${entry.relation}: it is not an open relation`,
                );
            }

            // add the contributing type to the relation's subject types
            const key = PermissionReference.key({
                packageId: entry.packageId,
                type: entry.type,
                name: entry.relation,
            });
            const contributed = this.#contributions.get(key) ?? [];
            contributed.push({
                packageId: type.definition.packageId,
                type: type.definition.name,
            });
            this.#contributions.set(key, contributed);
        }
    }

    /** Validate every permission of a policy, including unused reserved and elevated ones. */
    #validatePermissions(type: Policy): void {
        // resolve each relation's grant permission and subject types
        const definition = type.definition;
        for (const relation of Object.values(definition.relations)) {
            if (relation.grantedBy !== undefined) {
                this.expression(type.permission(relation.grantedBy));
            }
            for (const subject of relation.subjects) {
                this.#validateSubject(subject);
            }
        }

        // resolve the type's grant, reserved, elevated, administration and moving permissions
        if (definition.grantedBy !== undefined) {
            this.expression(type.permission(definition.grantedBy));
        }
        for (const name of [
            ...(definition.reserved ?? []),
            ...Object.keys(definition.elevated ?? {}),
            ...(definition.administration ?? []),
            ...(definition.moving ?? []),
        ]) {
            this.expression(type.permission(name));
        }

        // validate named permissions after resolving the declared relations
        for (const name of Object.keys(definition.permissions)) {
            this.#validate(type.permission(name), new Set());
        }
    }

    /** List every database's mappings: scope types onto ancestry, access types onto tables. */
    #implicitMappings(mappings: readonly TableMapping[]): TableMapping[] {
        // map scope types this database has no table of onto the ancestry every database has
        const scopes = this.policies()
            .filter(
                (policy) =>
                    policy.definition.scope === true &&
                    !mappings.some((mapping) => mapping.policy === policy),
            )
            .map((policy) => TableMapping.ancestry(policy));

        // map access's types onto its tables where no declared type represents them
        const unrepresented = ACCESS_MAPPINGS.filter(
            (mapping) =>
                this.policy({
                    packageId: mapping.policy.definition.packageId,
                    type: mapping.policy.definition.name,
                }) === mapping.policy &&
                !mappings.some((declared) => declared.table === mapping.table),
        );

        return [...scopes, ...unrepresented];
    }

    /** Register a mapping once and validate its columns against its policy. */
    #registerMapping(source: TableMapping): void {
        // require the mapping to reference the registered policy
        const mapping = TableMapping.freeze(source);
        const definition = mapping.policy.definition;
        if (
            this.policy({
                packageId: definition.packageId,
                type: definition.name,
            }) !== mapping.policy
        ) {
            throw new AccessError(
                "INVALID_DECLARATION",
                "mapping must reference the registered object declaration",
            );
        }

        // refuse a second mapping of the same type
        const key = ObjectTypeReference.key({
            packageId: definition.packageId,
            type: definition.name,
        });
        if (this.#mappings.has(key)) {
            throw new AccessError("INVALID_DECLARATION", `duplicate database mapping: ${key}`);
        }

        // validate the columns and the compiled expressions before registering
        TableMapping.validate(this, mapping);
        this.#requireCompilable(mapping);
        this.#mappings.set(key, mapping);
    }

    /** Require each relation to an enclosing scope to accept scope types and map to no field. */
    #validateScopeRelations(type: Policy): void {
        for (const [name, relation] of Object.entries(type.definition.relations)) {
            if (relation.isScope === true) {
                this.#validateScopeRelation(type, name, relation);
            }
        }
    }

    /** List the mapped types whose access rows live in this database. */
    #localTypes(replicates: (table: Table) => boolean): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) => !accessTables.includes(mapping.table) && !replicates(mapping.table),
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** List the mapped types this database replicates: inherited and replicated scope types. */
    #replicatedTypes(replicates: (table: Table) => boolean): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    mapping.inherited !== undefined ||
                    (replicates(mapping.table) &&
                        mapping.policy.definition.scope === true &&
                        mapping.table !== Scope.table),
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** List the replicated types whose rows live in a scope, beside inherited and scope types. */
    #scopedTypes(replicates: (table: Table) => boolean): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    replicates(mapping.table) &&
                    mapping.inherited === undefined &&
                    mapping.policy.definition.scope !== true &&
                    mapping.scope !== undefined,
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** List the types of global principals. */
    #globalTypes(): ObjectTypeReference[] {
        return [...this.#policies.values()]
            .filter((policy) => policy.definition.isGlobal === true)
            .map((policy) => ({
                packageId: policy.definition.packageId,
                type: policy.definition.name,
            }));
    }

    /** Collect the subject sets the declared relations accept, by key. */
    #subjectSets(): Map<string, PermissionReference> {
        return new Map(
            this.policies()
                .flatMap((policy) => Object.values(policy.definition.relations))
                .flatMap((relation) => relation.subjects)
                .flatMap((subject): [string, PermissionReference][] => {
                    // keep the subject sets
                    if (subject.relation === undefined) {
                        return [];
                    }
                    const set = {
                        packageId: subject.packageId,
                        type: subject.type,
                        name: subject.relation,
                    };

                    return [[PermissionReference.key(set), set]];
                }),
        );
    }

    /** Expand the declared sets and every scope's relations into the memberships roles bind to. */
    #expandMemberships(sets: ReadonlyMap<string, PermissionReference>): void {
        // add every relation of each scope type to the declared sets
        const expanded = new Map(sets);
        for (const policy of this.policies().filter((each) => each.definition.scope === true)) {
            for (const relation of Object.keys(policy.definition.relations)) {
                const set = {
                    packageId: policy.definition.packageId,
                    type: policy.definition.name,
                    name: relation,
                };
                expanded.set(PermissionReference.key(set), set);
            }
        }

        // expand each set, including the enclosing sets the expansion adds
        const expandedMembers = new Map<string, PermissionReference>();
        for (const [key, set] of expanded) {
            // expand a relation's set as it is
            if (Object.hasOwn(this.policy(set).definition.relations, set.name)) {
                expandedMembers.set(key, set);
            }
            // expand a permission's set through the relations deciding it
            else {
                this.#expandPermissionSet(set, expanded, expandedMembers);
            }
        }

        // record each member as a membership
        for (const member of expandedMembers.values()) {
            this.memberships.push({
                packageId: member.packageId,
                type: member.type,
                relation: member.name,
            });
        }
    }

    /** Expand a permission's set through its deciding relations and the enclosing scopes' sets. */
    #expandPermissionSet(
        set: PermissionReference,
        expanded: Map<string, PermissionReference>,
        expandedMembers: Map<string, PermissionReference>,
    ): void {
        // expand through the relations deciding the permission on the object
        const { relations, enclosing } = this.deciding(this.policy(set).permission(set.name));
        for (const relation of relations) {
            const deciding = { packageId: set.packageId, type: set.type, name: relation };
            const decidingKey = PermissionReference.key(deciding);
            expandedMembers.set(decidingKey, deciding);
            this.#implied.set(decidingKey, [...(this.#implied.get(decidingKey) ?? []), set.name]);
        }

        // and through the same sets of the scopes enclosing it, expanded in turn
        for (const scope of enclosing) {
            const scopeKey = PermissionReference.key(scope);
            this.#enclosed.set(scopeKey, [...(this.#enclosed.get(scopeKey) ?? []), set]);
            this.#enclosedTypes.add(ObjectTypeReference.key(set));
            expanded.set(scopeKey, scope);
        }
    }

    /** Expand subject sets through the fields that store them. */
    #registerFieldSets(sets: ReadonlyMap<string, PermissionReference>): void {
        for (const mapping of this.#mappings.values()) {
            const definition = mapping.policy.definition;
            for (const [relation, field] of Object.entries(mapping.relations)) {
                const key = PermissionReference.key({
                    packageId: definition.packageId,
                    type: definition.name,
                    name: relation,
                });
                if (sets.has(key)) {
                    Authorizer.#requireSubjectIndex(mapping, relation, field);
                    this.fields.push({
                        mapping,
                        relation,
                        field,
                        subject: this.onlySubject(mapping.policy, relation),
                    });
                }
            }
        }
    }

    /** Require every declared relation of a mapped type to decide a permission. */
    #requireDeciding(
        types: readonly Policy[],
        sets: ReadonlyMap<string, PermissionReference>,
    ): void {
        const parents = this.#parentRelations();
        const intrinsic = new Set(
            INTRINSIC_POLICIES.map((policy) => Authorizer.#policyKey(policy)),
        );
        for (const type of types.filter(
            (policy) =>
                !intrinsic.has(Authorizer.#policyKey(policy)) &&
                this.#mappings.has(Authorizer.#policyKey(policy)),
        )) {
            const definition = type.definition;
            const decided = new Set(Object.values(definition.permissions).flatMap(relationsOf));
            for (const name of Object.keys(definition.relations)) {
                const key = PermissionReference.key({
                    packageId: definition.packageId,
                    type: definition.name,
                    name,
                });
                if (!decided.has(name) && !sets.has(key) && !parents.has(key)) {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        `no permission uses relation ${definition.name}.${name}`,
                    );
                }
            }
        }
    }

    /** Collect the keys of the mapped relations to an owning parent. */
    #parentRelations(): Set<string> {
        return new Set(
            [...this.#mappings.values()].flatMap((mapping) =>
                mapping.parent === undefined
                    ? []
                    : [
                          PermissionReference.key({
                              packageId: mapping.policy.definition.packageId,
                              type: mapping.policy.name,
                              name: mapping.parent,
                          }),
                      ],
            ),
        );
    }

    /** Resolve the policy of a package-qualified object type. */
    policy(reference: Pick<ObjectReference, "packageId" | "type">): Policy {
        const type = this.#policies.get(ObjectTypeReference.key(reference));
        if (!type) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `unknown object type: ${reference.packageId}/${reference.type}`,
            );
        }

        return type;
    }

    /** Find a registered type's policy, absent for a type the authorizer does not know. */
    find(reference: Pick<ObjectReference, "packageId" | "type">): Policy | undefined {
        return this.#policies.get(ObjectTypeReference.key(reference));
    }

    /** Return the policies in registration order. */
    policies(): Policy[] {
        return [...this.#policies.values()];
    }

    /** Resolve a permission without accepting inherited JavaScript properties. */
    expression(reference: PermissionReference): AccessExpression {
        // read the permission's own declaration
        const definition = this.policy(reference).definition;
        const expression = Object.hasOwn(definition.permissions, reference.name)
            ? definition.permissions[reference.name]
            : undefined;
        if (expression === undefined) {
            throw new AccessError("INVALID_DECLARATION", `unknown permission: ${reference.name}`);
        }

        return expression;
    }

    /** Resolve a declared relation without accepting inherited JavaScript properties. */
    relation(type: Policy, name: string): RelationDescription {
        // read the relation's own declaration
        const relation = Object.hasOwn(type.definition.relations, name)
            ? type.definition.relations[name]
            : undefined;
        if (relation === undefined) {
            throw new AccessError("INVALID_DECLARATION", `unknown relation: ${name}`);
        }

        // add the types contributing to an open relation
        if (!relation.open) {
            return relation;
        }
        const key = PermissionReference.key({
            packageId: type.definition.packageId,
            type: type.definition.name,
            name,
        });

        return {
            ...relation,
            subjects: [...relation.subjects, ...(this.#contributions.get(key) ?? [])],
        };
    }

    /** Read an object type's table mapping, failing for a type without a mapped table. */
    requireMapping(reference: Pick<PermissionReference, "packageId" | "type">): TableMapping {
        const mapping = this.mapping(reference);
        if (!mapping) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `missing database mapping: ${reference.type}`,
            );
        }

        return mapping;
    }

    /** Read the mapping of an object type's table, absent for a type without a mapped table. */
    mapping(reference: Pick<PermissionReference, "packageId" | "type">): TableMapping | undefined {
        return this.#mappings.get(ObjectTypeReference.key(reference));
    }

    /** Return the table mappings in registration order. */
    mappings(): TableMapping[] {
        return [...this.#mappings.values()];
    }

    /** Read what a scope permission grants through a chain: each type and granting relation. */
    inherited(reference: PermissionReference): readonly ChainStep[] {
        // read a permission's chain once, refusing a scope type that encloses itself
        const key = PermissionReference.key(reference);
        const cached = this.#inherited.get(key);
        if (cached !== undefined) {
            return cached;
        }
        const steps = this.#inherit(reference, new Set());
        this.#inherited.set(key, steps);

        return steps;
    }

    /** Walk a permission's chain up its enclosing scopes, refusing one it reaches twice. */
    #inherit(reference: PermissionReference, visited: ReadonlySet<string>): ChainStep[] {
        // refuse a permission the walk reached before
        const key = PermissionReference.key(reference);
        if (visited.has(key)) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `cyclic scope chain: ${reference.type}.${reference.name}`,
            );
        }
        const { relations, contained, enclosing } = this.deciding(reference);
        const reached = new Set([...visited, key]);

        return [
            {
                type: { packageId: reference.packageId, type: reference.type },
                relations,
                ...(contained === undefined ? {} : { contained }),
            },
            ...enclosing.flatMap((permission) => this.#inherit(permission, reached)),
        ];
    }

    /** Read the permission another purely derives from, on its object or by a field relation. */
    source(
        permission: PermissionReference,
    ): { readonly permission: string; readonly relation?: string } | undefined {
        const expression = this.expression(permission);
        if (expression.kind === "permission") {
            return { permission: expression.name };
        } else if (
            expression.kind === "through" &&
            !expression.transitive &&
            this.mapping(permission)?.relations[expression.relation] !== undefined
        ) {
            return { permission: expression.permission, relation: expression.relation };
        }

        return undefined;
    }

    /** Read the object a row's field relation references, absent when the field is empty. */
    related(mapping: TableMapping, relation: string, row: Row): ObjectReference | undefined {
        // read the related identifier
        const field = mapping.relations[relation];
        if (field === undefined) {
            throw new AccessError("INVALID_DECLARATION", `${relation} is not a field relation`);
        }
        const id = row[field.column];
        if (id === null || id === undefined) {
            return undefined;
        } else if (typeof id !== "string") {
            throw new TypeError(`field ${field.column} is not an identifier`);
        }

        // type it by the one type the relation accepts
        const typed = field.subject;
        if (typed === undefined) {
            const only = this.onlySubject(mapping.policy, relation);

            return {
                packageId: only.packageId,
                type: only.type,
                scope: field.scope ?? TableMapping.scope(mapping, row),
                id,
            };
        }

        // or by the row's columns
        return {
            packageId: PackageId.parse(row[typed.packageId]),
            type: TableMapping.text(row, typed.type),
            scope: TableMapping.text(row, typed.scope),
            id,
        };
    }

    /** Read the scope deciding an object's access: a scope object itself, else its scope. */
    governingScope(target: ObjectReference): string {
        return this.policy(target).definition.scope === true ? target.id : target.scope;
    }

    /** Decide whether the transaction's database keeps an object's access rows. */
    async isLocal(transaction: Transaction, object: ObjectReference): Promise<boolean> {
        // follow a role to the scope object defining it
        if (this.mapping(object)?.table === accessRole) {
            const scope = await Scope.object(transaction.view(), object.scope);

            return this.isLocal(transaction, scope);
        }
        // keep a scope object's own access at the scope's home, which keeps the scope itself
        else if (this.policy(object).definition.scope === true) {
            return this.keeps(transaction.view(), object.id);
        }
        // keep another type's access where its rows are local
        else {
            return this.local.some(
                (type) => type.packageId === object.packageId && type.type === object.type,
            );
        }
    }

    /** Decide whether a snapshot's database stores a scope's own access rows. */
    async keeps(snapshot: Snapshot, scope: string): Promise<boolean> {
        const [row] = await Scope.read(snapshot, [scope]);

        return (
            row !== undefined &&
            !this.#isElsewhere(row) &&
            !(await Replica.isReplicated(snapshot, scope, accessRelationship))
        );
    }

    /** Decide whether a scope's row was created in another partition than this database's. */
    #isElsewhere(row: { readonly partition: string } | undefined): boolean {
        return (
            this.partition !== undefined && row !== undefined && row.partition !== this.partition
        );
    }

    /** Refuse writing the access rows of an object another database keeps. */
    async requireLocal(transaction: Transaction, object: ObjectReference): Promise<void> {
        // refuse access a scope's own partition keeps at its home, and access this database replicates
        if (!(await this.isLocal(transaction, object))) {
            const [row] =
                this.policy(object).definition.scope === true
                    ? await Scope.read(transaction.view(), [object.id])
                    : [];
            const reason = this.#isElsewhere(row)
                ? "it lives at its home"
                : "this database has a replica";
            throw new AccessError(
                "FORBIDDEN",
                `cannot write the access of ${object.type} ${object.id} here: ${reason}`,
            );
        }
    }

    /** List the replica requests of the scopes above one, one per scope. */
    async chain(
        database: DatabaseConnection,
        scope: string,
        options: ChainOptions,
        types: readonly ObjectTypeReference[],
    ): Promise<Subscription[]> {
        // read the ancestors the scope's row lists
        const [replica] = await database
            .select({ ancestors: Scope.table.ancestors })
            .from(Scope.table)
            .where(eq(Scope.table.scope, scope));
        const above = replica === undefined ? [] : [...replica.ancestors, Scope.universe.id];

        // replicate a home's chain above it, its parent's until it is born, and a follower's from itself
        const scopes =
            options.kind === "home"
                ? replica === undefined
                    ? [options.parent]
                    : above
                : [scope, ...above];

        // replicate each scope's rows for the scope below
        return scopes.map((replicated) =>
            Authorizer.chainSubscription({
                name: Authorizer.chainReplica(scope),
                scope: replicated,
                below: scope,
                parameters: {
                    local: [...this.local],
                    replicated: [...this.replicated],
                    via: [],
                    between: [],
                    types: [...types],
                },
            }),
        );
    }

    /** Request the one replica of the chains above a follower's outside scopes, in the universe. */
    async chainVia(
        database: DatabaseConnection,
        replica: {
            /** The replica's name, one record per follower. */
            readonly name: string;
            /** The scope the replica is kept for, the follower or the scope it lives in. */
            readonly below: string;
        },
        via: readonly { readonly id: string; readonly scope: string | null }[],
        types: readonly ObjectTypeReference[],
        scoped: readonly ObjectTypeReference[] = this.scoped,
        elsewhere: readonly ObjectTypeReference[] = [],
    ): Promise<Subscription | undefined> {
        // request nothing without a replicated scope or a universe relationship the decisions read
        if (via.length === 0 && types.length === 0) {
            return undefined;
        }

        // read the ancestors of the replicated scopes' replicas and their containing scopes in one query
        const replicated = new Set(via.map((object) => object.id));
        const rows = await database
            .select({ ancestors: Scope.table.ancestors })
            .from(Scope.table)
            .where(inArray(Scope.table.scope, [...replicated]));
        const containing = [
            ...via.flatMap((object) => (object.scope === null ? [] : [object.scope])),
            ...rows.flatMap((row) => row.ancestors),
        ];
        const between = new Set(
            containing.filter((scope) => scope !== Scope.universe.id && !replicated.has(scope)),
        );

        // replicate the chains up to the universe and the inherited rows in them
        return Authorizer.chainSubscription({
            name: replica.name,
            scope: Scope.universe.id,
            below: replica.below,
            parameters: {
                local: [...this.local],
                // take the scopes' own rows from the follower's replica of the universe
                replicated: [
                    ...this.replicated.filter(
                        (type) => this.requireMapping(type).inherited !== undefined,
                    ),
                    ...scoped,
                ].filter(
                    (type) =>
                        !elsewhere.some(
                            (other) =>
                                other.packageId === type.packageId && other.type === type.type,
                        ),
                ),
                via: [...replicated].toSorted(),
                between: [...between].toSorted(),
                types: [...types],
            },
        });
    }

    /** Build the chain replica's name for the scope below, one record per follower. */
    static chainReplica(below: string): string {
        return `${CHAIN_SHAPE}:${below}`;
    }

    /** Build the replica name of the outside chains a scope below reads. */
    static viaReplica(below: string): string {
        return `${CHAIN_SHAPE}:${below}:via`;
    }

    /** Request a scope's chain replica: access rows, inherited rows and own object. */
    static chainSubscription(
        request: Omit<Subscription, "shape" | "parameters" | "after"> & {
            readonly parameters: ChainParameters;
        },
    ): Subscription {
        return { ...request, shape: CHAIN_SHAPE, parameters: Authorizer.#json(request.parameters) };
    }

    /** Request the universe's rows a scope reads, decided for their reader. */
    static universeSubscription(
        request: Omit<Subscription, "shape" | "parameters" | "after"> & {
            readonly parameters: UniverseParameters;
        },
    ): Subscription {
        return {
            ...request,
            shape: UNIVERSE_SHAPE,
            parameters: Authorizer.#json(request.parameters),
        };
    }

    /** Build a scope's chain replica for a request. */
    replicateChain(request: Pick<Subscription, "name" | "scope" | "parameters">): Replica {
        const parameters = ChainParameters.parse(request.parameters);

        return this.#chainReplica(request.name, request.scope, parameters);
    }

    /** Build the replica of the universe's rows a scope reads for a request. */
    replicateUniverse(request: Pick<Subscription, "name" | "scope" | "parameters">): Replica {
        const parameters = UniverseParameters.parse(request.parameters);

        return this.#universeReplica(request.name, request.scope, parameters);
    }

    /** Write a request's parameters in JSON form. */
    static #json(parameters: ChainParameters | UniverseParameters): Subscription["parameters"] {
        return schema.jsonObject().parse(parameters);
    }

    /** Build a chain replica: access rows, replicated types' inherited rows and scopes' rows. */
    #chainReplica(name: string, scope: string, parameters: ChainParameters): Replica {
        // replicate the scope, and the replicated scopes and those between for a follower outside their chains
        const { via } = parameters;
        const scopes = [scope, ...parameters.between, ...via];

        // leave out the access rows of the objects the follower keeps, and of global principals living in the universe
        const remote: Condition = {
            NOT: {
                OR: [
                    ...parameters.local.map((type) => ({
                        objectPackageId: type.packageId,
                        objectType: type.type,
                    })),
                    ...this.global.map((type) => ({
                        objectPackageId: type.packageId,
                        objectType: type.type,
                        objectScope: Scope.universe.id,
                    })),
                ],
            },
        };
        // keep the universe's relationships of the object types the follower's decisions read
        const read: Condition = {
            AND: [
                remote,
                {
                    OR: [
                        { scope: { ne: Scope.universe.id } },
                        ...parameters.types.map((type) => ({
                            scope: Scope.universe.id,
                            objectPackageId: type.packageId,
                            objectType: type.type,
                        })),
                    ],
                },
            ],
        };
        const access = decisionTables.map((table): ChainTable => [
            table,
            table === accessRelationship ? read : undefined,
            scopes,
        ]);

        // add the rows of each replicated type this database keeps
        const kept = parameters.replicated.filter((type) => this.mapping(type) !== undefined);
        const replicated = kept.flatMap((type) => this.#replicatedRows(type, scopes, via));
        const owned = new Set(
            kept
                .filter((type) => this.requireMapping(type).policy.definition.scope === true)
                .map((type) => this.#replicatedTable(type)),
        );

        // replicate them under the request's name and scope, reading each table in its scopes
        const tables = [...access, ...replicated];

        return new Replica({
            name,
            scope,
            tables: tables.map(([table]) => table),
            where: new Map(
                tables.flatMap(([table, where]): [Table, Condition][] =>
                    where === undefined ? [] : [[table, where]],
                ),
            ),
            scopes: new Map(
                tables.map(([table, , living]): [Table, readonly string[]] => [table, living]),
            ),
            everywhere: owned,
        });
    }

    /** Select a replicated type's rows in a chain replica. */
    #replicatedRows(
        type: ObjectTypeReference,
        scopes: readonly string[],
        via: readonly string[],
    ): ChainTable[] {
        const mapping = this.requireMapping(type);
        const table = this.#replicatedTable(type);

        // replicate an inherited type's inherited rows in the scopes its objects may live in
        if (mapping.inherited !== undefined) {
            const scope = mapping.scope;
            const living =
                scope === undefined
                    ? scopes
                    : scopes.filter(
                          (id) =>
                              TableMapping.column(table, scope).definition.schema.safeParse(id)
                                  .success,
                      );

            return living.length === 0 ? [] : [[table, mapping.inherited, living]];
        }
        // replicate a scoped type's rows living in the replicated scopes of a follower outside their chains
        else if (mapping.policy.definition.scope !== true && mapping.scope !== undefined) {
            const scope = mapping.scope;
            const living = via.filter(
                (id) => TableMapping.column(table, scope).definition.schema.safeParse(id).success,
            );

            return living.length === 0 ? [] : [[table, undefined, living]];
        }
        // refuse a type that is neither inherited, a scope nor replicated through its scopes
        else if (mapping.policy.definition.scope !== true) {
            throw new AccessError("NOT_FOUND", `no inherited rows of ${type.type}`);
        }
        // replicate the scope objects' own rows whose identifiers are of the scope type
        else {
            const scopeIdentifier = TableMapping.column(table, mapping.id).definition.schema;
            const owned = scopes.filter((id) => scopeIdentifier.safeParse(id).success);

            return owned.length === 0 ? [] : [[table, { [mapping.id]: { in: owned } }, owned]];
        }
    }

    /** Build a universe replica: each requested type's rows, across the other rows' scopes. */
    #universeReplica(name: string, scope: string, parameters: UniverseParameters): Replica {
        // add each requested type's rows
        const rows = parameters.rows.map(({ type, where }): [Table, Condition] => [
            this.#replicatedTable(type),
            where,
        ]);

        // replicate rows across the scopes of other requested rows, and replicate rows that live anywhere from every scope
        const within = parameters.rows.flatMap(({ type, within: parents }): [Table, Table[]][] =>
            parents === undefined
                ? []
                : [
                      [
                          this.#replicatedTable(type),
                          parents.map((parent) => this.#replicatedTable(parent)),
                      ],
                  ],
        );
        const everywhere = parameters.rows.flatMap(({ type, isEverywhere }) =>
            isEverywhere === true ? [this.#replicatedTable(type)] : [],
        );

        return new Replica({
            name,
            scope,
            tables: rows.map(([table]) => table),
            where: new Map(rows),
            within: new Map(within),
            everywhere: new Set(everywhere),
        });
    }

    /** Read the table of a requested object type, refusing a type without a kept table. */
    #replicatedTable(type: ObjectTypeReference): Table {
        const mapping = this.#mappings.get(ObjectTypeReference.key(type));
        if (mapping === undefined || mapping.table === Scope.table) {
            throw new AccessError("NOT_FOUND", `no object type ${type.type} to replicate`);
        }

        return mapping.table;
    }

    /** List the access rows with changes that affect what a caller of a scope chain may have. */
    watch(chain: readonly string[], subjects: readonly Subject[] = []): Watch[] {
        const ids = [...new Set(subjects.map((subject) => subject.id))];

        return [
            ...decisionTables.map((table): Watch => ({ table, scopes: chain })),
            ...(ids.length === 0
                ? []
                : [
                      Watch.of(accessRelationship, "every", {
                          subjectId: { in: [...ids, WILDCARD] },
                      }),
                  ]),
        ];
    }

    /** Read a page of an object's relationships and role bindings in a snapshot, by identifier. */
    async relationships(
        snapshot: Snapshot,
        object: ObjectReference,
        access: Access,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<Relationship[]> {
        // admit the relationships the caller may read, as relationship reads decide them
        const read = this.policy({ packageId: ACCESS_PACKAGE_ID, type: "relationship" }).permission(
            "read",
        );
        const rows = await snapshot.ordered(accessRelationship, {
            admits: {
                current: this.where(read, access, accessRelationship),
                image: async (row) =>
                    (await this.checkRows(snapshot, read, access, [row])).permitted.has(0),
            },
            where: ObjectReference.columns("object", object),
            orderBy: { id: "asc" },
            ...(page.after === undefined
                ? {}
                : { after: { id: schema.identifier("relationship").parse(page.after) } }),
            limit: page.limit,
        });

        return rows.map((row) => Relationship.decode(row));
    }

    /** Encode a new object's first relationships as the rows its creation writes. */
    initialRelationships(
        object: ObjectReference,
        creation: Creation,
        now: number,
    ): RelationshipRow[] {
        const scope = this.governingScope(object);

        return (creation.relationships ?? []).map((request) => {
            // validate each relation before encoding it
            this.validate({ object, ...request }, now);
            const relationship = {
                id: `relationship-${v7()}`,
                object,
                relation: request.relation,
                subject: request.subject,
                createdAt: now,
                expiresAt: null,
            };

            return {
                ...Relationship.encode(relationship, scope),
                revision: 1,
                ...Manager.values(null),
                detachedAt: null,
            };
        });
    }

    /** Require a relationship with an accepted subject and a future expiry. */
    validate(
        request: {
            readonly object: ObjectReference;
            readonly subject?: Subject;
            readonly expiresAt?: number | null;
        } & ({ readonly relation: string } | { readonly role: string }),
        now: number,
    ): void {
        // read the subject, the expiry and whether the relationship binds a role
        const subject = request.subject;
        const expiresAt = request.expiresAt ?? null;
        const isRole = "role" in request;

        // require a subject the relation accepts
        if (
            "relation" in request &&
            subject !== undefined &&
            !accepts(this.relation(this.policy(request.object), request.relation), subject)
        ) {
            throw new AccessError(
                "FORBIDDEN",
                `relation ${request.relation} does not accept the subject`,
            );
        }
        // refuse a wildcard as a role's subject
        else if (isRole && subject !== undefined && (subject.id === "*" || subject.scope === "*")) {
            throw new AccessError("FORBIDDEN", "a role cannot bind to a wildcard");
        }
        // require a role's subject set to be one that caller resolution expands
        else if (
            isRole &&
            subject?.relation !== undefined &&
            !this.memberships.some(
                (set) =>
                    set.packageId === subject.packageId &&
                    set.type === subject.type &&
                    set.relation === subject.relation,
            )
        ) {
            throw new AccessError(
                "FORBIDDEN",
                `a role cannot bind to ${subject.type}#${subject.relation}: it is not a declared subject set or scope relation`,
            );
        }
        // require a future expiry
        else if (expiresAt !== null && (!Number.isFinite(expiresAt) || expiresAt <= now)) {
            throw new AccessError("FORBIDDEN", "a relationship must expire in the future");
        }
    }

    /** Remove the relationships bound to one request after it commits. */
    async release(database: DatabaseConnection, requestId: string): Promise<void> {
        // delete the request's relationships on local types
        const types = this.local.map((type) =>
            and(
                eq(accessRelationship.objectPackageId, type.packageId),
                eq(accessRelationship.objectType, type.type),
            ),
        );
        if (types.length > 0) {
            await database
                .delete(accessRelationship)
                .where(and(eq(accessRelationship.requestId, requestId), combine(types, "OR")));
        }
    }

    /** Resolve a principal in a scope at the strongest current authentication. */
    resolveAssured(
        snapshot: Snapshot,
        scope: string,
        subject: Subject,
        now: number,
        known?: readonly ScopeLink[],
    ): Promise<Access> {
        return this.resolve(
            snapshot,
            scope,
            {
                subjects: [subject],
                now,
                attributes: {},
                assurance: { level: HIGHEST_ASSURANCE, authenticatedAt: now },
            },
            known,
        );
    }

    /** Resolve a caller in a scope inside its transaction: subject sets, scope chain and roles. */
    resolve(
        snapshot: Snapshot,
        scope: string,
        context: AccessContext,
        links?: readonly ScopeLink[],
    ): Promise<Access> {
        return Access.resolve(snapshot, scope, context, this, links);
    }

    /** Restrict a table's rows to the caller's permitted ones, before any sort, count or page. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        return this.#compiler.where(permission, access, source);
    }

    /** Match one object in its scope if the caller has a permission directly or by a role above. */
    permits(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        return this.#compiler.permits(permission, target, access);
    }

    /** Start reading the grants of rows in a scope chain from a snapshot, shared by all callers. */
    reader(snapshot: Snapshot, scopes: readonly ObjectReference[]): TupleReader {
        return new TupleReader(this, snapshot, scopes);
    }

    /** Decide whether a caller has a permission on one object, and until when. */
    async check(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
        reader?: TupleReader,
    ): Promise<Decision> {
        // read the object, a scope whose object row lives elsewhere by its ancestry row
        const declared = this.requireMapping(target);
        const reading = reader ?? this.reader(snapshot, access.scopes);
        const kept = await reading.row(declared, target);
        const isStanding =
            kept === undefined &&
            declared.policy.definition.scope === true &&
            declared.table !== Scope.table;
        const mapping = isStanding ? TableMapping.ancestry(declared.policy) : declared;
        const row = isStanding ? await reading.row(mapping, target) : kept;
        if (row === undefined) {
            return { isAllowed: false };
        }

        // read the permission's tuple tree on the object
        const tree = await reading.tree(permission, row, mapping);
        const until = earliest([
            access.until,
            Authorizer.#grantsUntil(TupleTree.flatten(tree), access),
        ]);

        // admit the caller when it passes the gate and its authority has the permission
        const isAllowed =
            this.#gate(permission, mapping, row, access, "object") === undefined &&
            TupleTree.permits(tree, access);

        return { isAllowed, ...(until === undefined ? {} : { until }) };
    }

    /** Require the caller to have every permission on one object, reporting the first it lacks. */
    async require(
        snapshot: Snapshot,
        permissions: readonly PermissionReference[],
        target: ObjectReference,
        access: Access,
        reader?: TupleReader,
    ): Promise<void> {
        // decide in memory when reads are cheap, the snapshot is past, layered or recorded, or the target is the universe
        const database = snapshot.database;
        const isUniverse =
            target.packageId === Scope.universe.packageId && target.type === Scope.universe.type;
        if (!snapshot.isLive || database.state.locality === "embedded" || isUniverse) {
            const read = reader ?? this.reader(snapshot, access.scopes);
            for (const permission of permissions) {
                const decision = await this.check(snapshot, permission, target, access, read);
                if (!decision.isAllowed) {
                    await this.#refuse(snapshot, permission, target, access);
                }
            }

            return;
        }

        // decide every permission as a column of a single row
        const columns = permissions.map(
            (permission, index) =>
                sql`CASE WHEN ${this.permits(permission, target, access)} THEN 1 ELSE 0 END AS ${sql.identifier(`permitted_${index}`)}`,
        );
        const [row] = await database.execute(
            sql`SELECT ${sql.join(columns, sql`, `)}`,
            schema.record(
                schema.string(),
                schema.union([schema.number(), schema.string(), schema.bigint()]),
            ),
        );
        if (row === undefined) {
            throw new TypeError("the permission query returned no row");
        }
        const denied = permissions.find((_, index) => Number(row[`permitted_${index}`]) !== 1);
        if (denied) {
            await this.#refuse(snapshot, denied, target, access);
        }
    }

    /** Find the lowest fresh assurance level admitting a refused caller, with elevation age. */
    async challenge(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        admits: (stepped: Access) => Promise<boolean>,
    ): Promise<StepUp | undefined> {
        // start from the caller's own level and the elevation's, whichever is higher
        const context = access.context;
        const elevation = this.elevated.get(PermissionReference.key(permission));
        const lowest = Math.max(context.assurance?.level ?? 1, elevation?.assurance ?? 1);

        // decide again at each level as if the caller authenticated just now
        for (let level = lowest; level <= HIGHEST_ASSURANCE; level++) {
            const stepped = await this.resolve(snapshot, access.scope, {
                ...context,
                assurance: { level, authenticatedAt: context.now },
            });
            if (await admits(stepped)) {
                return {
                    assurance: level,
                    ...(elevation === undefined ? {} : { maxAge: elevation.maxAge }),
                };
            }
        }

        return undefined;
    }

    /** List a page of a type's principals with a permission on one object now, by subject key. */
    async subjects(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        type: ObjectTypeReference,
        now: number,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<Subject[]> {
        // read the permission's grants on the object with one reader of its scope chain
        const scope = this.governingScope(target);
        const links = await Scope.chain(snapshot, scope);
        const reader = this.reader(
            snapshot,
            links.map((link) => link.object),
        );
        const row = await reader.row(this.requireMapping(target), target);
        if (row === undefined) {
            return [];
        }
        const tree = await reader.tree(permission, row, this.requireMapping(target));

        // expand the grants' subject sets into the principals of the type
        const candidates = new Map<string, Subject>();
        const seen = new Set<string>();
        for (
            let frontier = TupleTree.flatten(tree).map((grant) => grant.subject);
            frontier.length > 0;
        ) {
            const sets: Subject[] = [];
            for (const subject of frontier) {
                const key = Subject.key(subject);
                if (seen.has(key) || subject.id === "*") {
                    continue;
                }
                seen.add(key);
                if (subject.relation !== undefined) {
                    sets.push(subject);
                } else if (subject.packageId === type.packageId && subject.type === type.type) {
                    candidates.set(key, subject);
                }
            }
            frontier = await Authorizer.#members(snapshot, sets);
        }

        // decide the candidates after the page's cursor in key order until the page is full
        const ordered = [...candidates.entries()]
            .filter(([key]) => page.after === undefined || key > page.after)
            .toSorted(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
        const permitted: Subject[] = [];
        for (const [, subject] of ordered) {
            if (permitted.length === page.limit) {
                break;
            }
            const access = await this.resolveAssured(snapshot, scope, subject, now, links);
            if ((await this.check(snapshot, permission, target, access, reader)).isAllowed) {
                permitted.push(subject);
            }
        }

        return permitted;
    }

    /** Decide whether a caller has a live relationship or role binding on a scope or in it. */
    async isRelatedIn(snapshot: Snapshot, scope: string, access: Access): Promise<boolean> {
        // find the scope's own object on the access's chain, which lives in its enclosing scope
        const own = access.scopes.find((link) => link.id === scope);
        if (own === undefined) {
            throw new AccessError("INVALID_CONTEXT", `access is not resolved in ${scope}`);
        }

        // list the subjects the authority matches in the scope and its enclosing scope
        const wanted = new Map<string, readonly string[]>();
        const subjects: readonly Subject[] = [
            ...access.authority.subjects,
            anyone.reference("*", "*"),
        ];
        for (const objectScope of new Set([scope, own.scope])) {
            for (const subject of subjects) {
                // match an identity with its wildcards, a subject set exactly
                const isPlain = subject.relation === undefined;
                for (const subjectScope of isPlain ? [subject.scope, "*"] : [subject.scope]) {
                    for (const id of isPlain ? [subject.id, "*"] : [subject.id]) {
                        const tuple = [
                            objectScope,
                            subject.packageId,
                            subject.type,
                            subjectScope,
                            id,
                        ];
                        wanted.set(JSON.stringify(tuple), tuple);
                    }
                }
            }
        }

        // read those subjects' relationships in both scopes in one statement
        const rows = await snapshot.select(
            accessRelationship,
            ["objectScope", "subjectPackageId", "subjectType", "subjectScope", "subjectId"],
            [...wanted.values()],
        );

        // keep a relationship on an object in the scope or on the scope itself, whose subject and conditions pass
        return rows.some(
            (row) =>
                (row.objectScope === scope ||
                    (row.objectId === own.id &&
                        row.objectPackageId === own.packageId &&
                        row.objectType === own.type)) &&
                access.authority.isMember(Relationship.decode(row).subject) &&
                new TupleCondition(row).failure(access.context) === undefined,
        );
    }

    /** Decide whether the caller is an owner of one object. */
    async isOwner(snapshot: Snapshot, target: ObjectReference, access: Access): Promise<boolean> {
        // deny a missing object
        const mapping = this.requireMapping(target);
        const reader = this.reader(snapshot, access.scopes);
        const row = await reader.row(mapping, target);
        if (row === undefined) {
            return false;
        }

        // read the owner bindings on or above the object
        const tree = await reader.ownership(row, mapping);

        return TupleTree.permits(tree, access);
    }

    /** Check a permission on each row, returning the caller's permitted positions and expiry. */
    async checkRows(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        rows: readonly Row[],
        reader = this.reader(snapshot, access.scopes),
        below: ReadonlyMap<string, Access> = new Map(),
        lookup: Lookup = "listing",
    ): Promise<Admission> {
        // decide each row by the caller's access in the row's scope
        const mapping = this.requireMapping(permission);
        const accesses = rows.map((row) => below.get(TableMapping.scope(mapping, row)) ?? access);
        for (const enclosed of below.values()) {
            reader.cover(enclosed.scopes);
        }

        // read the rows' grant trees
        const decided = zip(zip(rows, accesses), await reader.trees(permission, rows));
        const until = earliest([
            access.until,
            ...decided.map(([[, evaluated], tree]) =>
                earliest([
                    evaluated.until,
                    Authorizer.#grantsUntil(TupleTree.flatten(tree), evaluated),
                ]),
            ),
        ]);

        // admit the rows with a tree that admits the caller
        const permitted = decided.flatMap(([[row, evaluated], tree], position) =>
            this.#gate(permission, mapping, row, evaluated, lookup) === undefined &&
            TupleTree.permits(tree, evaluated)
                ? [position]
                : [],
        );

        return { permitted: new Set(permitted), ...(until === undefined ? {} : { until }) };
    }

    /** Explain a caller's permission on one object: the gate and each tuple's failure. */
    async explain(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
    ): Promise<Explanation> {
        // read the object as a row of its type
        if (permission.packageId !== target.packageId || permission.type !== target.type) {
            throw new AccessError(
                "INVALID_CONTEXT",
                "the permission must belong to the object's type",
            );
        }
        const mapping = this.requireMapping(target);
        const reader = this.reader(snapshot, access.scopes);
        const found = await reader.row(mapping, target);
        if (found === undefined) {
            throw new AccessError("NOT_FOUND", `no ${target.type} ${target.id}`);
        }

        // decide the authority by the permission's tree, and by each tuple on its own
        const blocked = this.#gate(permission, mapping, found, access, "object");
        const tree = await reader.tree(permission, found);
        const tuples = TupleTree.flatten(tree).flatMap((tuple) => {
            // drop relation sets the caller is no member of
            const failed = access.authority.failure(tuple, access);
            const isSet =
                tuple.subject.relation !== undefined &&
                tuple.subject.id === tuple.object.id &&
                tuple.subject.type === tuple.object.type &&
                tuple.subject.packageId === tuple.object.packageId;
            if (isSet && failed !== undefined) {
                return [];
            }

            return [
                {
                    path: [...tuple.path],
                    object: tuple.object,
                    subject: tuple.subject,
                    ...(tuple.role === undefined ? {} : { role: tuple.role.id }),
                    ...(failed === undefined ? {} : { failure: failed }),
                },
            ];
        });

        return {
            permission,
            object: target,
            isAllowed: blocked === undefined && TupleTree.permits(tree, access),
            ...(blocked === undefined ? {} : { gate: blocked }),
            tuples,
        };
    }

    /** Refuse a caller a permission on one object, with a step-up challenge when one admits it. */
    async #refuse(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
    ): Promise<never> {
        const stepUp = await this.challenge(
            snapshot,
            permission,
            access,
            async (stepped) => (await this.check(snapshot, permission, target, stepped)).isAllowed,
        );
        if (stepUp !== undefined) {
            throw new AccessError(
                "INSUFFICIENT_AUTHENTICATION",
                "authentication assurance is too low: authenticate again",
                { stepUp },
            );
        }
        throw new AccessError("FORBIDDEN", `permission denied: ${permission.name}`);
    }

    /** Read the gate a request fails on a row before any grant. */
    #gate(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Row,
        access: Access,
        lookup: Lookup,
    ): ReturnType<Access["gate"]> | "outside" {
        // refuse a request that its credential, elevation or suspension refuses
        const refused = access.gate(permission, mapping, row);
        if (refused !== undefined) {
            return refused;
        }

        // take an object read by its key in the scope unchanged
        if (lookup === "object") {
            return undefined;
        }

        // require a listed row to live in the scope or be the scope's own row, as a row of the mapped type
        const definition = mapping.policy.definition;
        const scope = TableMapping.scope(mapping, row);
        const object = access.scopeObject(mapping);
        const isScopeObject =
            object !== undefined &&
            scope === object.scope &&
            TableMapping.text(row, mapping.id) === object.id;

        return (scope !== access.scope && !isScopeObject) ||
            (mapping.isShared === true &&
                (row["packageId"] !== definition.packageId || row["type"] !== definition.name))
            ? "outside"
            : undefined;
    }

    /** Require a mapping to supply the trees and reference columns its expressions need. */
    #requireCompilable(mapping: TableMapping): void {
        const pending = Object.values(mapping.policy.definition.permissions);
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            if (expression.kind === "union" || expression.kind === "intersection") {
                pending.push(...expression.expressions);
            } else if (expression.kind === "exclusion") {
                pending.push(expression.include, expression.exclude);
            } else if (
                expression.kind === "through" &&
                expression.transitive &&
                mapping.trees?.[expression.relation] === undefined
            ) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `missing tree mapping: ${expression.relation}`,
                );
            } else if (
                (expression.kind === "granters" || expression.kind === "readers") &&
                mapping.references?.[expression.reference] === undefined
            ) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `unmapped reference: ${expression.reference}`,
                );
            }
        }
    }

    /** List the types a relation leads to for a permission. */
    scopeTypes(type: Policy, name: string, permission: string): readonly SubjectType[] {
        // follow a relation to its declared subject types
        const relation = this.relation(type, name);
        if (relation.isScope !== true || relation.subjects.length > 0) {
            return relation.subjects;
        }

        // follow the scope of any type to each scope type declaring the permission
        return [...this.#policies.values()]
            .filter(
                (policy) =>
                    policy.definition.scope === true &&
                    Object.hasOwn(policy.definition.permissions, permission),
            )
            .map((policy) => ({ packageId: policy.definition.packageId, type: policy.name }));
    }

    /** List the scope types an arrow to an enclosing scope follows along a chain, nearest first. */
    enclosingTypes(
        type: Policy,
        expression: Extract<AccessExpression, { kind: "through" }>,
        chain: readonly Pick<ObjectReference, "packageId" | "type">[],
    ): readonly SubjectType[] {
        // take the one declared scope type
        const types = this.scopeTypes(type, expression.relation, expression.permission);
        const relation = this.relation(type, expression.relation);
        if (relation.subjects.length === 1) {
            return types;
        }

        // take only the nearest scope's type among several or any
        const [nearest] = chain;

        return types.filter(
            (scope) =>
                nearest !== undefined &&
                scope.packageId === nearest.packageId &&
                scope.type === nearest.type,
        );
    }

    /** Read the one subject type a relation accepts, refusing a relation accepting several. */
    onlySubject(type: Policy, name: string): SubjectType {
        const [only, ...others] = this.relation(type, name).subjects;
        if (only === undefined || others.length > 0) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `relation ${type.definition.name}#${name} needs exactly one subject type`,
            );
        }

        return only;
    }

    /** Require a relation to each object's scope to accept scope types only, kept by the chain. */
    #validateScopeRelation(type: Policy, name: string, relation: RelationDescription): void {
        // require scope types stored by the scope chain, or none for the scope of any type
        const isScopeTypes = relation.subjects.every(
            (scope) =>
                scope.relation === undefined &&
                scope.wildcard !== true &&
                this.policy(scope).definition.scope === true,
        );
        if (!isScopeTypes) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${type.name}.${name} is a scope relation: it accepts scope types only`,
            );
        }
        if (
            this.mapping({ packageId: type.definition.packageId, type: type.name })?.relations[name]
        ) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${type.name}.${name} is a scope relation: a field cannot store it`,
            );
        }
    }

    /** Require a subject type to refer to a registered type and one of its relations. */
    #validateSubject(subject: SubjectType): void {
        // resolve the relation of a subject set
        const type = this.policy(subject);
        if (subject.relation !== undefined) {
            if (subject.wildcard) {
                throw new AccessError("INVALID_DECLARATION", "a subject set cannot be a wildcard");
            }
            // accept a relation, or a permission the object's own relations decide
            if (!Object.hasOwn(type.definition.relations, subject.relation)) {
                this.deciding(type.permission(subject.relation));
            } else {
                this.relation(type, subject.relation);
            }
        }
    }

    /** List the chain objects deciding an arrow to the enclosing scope. */
    chainSteps(
        policy: Policy,
        expression: Extract<AccessExpression, { kind: "through" }>,
        chain: readonly ObjectReference[],
    ): readonly (Omit<ChainStep, "type"> & { readonly object: ObjectReference })[] {
        return this.enclosingTypes(policy, expression, chain)
            .flatMap((type) =>
                this.inherited({
                    packageId: type.packageId,
                    type: type.type,
                    name: expression.permission,
                }),
            )
            .flatMap(({ type, relations, contained }) => {
                const object = chain.find(
                    (link) => link.packageId === type.packageId && link.type === type.type,
                );
                const isDeciding = relations.length > 0 || contained !== undefined;

                return object === undefined || !isDeciding
                    ? []
                    : [{ object, relations, ...(contained === undefined ? {} : { contained }) }];
            });
    }

    /** List the permissions of the scope types an arrow to the enclosing scope follows. */
    #enclosingPermissions(
        policy: Policy,
        expression: Extract<AccessExpression, { kind: "through" }>,
    ): PermissionReference[] {
        const { relation, permission } = expression;

        return this.scopeTypes(policy, relation, permission).map((scope) => ({
            packageId: scope.packageId,
            type: scope.type,
            name: permission,
        }));
    }

    /** Refuse deciding a chain permission by a field or scope chain relation. */
    #requireRelationships(reference: PermissionReference, name: string): void {
        const isScope = this.policy(reference).definition.relations[name]?.isScope === true;
        const isField = this.mapping(reference)?.relations[name] !== undefined;
        if (isScope || isField) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${reference.type}.${reference.name} cannot use ${name}: it is a ${isScope ? "scope" : "field"} relation`,
            );
        }
    }

    /** Read what decides a chain permission: object relations and enclosing scope permissions. */
    deciding(reference: PermissionReference): {
        readonly relations: readonly string[];
        readonly contained?: ChainStep["contained"];
        readonly enclosing: readonly PermissionReference[];
    } {
        // collect the relations and enclosing sets of the permission's unions and named permissions
        const policy = this.policy(reference);
        const relations = new Set<string>();
        let contained: ChainStep["contained"];
        const enclosing: PermissionReference[] = [];
        const seen = new Set<string>();
        const pending = [this.expression(reference)];
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            if (expression.kind === "none") {
                continue;
            }
            // take a relation stored in relationships
            else if (expression.kind === "relation") {
                this.#requireRelationships(reference, expression.name);
                relations.add(expression.name);
            }
            // take the scope's containment set, of any principal or of the named types
            else if (expression.kind === "contained") {
                const { principals } = expression;
                contained =
                    principals === undefined ||
                    (contained !== undefined && contained.principals === undefined)
                        ? {}
                        : { principals: [...(contained?.principals ?? []), ...principals] };
            }
            // follow a union's operands
            else if (expression.kind === "union") {
                pending.push(...expression.expressions);
            }
            // follow a named permission once
            else if (expression.kind === "permission" && !seen.has(expression.name)) {
                seen.add(expression.name);
                pending.push(this.expression(policy.permission(expression.name)));
            }
            // skip a named permission followed before
            else if (expression.kind === "permission") {
                continue;
            }
            // follow the scope enclosing the object
            else if (
                expression.kind === "through" &&
                policy.definition.relations[expression.relation]?.isScope === true
            ) {
                enclosing.push(...this.#enclosingPermissions(policy, expression));
            }
            // refuse what relationships on the chain cannot decide
            else {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `${reference.type}.${reference.name} is a chain permission: it can use relations only`,
                );
            }
        }

        return {
            relations: [...relations],
            ...(contained === undefined ? {} : { contained }),
            enclosing,
        };
    }

    /** List the permissions accepted as sets on a type's enclosed scopes. */
    enclosed(
        reference: Pick<PermissionReference, "packageId" | "type">,
        set: string,
    ): readonly PermissionReference[] {
        return this.#enclosed.get(PermissionReference.key({ ...reference, name: set })) ?? [];
    }

    /** Report whether scopes of a type take subject sets from the scopes enclosing them. */
    isEnclosed(reference: Pick<PermissionReference, "packageId" | "type">): boolean {
        return this.#enclosedTypes.has(ObjectTypeReference.key(reference));
    }

    /** List the permissions accepted as sets that a relationship makes its subject a member of. */
    implied(
        reference: Pick<PermissionReference, "packageId" | "type">,
        relation: string,
    ): readonly string[] {
        return this.#implied.get(PermissionReference.key({ ...reference, name: relation })) ?? [];
    }

    /** Resolve all expressions and keep recursion in explicit ancestor traversal. */
    #validate(reference: PermissionReference, parents: ReadonlySet<string>): void {
        // detect recursion by the complete permission reference
        const key = PermissionReference.key(reference);
        if (parents.has(key)) {
            throw new AccessError("INVALID_DECLARATION", `cyclic permission: ${reference.name}`);
        }

        // retain the named permission path while checking its expression
        const path = new Set(parents).add(key);
        this.#validateExpression(this.policy(reference), this.expression(reference), path);
    }

    /** Resolve references and scalar comparisons within a declared expression. */
    #validateExpression(type: Policy, root: AccessExpression, path: ReadonlySet<string>): void {
        const pending = [root];
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            switch (expression.kind) {
                case "none":
                case "granters":
                case "readers":
                case "contained":
                    break;
                case "self":
                    Authorizer.#validateSelf(type);
                    break;
                case "union":
                case "intersection":
                    pending.push(...expression.expressions);
                    break;
                case "exclusion":
                    pending.push(expression.include, expression.exclude);
                    break;
                case "relation":
                    this.relation(type, expression.name);
                    break;
                case "permission":
                    this.#validate(type.permission(expression.name), path);
                    break;
                case "resource":
                case "context":
                    Authorizer.#validateCondition(expression, type);
                    break;
                case "through":
                    this.#validateThrough(type, expression, path);
                    break;
            }
        }
    }

    /** Require a type admitting itself to be a principal type. */
    static #validateSelf(type: Policy): void {
        const { packageId, name } = type.definition;
        if (!TableMapping.isPrincipalType({ packageId, type: name })) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${name} represents no principal, so self admits no one`,
            );
        }
    }

    /** Require an arrow to follow a relation to plain objects of types declaring the permission. */
    #validateThrough(
        type: Policy,
        expression: Extract<AccessExpression, { kind: "through" }>,
        path: ReadonlySet<string>,
    ): void {
        // follow the relation to plain objects of types declaring the permission
        const definition = type.definition;
        const relation = this.relation(type, expression.relation);
        for (const subject of this.scopeTypes(type, expression.relation, expression.permission)) {
            if (subject.relation !== undefined || subject.wildcard) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `an arrow requires plain object subjects: ${expression.relation}`,
                );
            }

            // require ancestor traversal to stay within a single object type
            if (
                expression.transitive &&
                (relation.subjects.length !== 1 ||
                    subject.packageId !== definition.packageId ||
                    subject.type !== definition.name)
            ) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    "ancestor traversal requires a same-type relation",
                );
            }

            // require a permission of the enclosing scope that its chain's relationships decide
            if (definition.relations[expression.relation]?.isScope === true) {
                this.inherited({
                    packageId: subject.packageId,
                    type: subject.type,
                    name: expression.permission,
                });
            }

            // resolve the referenced permission on the related type
            this.#validate(
                {
                    packageId: subject.packageId,
                    type: subject.type,
                    name: expression.permission,
                },
                path,
            );
        }
    }

    /** Take the next time any grant may start or stop passing, absent if never. */
    static #grantsUntil(grants: readonly Tuple[], access: Access): number | undefined {
        const context = access.context;

        return earliest(
            grants.flatMap((grant) => [
                grant.condition?.until(context),
                ...grant.arrows.map((arrow) => arrow.until(context)),
            ]),
        );
    }

    /** Require a condition to read declared attributes and compare values of their type. */
    static #validateCondition(expression: ConditionExpression, type: Policy): void {
        // read the object's or the request's attributes
        const isObject = expression.kind === "resource";
        const definition = type.definition;
        const predicate = type.resolve(expression);

        // refuse relations and undeclared attributes, which resolve as relations
        const [relation] = Predicate.relations(predicate);
        if (relation !== undefined && Object.hasOwn(definition.relations, relation.via)) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `policy conditions cannot follow relations: ${relation.via}`,
            );
        } else if (relation !== undefined) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `undeclared ${isObject ? "object" : "request"} attribute: ${relation.via}`,
            );
        }

        // compare values of each attribute's type
        Authorizer.#validatePredicate(
            predicate,
            isObject ? definition.attributes : definition.context,
            isObject ? definition.context : undefined,
        );
    }

    /** Require comparisons with values of each attribute's declared type, ordering only numbers. */
    static #validatePredicate(
        predicate: Predicate,
        declared: Readonly<Record<string, AttributeType>>,
        placeholders: Readonly<Record<string, AttributeType>> | undefined,
    ): void {
        switch (predicate.kind) {
            case "compare":
                Authorizer.#validateCompare(predicate, declared, placeholders);
                break;
            case "oneOf":
                Authorizer.#validateListed(predicate, declared);
                break;
            case "missing":
                Authorizer.#attributeType(predicate.name, declared);
                break;
            case "like":
                // require a text attribute for a pattern
                if (Authorizer.#attributeType(predicate.name, declared) !== "string") {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        "pattern applies to text attributes only",
                    );
                }
                break;
            case "characters":
                // require a text attribute for a character set
                if (Authorizer.#attributeType(predicate.name, declared) !== "string") {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        "character set applies to text attributes only",
                    );
                }
                break;
            case "length":
                // require a text attribute, and compare its length as a number
                if (Authorizer.#attributeType(predicate.name, declared) !== "string") {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        "length applies to text attributes only",
                    );
                }
                Authorizer.#validatePredicate(
                    predicate.predicate,
                    { ...declared, [predicate.name]: "number" },
                    placeholders,
                );
                break;
            case "all":
            case "any":
                for (const nested of predicate.predicates) {
                    Authorizer.#validatePredicate(nested, declared, placeholders);
                }
                break;
            case "not":
                Authorizer.#validatePredicate(predicate.predicate, declared, placeholders);
                break;
            case "exists":
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `policy conditions cannot follow relations: ${predicate.via}`,
                );
            case "search":
                throw new AccessError("INVALID_DECLARATION", "policy conditions cannot search");
        }
    }

    /** Require a comparison with a value or attribute of the same type, ordering only numbers. */
    static #validateCompare(
        predicate: Extract<Predicate, { readonly kind: "compare" }>,
        declared: Readonly<Record<string, AttributeType>>,
        placeholders: Readonly<Record<string, AttributeType>> | undefined,
    ): void {
        // read the value's type from the value or its placeholder's request attribute
        const value = predicate.value;
        const valueType =
            value instanceof Placeholder
                ? Authorizer.#placeholderType(value.placeholder, placeholders)
                : typeof value === "object"
                  ? Authorizer.#attributeType(value.field, declared)
                  : typeof value;
        const isOrdered = predicate.operator !== "eq" && predicate.operator !== "ne";
        const attribute = Authorizer.#attributeType(predicate.name, declared);

        // order only numbers
        if (isOrdered && attribute !== "number") {
            throw new AccessError(
                "INVALID_DECLARATION",
                `ordered comparisons require a number attribute: ${predicate.name}`,
            );
        }
        // compare only values of the attribute's type
        else if (valueType !== attribute) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `comparisons require a ${attribute} value: ${predicate.name}`,
            );
        }
    }

    /** Require listed values of the attribute's type. */
    static #validateListed(
        predicate: Extract<Predicate, { readonly kind: "oneOf" }>,
        declared: Readonly<Record<string, AttributeType>>,
    ): void {
        // refuse a listed value of another type
        const attribute = Authorizer.#attributeType(predicate.name, declared);
        if (predicate.values.some((value) => typeof value !== attribute)) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `listed values require ${attribute} values: ${predicate.name}`,
            );
        }
    }

    /** Read a declared attribute's type. */
    static #attributeType(
        name: string,
        declared: Readonly<Record<string, AttributeType>>,
    ): AttributeType {
        const type = Object.hasOwn(declared, name) ? declared[name] : undefined;
        if (type === undefined) {
            throw new AccessError("INVALID_DECLARATION", `undeclared attribute: ${name}`);
        }

        return type;
    }

    /** Read the type of the request attribute a placeholder reads. */
    static #placeholderType(
        name: string,
        placeholders: Readonly<Record<string, AttributeType>> | undefined,
    ): AttributeType {
        // refuse placeholders in request conditions, which read request attributes directly
        if (placeholders === undefined) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `request conditions cannot read placeholders: ${name}`,
            );
        } else if (!Object.hasOwn(placeholders, name)) {
            throw new AccessError("INVALID_DECLARATION", `undeclared request attribute: ${name}`);
        }

        return Authorizer.#attributeType(name, placeholders);
    }

    /** Collect every policy the declared ones refer to, one per type, with intrinsic policies. */
    static #collectPolicies(policies: readonly Policy[]): Policy[] {
        // include every policy the declared policies reference
        const byType = new Map<string, Policy>();
        const visited = new Set<Policy>();
        const pending = [...policies];
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            if (visited.has(type)) {
                continue;
            }
            visited.add(type);
            pending.push(...type.references);

            // keep one policy per type, preferring a representing policy over the intrinsic one
            const existing = byType.get(Authorizer.#policyKey(type));
            if (existing === undefined || INTRINSIC_POLICIES.includes(existing)) {
                byType.set(Authorizer.#policyKey(type), type);
            }
            // refuse two distinct policies for one type
            else if (!INTRINSIC_POLICIES.includes(type)) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `duplicate object type: ${Authorizer.#policyKey(type)}`,
                );
            }
        }

        return [
            ...INTRINSIC_POLICIES.filter((type) => !byType.has(Authorizer.#policyKey(type))),
            ...byType.values(),
        ];
    }

    /** Index the authentication each elevated permission asks for by permission key. */
    static #elevatedPermissions(types: readonly Policy[]): Map<string, Elevation> {
        return new Map(
            types.flatMap((type) =>
                Object.entries(type.definition.elevated ?? {}).map(([name, elevation]) => [
                    PermissionReference.key(type.permission(name)),
                    elevation,
                ]),
            ),
        );
    }

    /** Require an index on the column of a field storing a subject set. */
    static #requireSubjectIndex(
        mapping: TableMapping,
        relation: string,
        field: FieldRelation["field"],
    ): void {
        // find an index or unique constraint leading with the column
        const subjectColumn = TableMapping.column(mapping.table, field.column);
        const isIndexed = mapping.table[TABLE]
            .constraints("sqlite")
            .some(
                (constraint) =>
                    (constraint.kind === "index" || constraint.kind === "unique") &&
                    constraint.columns[0] === subjectColumn,
            );

        // require an index or the primary key
        if (!isIndexed && subjectColumn.definition.primaryKey !== true) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `subject set ${mapping.policy.definition.name}#${relation} needs an index on ${field.column}`,
            );
        }
    }

    /** Collect the keys of the permissions policies list as reserved, administrative or moving. */
    static #listedPermissions(
        types: readonly Policy[],
        list: "reserved" | "administration" | "moving",
    ): Set<string> {
        return new Set(
            types.flatMap((type) =>
                (type.definition[list] ?? []).map((name) =>
                    PermissionReference.key({
                        packageId: type.definition.packageId,
                        type: type.definition.name,
                        name,
                    }),
                ),
            ),
        );
    }

    /** Key a policy by its package and its type name. */
    static #policyKey(policy: Policy): string {
        return ObjectTypeReference.key({
            packageId: policy.definition.packageId,
            type: policy.definition.name,
        });
    }

    /** Read the members of subject sets: the subjects of their objects' set relation. */
    static async #members(snapshot: Snapshot, sets: readonly Subject[]): Promise<Subject[]> {
        // read nothing for no sets
        if (sets.length === 0) {
            return [];
        }

        // read the relationships of each set's relation
        const wanted = new Set(sets.map((set) => Subject.key(set)));
        const rows = await Relationship.readByObject(
            snapshot,
            sets.map((set) => ({
                packageId: set.packageId,
                type: set.type,
                scope: set.scope,
                id: set.id,
            })),
        );

        return rows
            .map((row) => Relationship.decode(row))
            .filter(
                (relationship) =>
                    "relation" in relationship &&
                    wanted.has(
                        Subject.key({ ...relationship.object, relation: relationship.relation }),
                    ),
            )
            .map((relationship) => relationship.subject);
    }

    /** Delete the rows whose cascading relation names a deleted subject, as ON DELETE CASCADE. */
    async cascade(transaction: Transaction, subject: Subject): Promise<void> {
        // find the field relations that cascade and accept the subject
        const cascading = [...this.#mappings.values()].flatMap((mapping) =>
            Object.entries(mapping.policy.definition.relations ?? {})
                .filter(([, relation]) => relation.onDelete === "cascade")
                .filter(([, relation]) => accepts(relation, subject))
                .map(([name]) => ({ mapping, name, column: mapping.relations[name] })),
        );

        // delete the rows naming the subject: by its typed columns, else by its id within its type
        for (const { mapping, name, column } of cascading) {
            if (column === undefined) {
                throw new TypeError(`relation ${name} cascades only from a column`);
            }
            const typed = column.subject;
            const rows = await transaction.read(mapping.table).findMany({
                where: {
                    [column.column]: subject.id,
                    ...(typed === undefined
                        ? {}
                        : {
                              [typed.packageId]: subject.packageId,
                              [typed.type]: subject.type,
                              [typed.scope]: subject.scope,
                          }),
                },
            });
            await transaction.write(
                ...rows.map((row) => ({ table: mapping.table, before: row, after: null })),
            );
        }
    }
}
