import type { Authority } from "./authority.ts";
import {
    type Transaction,
    type SQL,
    type Table,
    Condition,
    Placeholder,
    Predicate,
    Snapshot,
    TABLE,
    Key,
    type Relation,
    type Row,
    type Select,
    type Visibility,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { Identifier, type UuidEnvironment, schema, zip } from "@destack/schema";
import {
    Replica,
    replicaKey,
    Scope,
    type Subscription,
    type ScopeLink,
    Watch,
    ObjectReference,
    ObjectTypeReference,
    type QuerySelection,
    Subject,
} from "@destack/sync";
import type { FamilyError } from "@destack/error";
import { AccessError, type AccessReason } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { relationsOf, PermissionReference, type Policy } from "../declare/policy.ts";
import {
    type AccessExpression,
    type AttributeType,
    CONTAINED,
    type ConditionExpression,
    subjectSet,
    union,
} from "../declare/expression.ts";
import { accepts, type RelationDescription, type SubjectType } from "../declare/subject.ts";
import {
    ACCESS_PACKAGE_ID,
    anyone,
    installedPackage,
    INTRINSIC_POLICIES,
} from "../declare/principal.ts";
import { ACCESS_MAPPINGS } from "../replica/replica.ts";
import { AccessContext, Caller, Credential } from "../context/context.ts";
import { HIGHEST_ASSURANCE, type Elevation, type StepUp } from "../context/elevation.ts";
import { accessRelationship } from "../relationship/table.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { Creation } from "./authorization.ts";
import { accessRole, accessRolePermission, OWNER_ROLE } from "../role/table.ts";
import { accessTables, decisionTables } from "../replica/replica.ts";
import { accessGrant } from "../grant/table.ts";
import { earliest, Access, type Related } from "./access.ts";
import { TupleCondition } from "./condition.ts";
import type { Decision, Explanation } from "./decision.ts";
import { Reader, Tree, type Tuple } from "./tuple.ts";
import { Lookup } from "./lookup.ts";
import { TableMapping, type FieldRelation } from "./mapping.ts";

/** The identifier of a wildcard subject, matching every identity of its type. */
const WILDCARD = "*";

/** How long an access replica stays live without a page: three 10-second beats, in ms. */
const LAG_MILLISECONDS = 30_000;

/** The relation from a relationship with a subject set to the relationships of that set. */
const SUBJECTS: Relation = {
    table: accessRelationship,
    cardinality: "many",
    on: { kind: "key", column: "objectKey", parent: "subjectSetKey" },
};

/** A table a chain replica reads: its row condition, absent for all rows, and its scopes. */
type ChainTable = [Table, Condition | undefined, readonly string[]];

/** A caller's permitted rows of a set by position, and the next time they change. */
export interface PermittedRows {
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
    /** The containment set the permission allows, limited to some principal types when given. */
    readonly contained?: { readonly principals?: readonly ObjectTypeReference[] };
}

/** The parameters of a chain replica: the target's chain, its types, and the outside scopes. */
export const ChainParameters = schema.object({
    /** The target's chain, whose objects' universe relationships a chain replica reads. */
    scopes: schema.array(schema.string().min(1)).readonly(),
    /** The object types the target homes and decides, left out of its own scope's rows. */
    local: schema.array(ObjectTypeReference).readonly(),
    /** The scopes the replica reads, each with the object types it propagates. */
    replicated: schema
        .array(
            schema.object({
                /** The replicated scope. */
                scope: schema.string().min(1),
                /** The object types whose rows the scope propagates or stores. */
                types: schema.array(ObjectTypeReference).readonly(),
            }),
        )
        .min(1)
        .readonly(),
    /** The scopes outside the target's chain a follower replicates via the universe. */
    via: schema.array(schema.string().min(1)).readonly(),
    /** The scopes between the outside ones and the universe, as the follower knows them. */
    between: schema.array(schema.string().min(1)).readonly(),
});
/** The parameters of a chain replica. */
export type ChainParameters = schema.Infer<typeof ChainParameters>;

/** The chain a scope's replicas cover: the chain above a home, or a follower's from itself. */
export interface ChainOptions {
    /** The scope's place: its home, storing its rows, or a follower, replicating them. */
    readonly kind: "home" | "follower";
    /** The scope's parent, where its chain continues. */
    readonly parent: string;
}

/** Decide policies over their objects' tables, in SQL or in memory from grants. */
export class Authorizer {
    /** The permission keys owner roles leave out, granted only by expressions and listing roles. */
    readonly reserved: ReadonlySet<string>;
    /** The authentication each elevated permission asks for, by permission key. */
    readonly elevated: ReadonlyMap<string, Elevation>;
    /** The keys of permissions that stay available while a scope is suspended. */
    readonly suspended: ReadonlySet<string>;
    /** The keys of permissions that stay available while a scope moves. */
    readonly moving: ReadonlySet<string>;
    /** Field relations that another relation uses as a subject set. */
    readonly fields: FieldRelation[];
    /** The tables resolving a caller's access reads, whose writes renew it. */
    readonly decidingTables: readonly Table[];
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
    /** How deep universe subject sets nest, as the universe's chain replica includes them. */
    readonly setDepth: number;
    /** The relations arrows follow, by permission key of their type and name. */
    readonly #arrows = new Set<string>();
    /** The types each type's arrows lead to, by the type's key. */
    readonly #reached = new Map<string, ObjectTypeReference[]>();
    /** The one partition whose scopes' access this database stores, none when absent. */
    readonly partition: string | undefined;
    /** The package serving each type, by type key. */
    readonly #servers: ReadonlyMap<string, PackageId>;
    /** Whether the database stores installations, whose packages' types they act on. */
    readonly #isInstalled: boolean;
    /** The effective expression of each permission read so far, by permission key. */
    readonly #expressions = new Map<string, AccessExpression>();
    /** The tables the database stores rows of, every mapped one when absent. */
    readonly #tables: ReadonlySet<Table> | undefined;
    /** The object types whose access rows live in this database, rather than as replicas. */
    readonly local: readonly ObjectTypeReference[];
    /** The object types replicated from scopes above: propagated rows and stored scope types. */
    readonly replicated: readonly ObjectTypeReference[];
    /** The replicated object types whose rows live in a scope. */
    readonly scoped: readonly ObjectTypeReference[];
    /** The policies indexed by package and type. */
    readonly #policies: Map<string, Policy>;
    /** The subject types other types contribute to each open relation, by type and relation. */
    readonly #contributions: Map<string, SubjectType[]>;
    /** The table mappings indexed by package and type. */
    readonly #mappings: Map<string, TableMapping>;

    /** Validate the policies, their referenced policies and their tables in this database. */
    constructor(
        policies: readonly Policy[],
        mappings: readonly TableMapping[] = [],
        options: {
            /** How long an access replica stays live without a page, in milliseconds. */
            readonly lag?: number;
            /** The tables whose rows the database replicates from their homes, none when absent. */
            readonly replicated?: ReadonlySet<Table>;
            /** The tables the database stores rows of, every mapped one when absent. */
            readonly tables?: ReadonlySet<Table>;
            /** The one partition whose scopes' access the database stores, else none. */
            readonly partition?: string;
            /** The mappings of the subject set types that permissions grant. */
            readonly sets?: readonly TableMapping[];
            /** The package serving each type, by type key. */
            readonly servers?: ReadonlyMap<string, PackageId>;
        } = {},
    ) {
        // start with empty state, the installations first, as every permission's expression reads them
        this.#servers = options.servers ?? new Map();
        this.#isInstalled = (options.sets ?? []).some((set) => set.policy === installedPackage);
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

        // register every policy the declared policies reference, by type, the installed packages where installations live
        const installed = this.#isInstalled ? [installedPackage] : [];
        const types = Authorizer.#collectPolicies([...policies, ...installed]);
        for (const type of types) {
            this.#policies.set(Authorizer.#policyKey(type), type);
        }

        // add contributing types to open relations, validate every permission and record its arrows
        for (const type of types) {
            this.#registerContributions(type);
        }
        for (const type of types) {
            this.#validatePermissions(type);
        }

        // index the permissions the policies list as reserved, elevated, administrative or moving
        this.reserved = Authorizer.#listedPermissions(types, "reserved");
        this.elevated = Authorizer.#elevatedPermissions(types);
        this.suspended = Authorizer.#listedPermissions(types, "suspended");
        this.moving = Authorizer.#listedPermissions(types, "moving");
        this.lag = options.lag ?? LAG_MILLISECONDS;
        this.partition = options.partition;
        this.#tables = options.tables;

        // register the declared mappings, the granted sets' and the mappings every database has
        const granted = (options.sets ?? []).filter((set) =>
            this.#policies.has(Authorizer.#policyKey(set.policy)),
        );
        const declared = [...mappings, ...granted];
        for (const source of [...declared, ...this.#universalMappings(declared)]) {
            this.#registerMapping(source);
        }
        for (const type of types) {
            this.#validateScopeRelations(type);
        }

        // list the types whose access rows this database stores
        const replicated = options.replicated ?? new Set<Table>();
        this.local = this.#localTypes(replicated);
        this.replicated = this.#replicatedTypes(replicated);
        this.scoped = this.#scopedTypes(replicated);

        // expand the subject sets roles may bind to, and measure how deep universe sets nest
        const sets = this.#subjectSets();
        this.setDepth = this.#setDepth();
        this.#expandMemberships(sets);
        this.#registerFieldSets(sets);
        const fieldTables = this.fields.map((field) => field.mapping.table);
        this.decidingTables = [...new Set([...decisionTables, accessGrant, ...fieldTables])];

        // require every declared relation of a mapped type to decide a permission
        this.#requireDeciding(types, sets);
    }

    /** Add a contributing type to each open relation it references. */
    #registerContributions(type: Policy): void {
        for (const entry of type.definition.contributes ?? []) {
            // refuse a contribution to a relation that is not open
            const target = this.#policies.get(ObjectTypeReference.key(entry));
            if (target?.definition.relations[entry.relation]?.open !== true) {
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `${type.name} cannot contribute to '${entry.type}.${entry.relation}': it is not an open relation`,
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
                ...(entry.members === undefined ? {} : { relation: entry.members }),
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

        // resolve the type's grant, reserved, elevated, suspended and moving permissions
        if (definition.grantedBy !== undefined) {
            this.expression(type.permission(definition.grantedBy));
        }
        for (const name of [
            ...(definition.reserved ?? []),
            ...Object.keys(definition.elevated ?? {}),
            ...(definition.suspended ?? []),
            ...(definition.moving ?? []),
        ]) {
            this.expression(type.permission(name));
        }

        // validate named permissions after resolving the declared relations
        for (const name of Object.keys(definition.permissions)) {
            this.#validate(type.permission(name), new Set());
        }
    }

    /** Report whether this database has a mapped type's table, every one without given tables. */
    hasTable(mapping: TableMapping): boolean {
        return this.#tables?.has(mapping.table) ?? true;
    }

    /** List every database's mappings: scope types onto ancestry, access types onto tables. */
    #universalMappings(mappings: readonly TableMapping[]): TableMapping[] {
        // map scope types this database has no table of onto the ancestry every database has
        const scopes = this.policies()
            .filter(
                (policy) =>
                    policy.definition.scope === true &&
                    !mappings.some((mapping) => Authorizer.#isSameType(mapping.policy, policy)),
            )
            .map((policy) => TableMapping.ancestry(policy));

        // map access's types onto its tables where no declared type represents them
        const unrepresented = ACCESS_MAPPINGS.filter(
            (mapping) =>
                Authorizer.#isSameType(
                    this.policy({
                        packageId: mapping.policy.definition.packageId,
                        type: mapping.policy.definition.name,
                    }),
                    mapping.policy,
                ) && !mappings.some((declared) => declared.table === mapping.table),
        );

        return [...scopes, ...unrepresented];
    }

    /** Register a mapping once and validate its columns against its policy. */
    #registerMapping(source: TableMapping): void {
        // require the mapping to reference the registered policy
        const mapping = TableMapping.freeze(source);
        const definition = mapping.policy.definition;
        const registered = this.policy({ packageId: definition.packageId, type: definition.name });
        if (!Authorizer.#isSameType(registered, mapping.policy)) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                "mapping must reference the registered object declaration",
            );
        }

        // refuse a second mapping of the same type
        const key = ObjectTypeReference.key({
            packageId: definition.packageId,
            type: definition.name,
        });
        if (this.#mappings.has(key)) {
            throw AccessError.of("DECLARATION_BROKEN", `duplicate database mapping: '${key}'`);
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
    #localTypes(replicated: ReadonlySet<Table>): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    !accessTables.includes(mapping.table) && !replicated.has(mapping.table),
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** List the mapped types whose tables this database replicates: propagated and scope types. */
    #replicatedTypes(replicated: ReadonlySet<Table>): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    !accessTables.includes(mapping.table) &&
                    this.hasTable(mapping) &&
                    (mapping.propagated !== undefined ||
                        (replicated.has(mapping.table) &&
                            mapping.policy.definition.scope === true &&
                            mapping.table !== Scope.table)),
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** List the replicated types whose rows live in a scope, beside propagated and scope types. */
    #scopedTypes(replicated: ReadonlySet<Table>): ObjectTypeReference[] {
        return [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    !accessTables.includes(mapping.table) &&
                    replicated.has(mapping.table) &&
                    mapping.propagated === undefined &&
                    mapping.policy.definition.scope !== true &&
                    mapping.scope !== undefined,
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
    }

    /** Collect the subject sets the declared relations accept and the permissions grant, by key. */
    #subjectSets(): Map<string, PermissionReference> {
        // collect the sets the relations accept, declared or contributed to open relations
        const accepted = [
            ...this.policies()
                .flatMap((policy) => Object.values(policy.definition.relations))
                .flatMap((relation) => relation.subjects),
            ...[...this.#contributions.values()].flat(),
        ].flatMap((subject) =>
            subject.relation === undefined
                ? []
                : [
                      {
                          packageId: subject.packageId,
                          type: subject.type,
                          name: subject.relation,
                      },
                  ],
        );

        // collect the sets the permissions grant to their members, the packages' installations among them
        const granted = this.policies().flatMap((policy) => policy.memberSets());
        if (this.#isInstalled) {
            granted.push({ ...installedPackage.typeReference, name: "installation" });
        }

        return new Map(
            [...accepted, ...granted].map((set) => [PermissionReference.key(set), set] as const),
        );
    }

    /** Measure how deep subject sets of universe objects nest, refusing a cycle among them. */
    #setDepth(): number {
        const roots = this.policies().flatMap((policy) =>
            this.#universeSets(policy).map((subject) => this.#nesting(this.policy(subject), [])),
        );

        return Math.max(0, ...roots);
    }

    /** Measure how deep universe subject sets nest below a policy, refusing a cycle. */
    #nesting(policy: Policy, path: readonly Policy[]): number {
        // refuse a set type found again along the path, listing the cycle
        const key = ObjectTypeReference.key(policy.typeReference);
        if (path.some((step) => ObjectTypeReference.key(step.typeReference) === key)) {
            const cycle = [...path, policy].map((step) => step.name).join(" -> ");
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `universe subject sets nest in a cycle: '${cycle}'`,
            );
        }

        // follow each universe set its relations accept
        const deeper = this.#universeSets(policy).map((subject) =>
            this.#nesting(this.policy(subject), [...path, policy]),
        );

        return 1 + Math.max(0, ...deeper);
    }

    /** List the subject sets of universe types a policy's relations accept. */
    #universeSets(policy: Policy): SubjectType[] {
        return Object.values(policy.definition.relations)
            .flatMap((relation) => relation.subjects)
            .filter((subject) => subject.relation !== undefined && this.#isUniverseType(subject));
    }

    /** Report whether a type's objects live in the universe alone, which every chain includes. */
    #isUniverseType(type: Pick<PermissionReference, "packageId" | "type">): boolean {
        const mapping = this.mapping(type);

        return mapping !== undefined && mapping.scope === undefined;
    }

    /** Expand the declared sets and every scope's relations into the memberships roles bind to. */
    #expandMemberships(sets: ReadonlyMap<string, PermissionReference>): void {
        // add every tuple relation of each scope type to the declared sets
        const expanded = new Map(sets);
        for (const policy of this.policies().filter((each) => each.definition.scope === true)) {
            const fields = this.mapping(policy.typeReference)?.relations ?? {};
            const tuples = Object.keys(policy.definition.relations).filter(
                (relation) => !Object.hasOwn(fields, relation),
            );
            for (const relation of tuples) {
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
            // expand a scope's contents and a relation's set as they are
            if (
                set.name === CONTAINED ||
                Object.hasOwn(this.policy(set).definition.relations, set.name)
            ) {
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
                    throw AccessError.of(
                        "DECLARATION_BROKEN",
                        `no permission uses relation '${definition.name}.${name}'`,
                    );
                }
            }
        }
    }

    /** Collect the keys of the mapped relations to an owner parent. */
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

    /** Report whether arrows follow a relation of a type, so its tuples lead to other objects. */
    isArrow(type: Pick<PermissionReference, "packageId" | "type">, relation: string): boolean {
        return this.#arrows.has(PermissionReference.key({ ...type, name: relation }));
    }

    /** Resolve the policy of a package-qualified object type. */
    policy(reference: Pick<ObjectReference, "packageId" | "type">): Policy {
        const policy = this.#policies.get(ObjectTypeReference.key(reference));
        if (!policy) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `unknown object type ${reference.type} of '${reference.packageId}'`,
            );
        }

        return policy;
    }

    /** Find a registered type's policy, absent for a type the authorizer does not know. */
    find(reference: Pick<ObjectReference, "packageId" | "type">): Policy | undefined {
        return this.#policies.get(ObjectTypeReference.key(reference));
    }

    /** Resolve the policy of an identifier's type by its prefix, refusing none or several. */
    policyOf(id: Identifier<string>): Policy {
        // match the policies of the identifier's prefix
        const prefix = Identifier.prefix(id);
        const [policy, ...others] = [...this.#policies.values()].filter(
            (each) => each.typeReference.type === prefix,
        );
        if (policy === undefined || others.length > 0) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                policy === undefined
                    ? `no known type has identifiers like '${id}'`
                    : `several known types have identifiers like '${id}'`,
            );
        }

        return policy;
    }

    /** Return the policies in registration order. */
    policies(): Policy[] {
        return [...this.#policies.values()];
    }

    /** Resolve a permission without accepting inherited JavaScript properties. */
    expression(reference: PermissionReference): AccessExpression {
        // read the expression once per permission
        const key = PermissionReference.key(reference);
        const known = this.#expressions.get(key);
        if (known !== undefined) {
            return known;
        }
        const effective = this.#effective(reference);
        this.#expressions.set(key, effective);

        return effective;
    }

    /** Read a permission's expression, given to the installations of its type's packages too. */
    #effective(reference: PermissionReference): AccessExpression {
        // read the permission's declaration
        const definition = this.policy(reference).definition;
        const expression = Object.hasOwn(definition.permissions, reference.name)
            ? definition.permissions[reference.name]
            : undefined;
        if (expression === undefined) {
            throw AccessError.of("DECLARATION_BROKEN", `unknown permission: '${reference.name}'`);
        }

        // give an unreserved permission to the installations of the packages declaring and serving its type
        if (!this.#isInstalled || definition.reserved?.includes(reference.name) === true) {
            return expression;
        }
        const server = this.#servers.get(ObjectTypeReference.key(reference));
        const packages = new Set([
            this.policy(reference).package.id,
            ...(server === undefined ? [] : [server]),
        ]);

        return union(
            expression,
            ...[...packages].map((id) => subjectSet(installedPackage, id, "installation", "chain")),
        );
    }

    /** List the chain subject sets a permission grants on its type's objects in their chain. */
    chainSets(reference: PermissionReference): Extract<AccessExpression, { kind: "subjectSet" }>[] {
        // grant no sets for a permission no policy here declares, as a role may name one
        const policy = this.find(reference);
        if (policy === undefined || !Object.hasOwn(policy.definition.permissions, reference.name)) {
            return [];
        }

        // follow the permission's unions and the named permissions of its type once each
        const sets: Extract<AccessExpression, { kind: "subjectSet" }>[] = [];
        const seen = new Set<string>([reference.name]);
        const pending = [this.expression(reference)];
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            // take a chain set
            if (expression.kind === "subjectSet" && expression.scope === "chain") {
                sets.push(expression);
            }
            // follow a union's operands
            else if (expression.kind === "union") {
                pending.push(...expression.expressions);
            }
            // follow a named permission of the type once
            else if (expression.kind === "permission" && !seen.has(expression.name)) {
                seen.add(expression.name);
                pending.push(this.expression({ ...reference, name: expression.name }));
            }
        }

        return sets;
    }

    /** Resolve a declared relation without accepting inherited JavaScript properties. */
    relation(type: Policy, name: string): RelationDescription {
        // read the relation's declaration
        const relation = Object.hasOwn(type.definition.relations, name)
            ? type.definition.relations[name]
            : undefined;
        if (relation === undefined) {
            throw AccessError.of("DECLARATION_BROKEN", `unknown relation: '${name}'`);
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
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

    /** Walk a permission's chain up its enclosing scopes, refusing one it visits twice. */
    #inherit(reference: PermissionReference, visited: ReadonlySet<string>): ChainStep[] {
        // refuse a permission the walk visited before
        const key = PermissionReference.key(reference);
        if (visited.has(key)) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `cyclic scope chain: '${reference.type}.${reference.name}'`,
            );
        }
        const { relations, contained, enclosing } = this.deciding(reference);
        const visiting = new Set([...visited, key]);

        return [
            {
                type: { packageId: reference.packageId, type: reference.type },
                relations,
                ...(contained === undefined ? {} : { contained }),
            },
            ...enclosing.flatMap((permission) => this.#inherit(permission, visiting)),
        ];
    }

    /** Read the permissions another derives from, on its object or by a field relation. */
    sources(
        permission: PermissionReference,
    ): readonly { readonly permission: string; readonly relation?: string }[] {
        // read a union's branches, else the expression alone
        const expression = this.expression(permission);
        const branches = expression.kind === "union" ? expression.expressions : [expression];

        // take each branch deriving from a permission on the object or through a mapped field relation
        const relations = this.mapping(permission)?.relations ?? {};

        return branches.flatMap((branch) =>
            branch.kind === "permission"
                ? [{ permission: branch.name }]
                : branch.kind === "through" &&
                    !branch.transitive &&
                    relations[branch.relation] !== undefined
                  ? [{ permission: branch.permission, relation: branch.relation }]
                  : [],
        );
    }

    /** Read the object a row's field relation references, absent when the field is empty. */
    async related(
        snapshot: Snapshot,
        mapping: TableMapping,
        relation: string,
        row: Row,
    ): Promise<ObjectReference | undefined> {
        // read the related identifier, or the key the subject holds
        const field = mapping.relations[relation];
        if (field === undefined) {
            throw AccessError.of("DECLARATION_BROKEN", `'${relation}' is not a field relation`);
        }
        const value = row[field.column];
        if (value === null || value === undefined) {
            return undefined;
        } else if (typeof value !== "string") {
            throw new TypeError(`field ${field.column} is not an identifier`);
        }

        // read the one subject type's object holding a key, none when nobody holds it
        if (field.key !== undefined) {
            const only = this.onlySubject(mapping.policy, relation);
            const target = this.requireMapping(only);
            const within =
                target.scope === undefined
                    ? Scope.universe.id
                    : (field.scope ?? TableMapping.scope(mapping, row));
            const [held] = await TableMapping.read(snapshot, target, within, [value], field.key);

            return held === undefined
                ? undefined
                : {
                      ...only,
                      scope: TableMapping.scope(target, held),
                      id: TableMapping.text(held, target.id),
                  };
        }
        // type an identifier by the one type the relation accepts
        else if (field.subject === undefined) {
            const only = this.onlySubject(mapping.policy, relation);

            return {
                packageId: only.packageId,
                type: only.type,
                scope: field.scope ?? TableMapping.scope(mapping, row),
                id: value,
            };
        }
        // or by the row's columns
        else {
            const typed = field.subject;

            return {
                packageId: PackageId.parse(row[typed.packageId]),
                type: TableMapping.text(row, typed.type),
                scope: TableMapping.text(row, typed.scope),
                id: value,
            };
        }
    }

    /** Read the scope deciding an object's access: a scope object itself, else its scope. */
    governingScope(target: ObjectReference): string {
        return this.policy(target).definition.scope === true ? target.id : target.scope;
    }

    /** Decide whether the transaction's database stores an object's access rows. */
    async isLocal(transaction: Transaction, object: ObjectReference): Promise<boolean> {
        // store a role's and a scope object's access at the home of the scope containing it
        const isScoped =
            this.mapping(object)?.table === accessRole ||
            this.policy(object).definition.scope === true;
        if (isScoped) {
            return this.isHome(transaction.view(), object.scope);
        }
        // store another type's access where its rows are local
        else {
            return this.local.some(
                (type) => type.packageId === object.packageId && type.type === object.type,
            );
        }
    }

    /** Decide whether a snapshot's database is a scope's home: its partition's, and no copy. */
    async isHome(snapshot: Snapshot, scope: string): Promise<boolean> {
        // home the universe at the database of its partition alone
        if (scope === Scope.universe.id) {
            return this.partition === Scope.universe.id;
        }

        // require the scope's row in this database's partition, stored by no replica, reading both together
        const [[row], isCopied] = await Promise.all([
            Scope.read(snapshot, [scope]),
            Authorizer.#isCopied(snapshot, scope),
        ]);
        const isPartition = this.partition !== undefined && row?.partition === this.partition;

        return isPartition && !isCopied;
    }

    /** Decide whether a replica stored a scope's row here, by key or with the whole scope. */
    static async #isCopied(snapshot: Pick<Snapshot, "read">, scope: string): Promise<boolean> {
        const [stored, isReplicated] = await Promise.all([
            snapshot.read(replicaKey).findFirst({
                where: { table: Scope.table[TABLE].sqlName, key: Key.name(Scope.table, { scope }) },
            }),
            Replica.isReplicated(snapshot, scope, Scope.table),
        ]);

        return stored !== undefined || isReplicated;
    }

    /** Refuse a relationship another database stores, returning the scope it lives in. */
    async requireRelationship(
        transaction: Transaction,
        request: { readonly object: ObjectReference } & ({ relation: string } | { role: string }),
    ): Promise<string> {
        // store a binding inside its scope at the scope's home, and any other beside its object
        const scope = await Relationship.scope(transaction.view(), request);
        if (scope === request.object.id) {
            await this.requireHome(transaction, request.object, request.object.id);
        } else {
            await this.requireLocal(transaction, request.object);
        }

        return scope;
    }

    /** Refuse writing the access rows of an object another database stores. */
    async requireLocal(transaction: Transaction, object: ObjectReference): Promise<void> {
        if (!(await this.isLocal(transaction, object))) {
            throw await this.#notLocal(transaction, object, object.scope);
        }
    }

    /** Refuse writing an object's access rows in a scope another database homes. */
    async requireHome(
        transaction: Transaction,
        object: ObjectReference,
        scope: string,
    ): Promise<void> {
        if (!(await this.isHome(transaction.view(), scope))) {
            throw await this.#notLocal(transaction, object, scope);
        }
    }

    /** Build the refusal of a write another database stores: a home, or a replica's source. */
    async #notLocal(
        transaction: Transaction,
        object: ObjectReference,
        scope: string,
    ): Promise<FamilyError<AccessReason>> {
        // name the scope this database lacks, the one it replicates, or the one homed elsewhere
        const view = transaction.view();
        const [row] = await Scope.read(view, [scope]);

        // refuse a scope this database lacks
        if (row === undefined) {
            return AccessError.of(
                "ACCESS_DENIED",
                `access of ${object.type} '${object.id}' is forbidden: this database lacks scope '${scope}'`,
            );
        }
        // refuse a scope this database replicates
        else if (await Authorizer.#isCopied(view, scope)) {
            return AccessError.of(
                "ACCESS_DENIED",
                `access of ${object.type} '${object.id}' is forbidden: this database has a replica of scope '${scope}'`,
            );
        }
        // refuse a scope another database homes
        else {
            return AccessError.of(
                "ACCESS_DENIED",
                `access of ${object.type} '${object.id}' is forbidden: scope '${scope}' has its home in another database`,
            );
        }
    }

    /** Build the one replica of a target's chain over each replicated scope. */
    replicateChain(
        request: Pick<Subscription, "name" | "scope" | "target">,
        parameters: ChainParameters,
    ): Replica {
        const arms = parameters.replicated.map(({ scope, types }) =>
            this.#chainScope({ ...request, scope }, parameters, types),
        );

        return Replica.union(request.name, request.scope, arms);
    }

    /** Build the rows a chain replica reads of one scope: its access rows and propagated rows. */
    #chainScope(
        request: Pick<Subscription, "name" | "scope" | "target">,
        parameters: ChainParameters,
        types: readonly ObjectTypeReference[],
    ): Replica {
        // read the replicated scope, and the outside scopes and those between for a follower
        const { name, scope, target } = request;
        const { local, via, between } = parameters;
        const scopes = [scope, ...between, ...via];
        const isGranting = scope === Scope.universe.id && via.length === 0;
        const where = new Map<Table, Condition>();

        // read the universe's relationships on chain objects alone, and elsewhere the chain's scope objects alone
        const isUniverse = scope === Scope.universe.id;
        const objects = [...new Set([scope, ...parameters.scopes, ...between, ...via])];
        const others = isUniverse
            ? { scope: { ne: scope } }
            : { NOT: { OR: this.#scopeTypes().map((type) => Authorizer.#objectType(type)) } };
        const followed = { OR: [others, { objectId: { in: objects } }] };

        // leave out of the target's own scope the access rows of the objects it homes
        const homed = { NOT: { OR: local.map((type) => Authorizer.#objectType(type)) } };
        const isHomed = !isUniverse && scope === target && local.length > 0;
        where.set(accessRelationship, isHomed ? { AND: [followed, homed] } : followed);

        // read the role permissions on the types the target decides: its own, those propagated, scopes, and those their arrows reach
        const decided = this.#reaching([...local, ...types, ...this.#scopeTypes()]);
        where.set(accessRolePermission, {
            OR: decided.map(({ packageId, type }) => ({ packageId, type })),
        });

        // read the grants to the principals in the target and to delegates acting in it, from every scope
        if (isGranting) {
            where.set(accessGrant, {
                OR: [{ delegateScope: target }, { delegateSpaceId: target }],
            });
        }
        const access = [...decisionTables, ...(isGranting ? [accessGrant] : [])];

        // take the stored types the scope propagates or stores for a follower outside
        const stored = types.filter((type) => {
            const mapping = this.mapping(type);

            return mapping !== undefined && this.hasTable(mapping);
        });

        // read each typed table once, with its condition
        const typed = Authorizer.#merged(
            stored.flatMap((type) => this.#replicatedRows(type, scopes, via)),
        );
        for (const [table, condition] of typed) {
            if (condition !== undefined) {
                where.set(table, condition);
            }
        }

        // replicate the scope types' rows from every scope
        const tables = [...access, ...typed.map(([table]) => table)];
        const scopeTables = stored
            .filter((type) => this.requireMapping(type).policy.definition.scope === true)
            .map((type) => this.#replicatedTable(type));

        return new Replica({
            name,
            scope,
            tables,
            where,
            scopes: new Map([
                ...access.map((table): [Table, readonly string[]] => [
                    table,
                    table === accessGrant ? [scope] : scopes,
                ]),
                ...typed.map(([table, , living]): [Table, readonly string[]] => [table, living]),
            ]),
            everywhere: new Set([...(isGranting ? [accessGrant] : []), ...scopeTables]),
            ...(isUniverse && this.setDepth > 0
                ? {
                      includes: new Map([
                          [accessRelationship, Authorizer.#subjects(this.setDepth)],
                      ]),
                      relations: new Map([[accessRelationship, { subjects: SUBJECTS }]]),
                  }
                : {}),
        });
    }

    /** List the object types that are scopes. */
    #scopeTypes(): ObjectTypeReference[] {
        return this.policies()
            .filter((policy) => policy.definition.scope === true)
            .map((policy) => policy.typeReference);
    }

    /** List some types with every type their arrows lead to, transitively, each once. */
    #reaching(types: readonly ObjectTypeReference[]): ObjectTypeReference[] {
        const reached = new Map(types.map((type) => [Authorizer.#typeKey(type), type]));
        for (const type of reached.values()) {
            for (const next of this.#reached.get(Authorizer.#typeKey(type)) ?? []) {
                if (!reached.has(Authorizer.#typeKey(next))) {
                    reached.set(Authorizer.#typeKey(next), next);
                }
            }
        }

        return [...reached.values()];
    }

    /** Key an object type by its package and name. */
    static #typeKey(type: ObjectTypeReference): string {
        return `${type.packageId}/${type.type}`;
    }

    /** Match the relationships on objects of one type. */
    static #objectType(type: ObjectTypeReference): Condition {
        return { objectPackageId: type.packageId, objectType: type.type };
    }

    /** Merge the typed reads of one table: any type's rows, in every type's scopes. */
    static #merged(reads: readonly ChainTable[]): ChainTable[] {
        // group the reads by table, in the first one's order
        const merged = new Map<Table, ChainTable>();
        for (const [table, condition, living] of reads) {
            const known = merged.get(table);
            // take the first type's read of a table
            if (known === undefined) {
                merged.set(table, [table, condition, living]);
            }
            // read every row once a type reads them all, else any type's rows, in the scopes of both
            else {
                const [, before, scoped] = known;
                const either =
                    before === undefined || condition === undefined
                        ? undefined
                        : { OR: [before, condition] };
                merged.set(table, [table, either, [...new Set([...scoped, ...living])]]);
            }
        }

        return [...merged.values()];
    }

    /** Nest the includes of the subjects' relationships some levels deep. */
    static #subjects(depth: number): NonNullable<QuerySelection["with"]> {
        return { subjects: depth === 1 ? true : { with: Authorizer.#subjects(depth - 1) } };
    }

    /** Decide whether a scope propagates rows of an object type. */
    propagates(type: ObjectTypeReference, scope: string): boolean {
        return this.#replicatedRows(type, [scope], []).length > 0;
    }

    /** Select a replicated type's rows in a chain replica. */
    #replicatedRows(
        type: ObjectTypeReference,
        scopes: readonly string[],
        via: readonly string[],
    ): ChainTable[] {
        const mapping = this.requireMapping(type);
        const table = this.#replicatedTable(type);

        // replicate a propagated type's rows in the scopes its objects may live in
        if (mapping.propagated !== undefined) {
            const scope = mapping.scope;
            const living =
                scope === undefined
                    ? scopes
                    : scopes.filter(
                          (id) =>
                              TableMapping.column(table, scope).definition.schema.safeParse(id)
                                  .success,
                      );

            return living.length === 0 ? [] : [[table, mapping.propagated, living]];
        }
        // replicate a scoped type's rows living in the replicated scopes of a follower outside their chains
        else if (mapping.policy.definition.scope !== true && mapping.scope !== undefined) {
            const scope = mapping.scope;
            const living = via.filter(
                (id) => TableMapping.column(table, scope).definition.schema.safeParse(id).success,
            );

            return living.length === 0 ? [] : [[table, undefined, living]];
        }
        // refuse a type that is neither propagated, a scope nor replicated through its scopes
        else if (mapping.policy.definition.scope !== true) {
            throw AccessError.of("ROWS_NOT_FOUND", `propagated rows of ${type.type} not found`);
        }
        // replicate the scope objects' rows whose identifiers are of the scope type
        else {
            const scopeIdentifier = TableMapping.column(table, mapping.id).definition.schema;
            const identified = scopes.filter((id) => scopeIdentifier.safeParse(id).success);

            return identified.length === 0
                ? []
                : [[table, { [mapping.id]: { in: identified } }, identified]];
        }
    }

    /** Read the table of a requested object type, refusing a type without a stored table. */
    #replicatedTable(type: ObjectTypeReference): Table {
        const mapping = this.#mappings.get(ObjectTypeReference.key(type));
        if (mapping === undefined || mapping.table === Scope.table) {
            throw AccessError.of(
                "TYPE_NOT_FOUND",
                `object type ${type.type} to replicate not found`,
            );
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
        // allow the relationships the caller may read, as relationship reads decide them
        const read = this.policy({ packageId: ACCESS_PACKAGE_ID, type: "relationship" }).permission(
            "read",
        );
        const where = ObjectReference.columns("object", object);
        const rows = await snapshot.ordered(accessRelationship, {
            visible: await this.visibility(snapshot, read, access),
            where,
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
        environment: UuidEnvironment,
    ): Select<typeof accessRelationship>[] {
        return (creation.relationships ?? []).map((request) => {
            // validate each relation before encoding it
            this.validate({ object, ...request }, now);
            const relationship = {
                id: Identifier.create("relationship", environment),
                object,
                relation: request.relation,
                subject: request.subject,
                createdAt: now,
                expiresAt: null,
            };

            return {
                ...Relationship.encode(relationship),
                revision: 1,
                ...Manager.values(null),
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
            throw AccessError.of(
                "SUBJECT_INVALID",
                `subject is invalid: relation '${request.relation}' does not accept it`,
            );
        }
        // refuse a wildcard as a role's subject
        else if (isRole && subject !== undefined && (subject.id === "*" || subject.scope === "*")) {
            throw AccessError.of(
                "WILDCARD_ROLE_INVALID",
                "role binding is invalid: a role cannot bind to a wildcard",
            );
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
            throw AccessError.of(
                "ROLE_SUBJECT_DENIED",
                `subject '${subject.type}#${subject.relation}' is forbidden: roles bind to subject sets and scope relations`,
            );
        }
        // refuse an expiry on a relationship that must stay, and require a future one elsewhere
        else if (expiresAt !== null && this.isRequired(request)) {
            throw AccessError.of(
                "RELATIONSHIP_EXPIRY_INVALID",
                "relationship expiry is invalid: a required relationship cannot expire",
            );
        } else if (expiresAt !== null && (!Number.isFinite(expiresAt) || expiresAt <= now)) {
            throw AccessError.of(
                "RELATIONSHIP_EXPIRY_INVALID",
                "relationship expiry is invalid: it must be in the future",
            );
        }
    }

    /** Report whether an object must keep one such relationship: an owner or required relation. */
    isRequired(
        request: { readonly object: ObjectReference } & ({ relation: string } | { role: string }),
    ): boolean {
        return "role" in request
            ? request.role === OWNER_ROLE.id
            : this.relation(this.policy(request.object), request.relation).isRequired === true;
    }

    /** Resolve a principal in a scope at the strongest current authentication. */
    resolveAssured(
        snapshot: Snapshot,
        scope: string,
        subject: Subject,
        now: number,
        known?: readonly ScopeLink[],
    ): Promise<Access> {
        return this.resolve(snapshot, scope, AccessContext.of(Caller.of(subject), now), known);
    }

    /** Resolve a caller in a scope inside its transaction: subject sets, scope chain and roles. */
    resolve(
        snapshot: Snapshot,
        scope: string,
        context: AccessContext,
        links?: readonly ScopeLink[],
        assumed?: number,
    ): Promise<Access> {
        return Access.resolve(snapshot, scope, context, this, links, assumed);
    }

    /** Match a table's rows a caller may hold a permission on: a superset `checkRows` decides. */
    candidates(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        source?: Table,
    ): Promise<SQL> {
        return new Lookup(this, snapshot, access).where(permission, source);
    }

    /** Show a permission's rows to a caller: candidates in SQL, each decided by its tuple tree. */
    async visibility(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        source?: Table,
    ): Promise<Visibility> {
        return {
            candidates: await this.candidates(snapshot, permission, access, source),
            decide: async (rows) =>
                (await this.checkRows(snapshot, permission, access, rows)).permitted,
        };
    }

    /** Start reading the grants of rows in a scope chain from a snapshot, shared by all callers. */
    reader(snapshot: Snapshot, scopes: readonly ObjectReference[]): Reader {
        return new Reader(this, snapshot, scopes);
    }

    /** Decide whether a caller has a permission on one object, and until when. */
    async check(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
        reader?: Reader,
    ): Promise<Decision> {
        // allow a permission the caller's stated sets decide on the object without its row
        if (this.#isStated(permission, target, access)) {
            return {
                isAllowed: true,
                ...(access.until === undefined ? {} : { until: access.until }),
            };
        }

        // read the object, a scope whose object row lives elsewhere by its ancestry row
        const declared = this.requireMapping(target);
        const reading = reader ?? this.reader(snapshot, access.scopes);
        const found = await reading.row(declared, target);
        const isStanding =
            found === undefined &&
            declared.policy.definition.scope === true &&
            declared.table !== Scope.table;
        const mapping = isStanding ? TableMapping.ancestry(declared.policy) : declared;
        const row = isStanding ? await reading.row(mapping, target) : found;
        if (row === undefined) {
            return { isAllowed: false };
        }

        // read the permission's tuple tree on the object
        const tree = await reading.tree(permission, row, mapping);
        const until = earliest([access.until, Authorizer.#grantsUntil(Tree.flatten(tree), access)]);

        // allow the caller when it passes the gate and its authority has the permission
        const isAllowed =
            (await this.#gate(snapshot, permission, mapping, row, access)) === undefined &&
            Tree.permits(tree, access);

        return { isAllowed, ...(until === undefined ? {} : { until }) };
    }

    /** Require the caller to have every permission on one object, reporting the first it lacks. */
    async require(
        snapshot: Snapshot,
        permissions: readonly PermissionReference[],
        target: ObjectReference,
        access: Access,
        reader?: Reader,
    ): Promise<void> {
        // leave the permissions the caller's stated sets decide on the object without its row
        const undecided = permissions.filter(
            (permission) => !this.#isStated(permission, target, access),
        );
        if (undecided.length === 0) {
            return;
        }

        // decide each permission by its tuple tree, through one reader of the object's grants
        const read = reader ?? this.reader(snapshot, access.scopes);
        for (const permission of undecided) {
            const decision = await this.check(snapshot, permission, target, access, read);
            if (!decision.isAllowed) {
                await this.#refuse(snapshot, permission, target, access);
            }
        }
    }

    /** Decide whether stated sets grant an ungated permission on an object without its row. */
    #isStated(permission: PermissionReference, target: ObjectReference, access: Access): boolean {
        // decide declared permissions alone, leaving others to refuse on the row
        const { permissions } = this.policy(permission).definition;

        return (
            Object.hasOwn(permissions, permission.name) &&
            access.allows(permission, target) &&
            access.blocked(permission) === undefined &&
            Authorizer.#states(this.expression(permission), target, access.authority)
        );
    }

    /** Decide whether an expression holds on an object by the subject sets an authority states. */
    static #states(
        expression: AccessExpression,
        target: ObjectReference,
        authority: Authority,
    ): boolean {
        // hold a relation the authority states on the object, any or every branch of a set
        switch (expression.kind) {
            case "relation":
                return authority.isMember({ ...target, relation: expression.name });
            case "union":
                return expression.expressions.some((each) =>
                    Authorizer.#states(each, target, authority),
                );
            case "intersection":
                return expression.expressions.every((each) =>
                    Authorizer.#states(each, target, authority),
                );
            case "contained":
            case "context":
            case "exclusion":
            case "granters":
            case "none":
            case "permission":
            case "readers":
            case "resource":
            case "self":
            case "subjectSet":
            case "through":
                return false;
        }
    }

    /** Find the lowest fresh assurance level allowing a refused caller, with elevation age. */
    async challenge(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        allows: (stepped: Access) => Promise<boolean>,
    ): Promise<StepUp | undefined> {
        // start from the caller's level and the elevation's, whichever is higher
        const context = access.context;
        const elevation = this.elevated.get(PermissionReference.key(permission));
        const lowest = Math.max(
            AccessContext.assurance(context)?.level ?? 1,
            elevation?.assurance ?? 1,
        );

        // decide again at each level as if the caller authenticated just now
        for (let level = lowest; level <= HIGHEST_ASSURANCE; level++) {
            const assurance = { level, authenticatedAt: context.now };
            let credential: Credential | undefined;
            // step up a caller's own credential
            if (context.credential !== undefined) {
                credential = { ...context.credential, assurance };
            }
            // step up code acting for a person as the person's approval of this permission
            else if (Caller.isDelegated(context)) {
                const approval = PermissionReference.key(permission);
                credential = Credential.approval(
                    approval,
                    assurance,
                    permission.packageId,
                    context.now,
                );
            }
            const stepped = await this.resolve(snapshot, access.scope, {
                ...context,
                ...(credential === undefined ? {} : { credential }),
            });
            if (await allows(stepped)) {
                return {
                    assurance: level,
                    ...(elevation === undefined ? {} : { maxAge: elevation.maxAge }),
                };
            }
        }

        return undefined;
    }

    /** Refuse a denied caller for its gate: a step-up it can take, or its suspended scope. */
    async refusal(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        allows: (access: Access) => Promise<boolean>,
    ): Promise<Error | undefined> {
        // challenge for step-up authentication first
        const stepUp = await this.challenge(snapshot, permission, access, allows);
        if (stepUp !== undefined) {
            return AccessError.of(
                "ASSURANCE_UNAUTHENTICATED",
                "authentication required: authentication assurance is too low",
                { data: stepUp },
            );
        }
        // refuse a caller its suspended scope alone denies
        else if (access.isSuspended && (await allows(access.active()))) {
            return AccessError.of(
                "SCOPE_DENIED",
                `scope '${access.scope}' is forbidden: it is suspended`,
            );
        }

        return undefined;
    }

    /** Read the fewest approvals letting a denied caller use a permission, absent for none. */
    async requiredApprovals(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        allows: (approved: Access) => Promise<boolean>,
    ): Promise<number | undefined> {
        // decide again at each count the scope's roles give the permission at, fewest first
        for (const approvals of access.waiting(permission)) {
            const { scope, context } = access;
            const approved = await this.resolve(snapshot, scope, context, undefined, approvals);
            if (await allows(approved)) {
                return approvals;
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
            let frontier = Tree.flatten(tree).map((grant) => grant.subject);
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
        // find the scope's object on the access's chain, which lives in its enclosing scope
        const scoped = access.scopes.find((link) => link.id === scope);
        if (scoped === undefined) {
            throw AccessError.of("ACCESS_BROKEN", `access is not resolved in '${scope}'`);
        }

        // list the subjects the authority matches in the scope and its enclosing scope
        const wanted = new Map<string, readonly string[]>();
        const subjects: readonly Subject[] = [
            ...access.authority.subjects,
            anyone.reference("*", "*"),
        ];
        for (const objectScope of new Set([scope, scoped.scope])) {
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

        // select a relationship on an object in the scope or on the scope itself, whose subject and conditions pass
        return rows.some(
            (row) =>
                (row.objectScope === scope ||
                    (row.objectId === scoped.id &&
                        row.objectPackageId === scoped.packageId &&
                        row.objectType === scoped.type)) &&
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

        return Tree.permits(tree, access);
    }

    /** Check a permission on each row, returning the caller's permitted positions and expiry. */
    async checkRows(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        rows: readonly Row[],
        reader = this.reader(snapshot, access.scopes),
        below: ReadonlyMap<string, Access> = new Map(),
    ): Promise<PermittedRows> {
        // decide each row by the caller's access in the row's scope
        const mapping = this.requireMapping(permission);
        const accesses = rows.map((row) => below.get(TableMapping.scope(mapping, row)) ?? access);
        for (const enclosed of below.values()) {
            reader.record(enclosed.scopes);
        }

        // read the rows' grant trees
        const decided = zip(zip(rows, accesses), await reader.trees(permission, rows));
        const until = earliest([
            access.until,
            ...decided.map(([[, evaluated], tree]) =>
                earliest([evaluated.until, Authorizer.#grantsUntil(Tree.flatten(tree), evaluated)]),
            ),
        ]);

        // allow the rows of the caller's chain that pass the gate with a tree allowing the caller
        const gates = await Promise.all(
            decided.map(([[row, evaluated]]) =>
                this.#gate(snapshot, permission, mapping, row, evaluated),
            ),
        );
        const permitted = decided.flatMap(([[row, evaluated], tree], position) =>
            gates[position] === undefined &&
            Authorizer.#isListed(mapping, row, evaluated) &&
            Tree.permits(tree, evaluated)
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
            throw AccessError.of(
                "PERMISSION_TYPE_INVALID",
                "permission is invalid: it must belong to the object's type",
            );
        }
        const mapping = this.requireMapping(target);
        const reader = this.reader(snapshot, access.scopes);
        const found = await reader.row(mapping, target);
        if (found === undefined) {
            throw AccessError.of("OBJECT_NOT_FOUND", `${target.type} '${target.id}' not found`);
        }

        // decide the authority by the permission's tree, and by each tuple alone
        const blocked = await this.#gate(snapshot, permission, mapping, found, access);
        const tree = await reader.tree(permission, found);
        const tuples = Tree.flatten(tree).flatMap((tuple) => {
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
            isAllowed: blocked === undefined && Tree.permits(tree, access),
            ...(blocked === undefined ? {} : { gate: blocked }),
            tuples,
        };
    }

    /** Refuse a caller a permission on one object, with a step-up challenge when one allows it. */
    async #refuse(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
    ): Promise<never> {
        const refused = await this.refusal(
            snapshot,
            permission,
            access,
            async (each) => (await this.check(snapshot, permission, target, each)).isAllowed,
        );
        throw (
            refused ??
            AccessError.of(
                "PERMISSION_DENIED",
                `permission '${permission.name}' is forbidden: the caller lacks it`,
            )
        );
    }

    /** Read the gate a request fails on a row before any grant. */
    async #gate(
        snapshot: Snapshot,
        permission: PermissionReference,
        mapping: TableMapping,
        row: Row,
        access: Access,
    ): Promise<ReturnType<Access["gate"]>> {
        // refuse a request that its credential, elevation or suspension refuses
        const related =
            access.permissions === undefined
                ? undefined
                : await this.#derived(snapshot, permission, mapping, row);

        return access.gate(permission, mapping, row, related);
    }

    /** Report whether a listed row is the mapped type's, in the caller's scope chain. */
    static #isListed(mapping: TableMapping, row: Row, access: Access): boolean {
        // require the row's scope on the chain, and the mapped type in a shared table
        const definition = mapping.policy.definition;
        const scope = TableMapping.scope(mapping, row);
        const isChained =
            scope === access.scope || access.scopes.some((object) => object.scope === scope);
        const isTyped =
            mapping.isShared !== true ||
            (row["packageId"] === definition.packageId && row["type"] === definition.name);

        return isChained && isTyped;
    }

    /** Read the objects a permission derives from through field relations. */
    async #derived(
        snapshot: Snapshot,
        permission: PermissionReference,
        mapping: TableMapping,
        row: Row,
    ): Promise<Related> {
        // walk the permission's sources across objects, each permission on each object once
        const related = new Map<string, Map<string, ObjectReference>>();
        const visited = new Set<string>();
        const pending = [{ permission, mapping, row }];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const id = TableMapping.text(next.row, next.mapping.id);
            const visit = `${id} ${next.permission.name}`;
            if (!visited.has(visit)) {
                visited.add(visit);
                for (const source of this.sources(next.permission)) {
                    pending.push(...(await this.#followed(snapshot, next, source, related)));
                }
            }
        }

        return related;
    }

    /** Follow one source of a permission on a row, to the same row or a related one. */
    async #followed(
        snapshot: Snapshot,
        from: {
            readonly permission: PermissionReference;
            readonly mapping: TableMapping;
            readonly row: Row;
        },
        source: { readonly permission: string; readonly relation?: string },
        related: Map<string, Map<string, ObjectReference>>,
    ): Promise<{ permission: PermissionReference; mapping: TableMapping; row: Row }[]> {
        // follow a source on the same row
        if (source.relation === undefined) {
            return [{ ...from, permission: { ...from.permission, name: source.permission } }];
        }

        // record the object the relation refers to, none for an empty field
        const target = await this.related(snapshot, from.mapping, source.relation, from.row);
        if (target === undefined) {
            return [];
        }
        const id = TableMapping.text(from.row, from.mapping.id);
        const relations = related.get(id) ?? new Map<string, ObjectReference>();
        relations.set(source.relation, target);
        related.set(id, relations);

        // read the target's row where its permission derives further
        const derived = { packageId: target.packageId, type: target.type, name: source.permission };
        const mapping = this.mapping(derived);
        if (mapping === undefined || this.sources(derived).length === 0) {
            return [];
        }
        const [found] = await TableMapping.read(snapshot, mapping, target.scope, [target.id]);

        return found === undefined ? [] : [{ permission: derived, mapping, row: found }];
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
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `missing tree mapping: '${expression.relation}'`,
                );
            } else if (
                (expression.kind === "granters" || expression.kind === "readers") &&
                mapping.references?.[expression.reference] === undefined
            ) {
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `unmapped reference: '${expression.reference}'`,
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `relation '${type.definition.name}#${name}' needs exactly one subject type`,
            );
        }

        return only;
    }

    /** Require a scope relation to accept plain scope types only, stored outside fields. */
    #validateScopeRelation(type: Policy, name: string, relation: RelationDescription): void {
        // require plain scope types, without relations or wildcards
        const isScopeTypes = relation.subjects.every(
            (scope) =>
                scope.relation === undefined &&
                scope.wildcard !== true &&
                this.policy(scope).definition.scope === true,
        );
        if (!isScopeTypes) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `'${type.name}.${name}' is a scope relation: it accepts scope types only`,
            );
        }
        if (
            this.mapping({ packageId: type.definition.packageId, type: type.name })?.relations[name]
        ) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `'${type.name}.${name}' is a scope relation: a field cannot store it`,
            );
        }
    }

    /** Require a subject type to refer to a registered type and one of its relations. */
    #validateSubject(subject: SubjectType): void {
        // resolve the relation of a subject set
        const type = this.policy(subject);
        if (subject.relation !== undefined) {
            if (subject.wildcard) {
                throw AccessError.of("DECLARATION_BROKEN", "a subject set cannot be a wildcard");
            }
            // accept a scope's contents, a relation, or a permission the object's relations decide
            if (subject.relation === CONTAINED) {
                if (type.definition.scope !== true) {
                    throw AccessError.of(
                        "DECLARATION_BROKEN",
                        `${type.name} is no scope: it contains nothing`,
                    );
                }
            } else if (!Object.hasOwn(type.definition.relations, subject.relation)) {
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `'${reference.type}.${reference.name}' cannot use '${name}': it is a '${isScope ? "scope" : "field"}' relation`,
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
            // skip a named permission followed before, and a subject set, which grants the scope's own row alone
            else if (expression.kind === "permission" || expression.kind === "subjectSet") {
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
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `'${reference.type}.${reference.name}' is a chain permission: it can use relations only`,
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

    /** Resolve all expressions, recursing only in explicit ancestor traversal. */
    #validate(reference: PermissionReference, parents: ReadonlySet<string>): void {
        // detect recursion by the complete permission reference
        const key = PermissionReference.key(reference);
        if (parents.has(key)) {
            throw AccessError.of("DECLARATION_BROKEN", `cyclic permission: '${reference.name}'`);
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
                case "subjectSet":
                    this.relation(this.policy(expression.object), expression.relation);
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
                    this.#registerThrough(type, expression, path);
                    break;
            }
        }
    }

    /** Require a type allowing itself to be a principal type. */
    static #validateSelf(type: Policy): void {
        const { packageId, name } = type.definition;
        if (!TableMapping.isPrincipalType({ packageId, type: name })) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `'${name}' is not a principal, so self allows no one`,
            );
        }
    }

    /** Require an arrow to lead to plain types with the permission, and record its relation. */
    #registerThrough(
        type: Policy,
        expression: Extract<AccessExpression, { kind: "through" }>,
        path: ReadonlySet<string>,
    ): void {
        // follow the relation to plain objects of types declaring the permission
        const definition = type.definition;
        const relation = this.relation(type, expression.relation);
        this.#arrows.add(
            PermissionReference.key({
                packageId: definition.packageId,
                type: definition.name,
                name: expression.relation,
            }),
        );
        for (const subject of this.scopeTypes(type, expression.relation, expression.permission)) {
            // record the type the arrow leads to
            const key = Authorizer.#typeKey(type.typeReference);
            const reached = this.#reached.get(key) ?? [];
            this.#reached.set(key, [
                ...reached,
                { packageId: subject.packageId, type: subject.type },
            ]);

            // refuse an arrow to a subject set or a wildcard
            if (subject.relation !== undefined || subject.wildcard) {
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `an arrow requires plain object subjects: '${expression.relation}'`,
                );
            }

            // require ancestor traversal to stay within a single object type
            if (
                expression.transitive &&
                (relation.subjects.length !== 1 ||
                    subject.packageId !== definition.packageId ||
                    subject.type !== definition.name)
            ) {
                throw AccessError.of(
                    "DECLARATION_BROKEN",
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `policy conditions cannot follow relations: '${relation.via}'`,
            );
        } else if (relation !== undefined) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `undeclared '${isObject ? "object" : "request"}' attribute: '${relation.via}'`,
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
                    throw AccessError.of(
                        "DECLARATION_BROKEN",
                        "pattern applies to text attributes only",
                    );
                }
                break;
            case "characters":
                // require a text attribute for a character set
                if (Authorizer.#attributeType(predicate.name, declared) !== "string") {
                    throw AccessError.of(
                        "DECLARATION_BROKEN",
                        "character set applies to text attributes only",
                    );
                }
                break;
            case "length":
                // require a text attribute, and compare its length as a number
                if (Authorizer.#attributeType(predicate.name, declared) !== "string") {
                    throw AccessError.of(
                        "DECLARATION_BROKEN",
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
                throw AccessError.of(
                    "DECLARATION_BROKEN",
                    `policy conditions cannot follow relations: '${predicate.via}'`,
                );
            case "search":
                throw AccessError.of("DECLARATION_BROKEN", "policy conditions cannot search");
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `ordered comparisons require a number attribute: '${predicate.name}'`,
            );
        }
        // compare only values of the attribute's type
        else if (valueType !== attribute) {
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `comparisons require a '${attribute}' value: '${predicate.name}'`,
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `listed values require '${attribute}' values: '${predicate.name}'`,
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
            throw AccessError.of("DECLARATION_BROKEN", `undeclared attribute: '${name}'`);
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `request conditions cannot read placeholders: '${name}'`,
            );
        } else if (!Object.hasOwn(placeholders, name)) {
            throw AccessError.of("DECLARATION_BROKEN", `undeclared request attribute: '${name}'`);
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

            // take one policy per type, a representing one over the intrinsic one
            const existing = byType.get(Authorizer.#policyKey(type));
            if (existing === undefined || INTRINSIC_POLICIES.includes(existing)) {
                byType.set(Authorizer.#policyKey(type), type);
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
            throw AccessError.of(
                "DECLARATION_BROKEN",
                `subject set '${mapping.policy.definition.name}#${relation}' needs an index on ${field.column}`,
            );
        }
    }

    /** Collect the keys of the permissions policies list as reserved, administrative or moving. */
    static #listedPermissions(
        types: readonly Policy[],
        list: "reserved" | "suspended" | "moving",
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

    /** Decide whether two policies declare one type: one key, both intrinsic or both declared. */
    static #isSameType(left: Policy, right: Policy): boolean {
        return (
            left === right ||
            (Authorizer.#policyKey(left) === Authorizer.#policyKey(right) &&
                INTRINSIC_POLICIES.includes(left) === INTRINSIC_POLICIES.includes(right))
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

    /** Delete the rows whose cascading relation has a deleted subject, as ON DELETE CASCADE. */
    async cascade(transaction: Transaction, subject: Subject): Promise<void> {
        // find the field relations that cascade and accept the subject
        const cascading = [...this.#mappings.values()].flatMap((mapping) =>
            Object.entries(mapping.policy.definition.relations ?? {})
                .filter(([, relation]) => relation.onDelete === "cascade")
                .filter(([, relation]) => accepts(relation, subject))
                .map(([name]) => ({ mapping, name, column: mapping.relations[name] })),
        );

        // delete the rows with the subject: by its typed columns, else by its id within its type
        for (const { mapping, name, column } of cascading) {
            if (column === undefined) {
                throw new TypeError(`relation '${name}' can only cascade from a column`);
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
