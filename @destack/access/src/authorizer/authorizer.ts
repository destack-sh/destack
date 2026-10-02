import { v7 } from "uuid";
import {
    and,
    eq,
    or,
    sql,
    type DatabaseConnection,
    type SQL,
    type Table,
    Condition,
    Snapshot,
    TABLE,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { identifier, schema, zip } from "@destack/schema";
import {
    Replica,
    Scope,
    type ReplicaRequest,
    type ScopeLink,
    type Watch,
    type ObjectReference,
    ObjectTypeReference,
    Subject,
} from "@destack/sync";
import { AccessError } from "../error/index.ts";
import { relationsOf, PermissionReference, type Policy } from "../policy/policy.ts";
import type { AccessExpression } from "../policy/expression.ts";
import { accepts, type RelationDefinition, type SubjectType } from "../policy/subject.ts";
import { INTRINSIC_POLICIES } from "../policy/principal.ts";
import { type AccessContext } from "../context/context.ts";
import { HIGHEST_ASSURANCE, type Elevation, type StepUp } from "../context/elevation.ts";
import { accessRelationship, type RelationshipRow } from "../relationship/table.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { Creation } from "./authorization.ts";
import { accessRole } from "../role/table.ts";
import { accessTables, COPY_NAME, decisionTables } from "../replica/replica.ts";
import { earliest, Access } from "./access.ts";
import { Compiler } from "./compiler.ts";
import type { Decision, Explanation } from "./decision.ts";
import { GrantReader, GrantTree, type Grant, type Lookup } from "./grant.ts";
import { column, TableMapping, type FieldRelation } from "./mapping.ts";

/** The identifier of a wildcard subject, which relates every identity of its type. */
const WILDCARD = "*";

/**
 * How long a copy of access may go without hearing from its home before decisions refuse it, by default, in milliseconds.
 *
 * Feeds repeat their position every ten seconds, and a copy three beats late has lost its source.
 */
const LAG_MILLISECONDS = 30_000;

/** The rows of a set a caller has a permission on, by position, and the next moment time alone may change that. */
export interface Admission {
    /** The positions of the rows the caller has the permission on. */
    readonly permitted: ReadonlySet<number>;
    /** The next moment time may change the rows the caller has it on. */
    readonly until?: number;
}

/** Decide policies over their objects' tables, in SQL or in memory from grants. */
export class Authorizer {
    /** The keys of permissions that only their expressions grant. */
    readonly reserved: ReadonlySet<string>;
    /** The authentication each elevated permission asks for, by permission key. */
    readonly elevated: ReadonlyMap<string, Elevation>;
    /** The keys of permissions that stay available while a scope is suspended. */
    readonly administration: ReadonlySet<string>;
    /** Field relations some relation names as a subject set. */
    readonly fields: FieldRelation[] = [];
    /** The subject sets that relations accept and roles may bind to. */
    readonly memberships: SubjectType[] = [];
    /** The permissions accepted as subject sets, by the type and relation of each relationship deciding them. */
    readonly #implied = new Map<string, string[]>();
    /** The permissions accepted as subject sets on enclosed scopes, by the type and permission of the enclosing scope's set. */
    readonly #enclosed = new Map<string, PermissionReference[]>();
    /** The scope types that take subject sets from the scopes enclosing them. */
    readonly #enclosedTypes = new Set<string>();
    /** The scope chain steps of permissions read through enclosing scopes, by permission key. */
    readonly #inherited = new Map<
        string,
        readonly {
            readonly type: Pick<PermissionReference, "packageId" | "type">;
            readonly relations: readonly string[];
        }[]
    >();
    /** How long a copy of access may go without its home before decisions refuse it, in milliseconds. */
    readonly lag: number;
    /** The object types whose access rows live in this database, rather than as copies. */
    readonly local: readonly ObjectTypeReference[];
    /** The object types this database keeps copies of from the scopes above it: inherited rows, and the scopes' own rows of the scope types it keeps a table of. */
    readonly copied: readonly ObjectTypeReference[];
    /** The types whose objects live in the universe, whose access rows stay in the global tier. */
    readonly universal: readonly ObjectTypeReference[];
    /** The policies indexed by package and type. */
    readonly #policies = new Map<string, Policy>();
    /** The subject types other types contribute to each open relation, by type and relation. */
    readonly #contributions = new Map<string, SubjectType[]>();
    /** The table mappings indexed by package and type. */
    readonly #mappings = new Map<string, TableMapping>();
    /** The compiler of the registered policies to SQL. */
    readonly #compiler: Compiler;

    /** Validate the policies, their referenced policies and their tables in this database. */
    constructor(
        policies: readonly Policy[],
        mappings: readonly TableMapping[] = [],
        options: {
            /** How long a copy of access may go without hearing from its home, in milliseconds. */
            readonly lag?: number;
            /** Decide whether the database keeps a table's rows as copies from their home, which writes their access rows. */
            readonly copies?: (table: Table) => boolean;
        } = {},
    ) {
        // include every policy the declared policies name
        const reachable = new Map<string, Policy>();
        const visited = new Set<Policy>();
        const pending = [...policies];
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            if (visited.has(type)) {
                continue;
            }
            visited.add(type);
            pending.push(...type.references);

            // keep one policy per type, preferring a representing policy over the intrinsic one
            const existing = reachable.get(policyKey(type));
            if (existing === undefined || INTRINSIC_POLICIES.includes(existing)) {
                reachable.set(policyKey(type), type);
            }
            // refuse two distinct policies for one type
            else if (!INTRINSIC_POLICIES.includes(type)) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `duplicate object type: ${policyKey(type)}`,
                );
            }
        }
        const types = [
            ...INTRINSIC_POLICIES.filter((type) => !reachable.has(policyKey(type))),
            ...reachable.values(),
        ];

        // register each policy under its type
        for (const type of types) {
            this.#policies.set(policyKey(type), type);
        }

        // add each contributing type to the open relation it names
        for (const type of types) {
            for (const entry of type.definition.contributes ?? []) {
                const target = this.#policies.get(ObjectTypeReference.key(entry));
                if (target?.definition.relations[entry.relation]?.open !== true) {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        `${type.name} contributes to ${entry.type}.${entry.relation}, which is not an open relation`,
                    );
                }
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

        // validate every named permission, including unused reserved, elevated and administration permissions
        for (const type of types) {
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

            // resolve the type's grant, reserved, elevated and administration permissions
            if (definition.grantedBy !== undefined) {
                this.expression(type.permission(definition.grantedBy));
            }
            for (const name of [
                ...(definition.reserved ?? []),
                ...Object.keys(definition.elevated ?? {}),
                ...(definition.administration ?? []),
            ]) {
                this.expression(type.permission(name));
            }

            // validate named permissions after resolving the declared relations
            for (const name of Object.keys(definition.permissions)) {
                this.#validate(type.permission(name), new Set());
            }
        }

        // index the permissions the policies list as reserved, elevated or administrative
        this.reserved = listedPermissions(types, "reserved");
        this.elevated = new Map(
            types.flatMap((type) =>
                Object.entries(type.definition.elevated ?? {}).map(([name, elevation]) => [
                    PermissionReference.key(type.permission(name)),
                    elevation,
                ]),
            ),
        );
        this.administration = listedPermissions(types, "administration");
        this.lag = options.lag ?? LAG_MILLISECONDS;
        this.#compiler = new Compiler(this);

        // map scope types this database has no table of onto the ancestry every database has
        const scopes = this.policies()
            .filter(
                (policy) =>
                    policy.definition.scope === true &&
                    !mappings.some((mapping) => mapping.policy === policy),
            )
            .map((policy): TableMapping => ({
                policy,
                table: Scope.table,
                id: "scope",
                scope: "parent",
                isShared: true,
                attributes: {},
                relations: {},
            }));

        // register each mapping once and validate its columns against its policy
        for (const source of [...mappings, ...scopes]) {
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
            const key = ObjectTypeReference.key({
                packageId: definition.packageId,
                type: definition.name,
            });
            if (this.#mappings.has(key)) {
                throw new AccessError("INVALID_DECLARATION", `duplicate database mapping: ${key}`);
            }
            TableMapping.validate(this, mapping);
            this.#requireCompilable(mapping);
            this.#mappings.set(key, mapping);
        }

        // require each relation to an enclosing scope to name one scope type and live in no field
        for (const type of types) {
            for (const [name, relation] of Object.entries(type.definition.relations)) {
                if (relation.isScope === true) {
                    this.#validateScopeRelation(type, name, relation);
                }
            }
        }

        // list the types whose access rows this database keeps, and the types it keeps copies of
        const copies = options.copies ?? (() => false);
        this.local = [...this.#mappings.values()]
            .filter((mapping) => !accessTables.includes(mapping.table) && !copies(mapping.table))
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
        this.copied = [...this.#mappings.values()]
            .filter(
                (mapping) =>
                    mapping.inherited !== undefined ||
                    (copies(mapping.table) &&
                        mapping.policy.definition.scope === true &&
                        mapping.table !== Scope.table),
            )
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
        this.universal = [...this.#policies.values()]
            .filter((policy) => policy.definition.isGlobal === true)
            .map((policy) => ({
                packageId: policy.definition.packageId,
                type: policy.definition.name,
            }));

        // expand the declared subject sets and every scope's relations, the sets roles may bind to
        const sets = new Map(
            this.policies()
                .flatMap((policy) => Object.values(policy.definition.relations))
                .flatMap((relation) => relation.subjects)
                .flatMap((subject): [string, PermissionReference][] => {
                    // keep the subjects naming a set
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
        const expandedMembers = new Map<string, PermissionReference>();
        for (const [key, set] of expanded) {
            const policy = this.policy(set);

            // expand a relation's set as it is
            if (Object.hasOwn(policy.definition.relations, set.name)) {
                expandedMembers.set(key, set);
            }
            // expand a permission's set through the relations deciding it on the object
            else {
                const { relations, enclosing } = this.deciding(policy.permission(set.name));
                for (const relation of relations) {
                    const deciding = { packageId: set.packageId, type: set.type, name: relation };
                    const decidingKey = PermissionReference.key(deciding);
                    expandedMembers.set(decidingKey, deciding);
                    this.#implied.set(decidingKey, [
                        ...(this.#implied.get(decidingKey) ?? []),
                        set.name,
                    ]);
                }

                // and through the same sets of the scopes enclosing it, expanded in turn
                for (const scope of enclosing) {
                    const scopeKey = PermissionReference.key(scope);
                    this.#enclosed.set(scopeKey, [...(this.#enclosed.get(scopeKey) ?? []), set]);
                    this.#enclosedTypes.add(ObjectTypeReference.key(set));
                    expanded.set(scopeKey, scope);
                }
            }
        }
        for (const member of expandedMembers.values()) {
            this.memberships.push({
                packageId: member.packageId,
                type: member.type,
                relation: member.name,
            });
        }

        // expand subject sets through the fields that keep them
        for (const mapping of this.#mappings.values()) {
            const definition = mapping.policy.definition;
            for (const [relation, field] of Object.entries(mapping.relations)) {
                if (
                    sets.has(
                        PermissionReference.key({
                            packageId: definition.packageId,
                            type: definition.name,
                            name: relation,
                        }),
                    )
                ) {
                    const subjectColumn = column(mapping.table, field.column);
                    const isIndexed = mapping.table[TABLE]
                        .constraints("sqlite")
                        .some(
                            (constraint) =>
                                (constraint.kind === "index" || constraint.kind === "unique") &&
                                constraint.columns[0] === subjectColumn,
                        );
                    if (!isIndexed && subjectColumn.definition.primaryKey !== true) {
                        throw new AccessError(
                            "INVALID_DECLARATION",
                            `subject set ${definition.name}#${relation} needs an index on ${field.column}`,
                        );
                    }
                    this.fields.push({
                        mapping,
                        relation,
                        field,
                        subject: this.onlySubject(mapping.policy, relation),
                    });
                }
            }
        }

        // require every declared relation of a mapped type to decide a permission
        const parents = new Set(
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
        const intrinsic = new Set(INTRINSIC_POLICIES.map((policy) => policyKey(policy)));
        for (const type of types.filter(
            (policy) => !intrinsic.has(policyKey(policy)) && this.#mappings.has(policyKey(policy)),
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
                        `relation ${definition.name}.${name} decides no permission`,
                    );
                }
            }
        }
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
    relation(type: Policy, name: string): RelationDefinition {
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

    /** Return the table mapping of an object type, failing for a type this database maps no table of. */
    mapping(reference: Pick<PermissionReference, "packageId" | "type">): TableMapping {
        const mapping = this.mappingOf(reference);
        if (!mapping) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `missing database mapping: ${reference.type}`,
            );
        }

        return mapping;
    }

    /** Read the table mapping of an object type, absent for a type this database maps no table of. */
    mappingOf(
        reference: Pick<PermissionReference, "packageId" | "type">,
    ): TableMapping | undefined {
        return this.#mappings.get(ObjectTypeReference.key(reference));
    }

    /** Return the table mappings in registration order. */
    mappings(): TableMapping[] {
        return [...this.#mappings.values()];
    }

    /** Read what a permission of a scope type grants through a scope chain: each type along the chain, and the relations on its object that grant it. */
    inherited(reference: PermissionReference): readonly {
        readonly type: Pick<PermissionReference, "packageId" | "type">;
        readonly relations: readonly string[];
    }[] {
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
    #inherit(
        reference: PermissionReference,
        visited: ReadonlySet<string>,
    ): {
        readonly type: Pick<PermissionReference, "packageId" | "type">;
        readonly relations: readonly string[];
    }[] {
        // refuse a permission the walk reached before
        const key = PermissionReference.key(reference);
        if (visited.has(key)) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${reference.type}.${reference.name} encloses itself`,
            );
        }
        const { relations, enclosing } = this.deciding(reference);
        const reached = new Set([...visited, key]);

        return [
            { type: { packageId: reference.packageId, type: reference.type }, relations },
            ...enclosing.flatMap((permission) => this.#inherit(permission, reached)),
        ];
    }

    /** Read the permission another purely derives from, on its object or through a field relation. */
    source(
        permission: PermissionReference,
    ): { readonly permission: string; readonly relation?: string } | undefined {
        const expression = this.expression(permission);
        if (expression.kind === "permission") {
            return { permission: expression.name };
        } else if (
            expression.kind === "through" &&
            !expression.transitive &&
            this.mappingOf(permission)?.relations[expression.relation] !== undefined
        ) {
            return { permission: expression.permission, relation: expression.relation };
        }

        return undefined;
    }

    /** Read the object a row's field relation names, absent when the row names none. */
    related(
        mapping: TableMapping,
        relation: string,
        row: Readonly<Record<string, unknown>>,
    ): ObjectReference | undefined {
        // read the related identifier
        const field = mapping.relations[relation];
        if (field === undefined) {
            throw new AccessError("INVALID_DECLARATION", `${relation} is no field relation`);
        }
        const id = row[field.column];
        if (id === null || id === undefined) {
            return undefined;
        } else if (typeof id !== "string") {
            throw new TypeError(`field ${field.column} keeps no identifier`);
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

    /** Read the scope with the chain that decides access to an object: a scope object itself, else its containing scope. */
    governingScope(target: ObjectReference): string {
        return this.policy(target).definition.scope === true ? target.id : target.scope;
    }

    /**
     * Decide whether this database keeps an object's access rows: those of objects of a type it maps a table of.
     *
     * A role, and so its inclusions, lives with the scope object defining it.
     */
    async isLocal(database: DatabaseConnection, object: ObjectReference): Promise<boolean> {
        // follow a role to the scope object defining it
        if (this.mappingOf(object)?.table === accessRole) {
            const scope = await Scope.object(Snapshot.live(database), object.scope);

            return this.isLocal(database, scope);
        }

        return this.local.some(
            (type) => type.packageId === object.packageId && type.type === object.type,
        );
    }

    /** Refuse writing the access rows of an object another database keeps. */
    async requireLocal(database: DatabaseConnection, object: ObjectReference): Promise<void> {
        if (!(await this.isLocal(database, object))) {
            throw new AccessError(
                "FORBIDDEN",
                `access of ${object.type} ${object.id} is written in the database keeping it`,
            );
        }
    }

    /** List the requests of the copies of the scopes above one, and of the scope itself unless the database is its home. */
    async chain(
        database: DatabaseConnection,
        below: string,
        options: { readonly isHome: boolean },
    ): Promise<Omit<ReplicaRequest, "after">[]> {
        // read the ancestors the scope's copy lists, the scope alone until it arrives
        const [copy] = await database
            .select({ ancestors: Scope.table.ancestors })
            .from(Scope.table)
            .where(eq(Scope.table.scope, below));
        const scopes = [
            ...(options.isHome ? [] : [below]),
            ...(copy === undefined ? [] : [...copy.ancestors, Scope.universe.id]),
        ];

        // copy each scope's access rows, inherited rows and own object, decided for the scope below
        return scopes.map((scope) => ({
            name: COPY_NAME,
            scope,
            below,
            access: true,
            local: [...this.local],
            copied: [...this.copied],
            rows: [],
        }));
    }

    /** Build the copy a request asks for: a scope's access rows, inherited rows and global rows. */
    replicaOf(request: Omit<ReplicaRequest, "after">): Replica {
        // leave out the access rows of the objects the follower keeps, and of objects living in the universe
        const own = Condition.not(
            Condition.any(
                ...[...request.local, ...this.universal].map((type) =>
                    Condition.all(
                        Condition.eq("packageId", type.packageId),
                        Condition.eq("type", type.type),
                    ),
                ),
            ),
        );
        const access: [Table, Condition | undefined][] = request.access
            ? decisionTables.map((table) => [table, table === accessRelationship ? own : undefined])
            : [];

        // add each copied type's inherited rows, or the scope's own row when its identifier is of a copied scope type
        const copied = request.copied.flatMap((type): [Table, Condition][] => {
            // copy an inherited type's inherited rows
            const mapping = this.mapping(type);
            const table = this.#copiedTable(type);
            if (mapping.inherited !== undefined) {
                return [[table, mapping.inherited]];
            } else if (mapping.policy.definition.scope !== true) {
                throw new AccessError("NOT_FOUND", `no inherited rows of ${type.type}`);
            }
            const scopeIdentifier = column(table, mapping.id).definition.schema;

            return scopeIdentifier.safeParse(request.scope).success
                ? [[table, Condition.eq(mapping.id, request.scope)]]
                : [];
        });
        const owned = new Set(
            request.copied
                .filter((type) => this.mapping(type).inherited === undefined)
                .map((type) => this.#copiedTable(type)),
        );

        // add each requested type's rows
        const rows = request.rows.map(({ type, where }): [Table, Condition] => [
            this.#copiedTable(type),
            where,
        ]);

        // copy across scopes the rows living in the scopes of other requested types' rows
        const within = request.rows.flatMap(({ type, within: parents }): [Table, Table[]][] =>
            parents === undefined
                ? []
                : [[this.#copiedTable(type), parents.map((parent) => this.#copiedTable(parent))]],
        );

        // copy them under the request's name and scope
        const tables = [...access, ...copied, ...rows];

        return new Replica({
            name: request.name,
            scope: request.scope,
            tables: tables.map(([table]) => table),
            where: new Map(
                tables.flatMap(([table, where]): [Table, Condition][] =>
                    where === undefined ? [] : [[table, where]],
                ),
            ),
            within: new Map(within),
            everywhere: owned,
        });
    }

    /** Read the table of an object type a copy asks for, refusing a type this database keeps no table of. */
    #copiedTable(type: ObjectTypeReference): Table {
        const mapping = this.#mappings.get(ObjectTypeReference.key(type));
        if (mapping === undefined || mapping.table === Scope.table) {
            throw new AccessError("NOT_FOUND", `no object type ${type.type} to copy`);
        }

        return mapping.table;
    }

    /**
     * List the access rows with changes that affect what a caller of a scope chain may have.
     *
     * A scope object's relationships live in that scope, so the relationships of a caller's subjects are watched in every scope.
     */
    watch(chain: readonly string[], subjects: readonly Subject[] = []): Watch[] {
        const ids = [...new Set(subjects.map((subject) => subject.id))];

        return [
            ...decisionTables.map((table): Watch => ({ table, scopes: chain })),
            ...(ids.length === 0
                ? []
                : [
                      {
                          table: accessRelationship,
                          scopes: "every" as const,
                          where: Condition.oneOf("subjectId", [...ids, WILDCARD]),
                      },
                  ]),
        ];
    }

    /** Read a page of one object's relationships and role bindings as a snapshot shows them, ordered by identifier. */
    async relationships(
        snapshot: Snapshot,
        object: ObjectReference,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<Relationship[]> {
        const rows = await snapshot.ordered(accessRelationship, {
            where: Condition.all(
                Condition.eq("objectScope", object.scope),
                Condition.eq("packageId", object.packageId),
                Condition.eq("type", object.type),
                Condition.eq("objectId", object.id),
            ),
            order: [{ column: "id", direction: "asc" }],
            ...(page.after === undefined
                ? {}
                : { after: { id: identifier("relationship").parse(page.after) } }),
            count: page.limit,
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
                managerInstallationId: null,
                managerPackageId: null,
                managerName: null,
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
            throw new AccessError("FORBIDDEN", `relation ${request.relation} rejects the subject`);
        }
        // require a role's subject to be named, since a wildcard would bind the role to everyone
        else if (isRole && subject !== undefined && (subject.id === "*" || subject.scope === "*")) {
            throw new AccessError("FORBIDDEN", "a role binds only to named subjects");
        }
        // require a role's subject set to be one resolving a caller expands
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
                `a role binds only to declared subject sets and scope relations, not ${subject.type}#${subject.relation}`,
            );
        }
        // require a future expiry
        else if (expiresAt !== null && (!Number.isFinite(expiresAt) || expiresAt <= now)) {
            throw new AccessError("FORBIDDEN", "a relationship expires in the future");
        }
    }

    /** Remove the relationships bound to one request after it commits. */
    async release(database: DatabaseConnection, requestId: string): Promise<void> {
        // delete the request's relationships about the types this database keeps
        const types = this.local.map((type) =>
            and(
                eq(accessRelationship.packageId, type.packageId),
                eq(accessRelationship.type, type.type),
            ),
        );
        if (types.length > 0) {
            await database
                .delete(accessRelationship)
                .where(and(eq(accessRelationship.requestId, requestId), or(...types)));
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

    /** Resolve a caller in a scope inside its transaction: its subject sets, the scope chain and the roles along it. */
    resolve(
        snapshot: Snapshot,
        scope: string,
        context: AccessContext,
        links?: readonly ScopeLink[],
    ): Promise<Access> {
        return Access.resolve(snapshot, scope, context, this, links);
    }

    /** Restrict the rows of a table to those the caller has a permission on, before sorting, counting, pagination, or mutation. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        return this.#compiler.where(permission, access, source);
    }

    /** Match one object, in its own scope, if the caller has a permission on it or through a role bound on or above it. */
    permits(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        return this.#compiler.permits(permission, target, access);
    }

    /** Start reading the grants of rows within a scope chain, as a snapshot shows them, shared by every caller of the scope. */
    reader(snapshot: Snapshot, scopes: readonly ObjectReference[]): GrantReader {
        return new GrantReader(this, snapshot, scopes);
    }

    /** Decide whether a caller has a permission on one object, and until when that stays true by time alone. */
    async check(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
        reader?: GrantReader,
    ): Promise<Decision> {
        // deny a missing object
        const mapping = this.mapping(target);
        const reading = reader ?? this.reader(snapshot, access.scopes);
        const row = await reading.row(mapping, target);
        if (row === undefined) {
            return { isAllowed: false };
        }

        // read the tree the permission reaches on the object
        const tree = await reading.tree(permission, row, mapping);
        const until = earliest([access.until, grantsUntil(GrantTree.flatten(tree), access)]);

        // admit the caller when every authority has the permission, past its gate for one object
        const isAllowed =
            this.#gate(permission, mapping, row, access, "object") === undefined &&
            access.authorities.every((authority) => GrantTree.permits(tree, authority, access));

        return { isAllowed, ...(until === undefined ? {} : { until }) };
    }

    /**
     * Require the caller to have every permission on one object, naming the first it lacks.
     *
     * A live networked database decides in one statement, and an embedded database or a past snapshot decides from grant trees.
     */
    async require(
        snapshot: Snapshot,
        permissions: readonly PermissionReference[],
        target: ObjectReference,
        access: Access,
        reader?: GrantReader,
    ): Promise<void> {
        // decide in memory where reads are cheap or the database is past
        const database = snapshot.database;
        if (snapshot.position !== undefined || database.state.locality === "embedded") {
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

    /**
     * Find the fresh authentication that would admit a refused caller: the lowest assurance level at which a decision passes, with the permission's elevation age.
     *
     * Refusals stronger authentication would not lift, such as missing grants, find none.
     */
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

    /**
     * List a page of the principals of a type with a permission on one object now, in subject key order, through subject sets.
     *
     * Each candidate is decided as a check decides it, authenticated as strongly as the permission asks.
     * A wildcard of a whole type names no principals, so the principals it admits are not listed.
     */
    async subjects(
        snapshot: Snapshot,
        permission: PermissionReference,
        target: ObjectReference,
        type: ObjectTypeReference,
        now: number,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<Subject[]> {
        // read the grants the permission reaches on the object, through one reader of its scope chain
        const scope = this.governingScope(target);
        const links = await Scope.chain(snapshot, scope);
        const reader = this.reader(
            snapshot,
            links.map((link) => link.object),
        );
        const row = await reader.row(this.mapping(target), target);
        if (row === undefined) {
            return [];
        }
        const tree = await reader.tree(permission, row, this.mapping(target));

        // expand the grants' subject sets into the principals of the type
        const candidates = new Map<string, Subject>();
        const seen = new Set<string>();
        for (
            let frontier = GrantTree.flatten(tree).map((grant) => grant.subject);
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
            frontier = await members(snapshot, sets);
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

    /** Decide whether every authority of the caller owns one object. */
    async owns(snapshot: Snapshot, target: ObjectReference, access: Access): Promise<boolean> {
        // own no missing object
        const mapping = this.mapping(target);
        const reader = this.reader(snapshot, access.scopes);
        const row = await reader.row(mapping, target);
        if (row === undefined) {
            return false;
        }

        // read the owner bindings on or above the object
        const tree = await reader.ownership(row, mapping);

        return access.authorities.every((authority) => GrantTree.permits(tree, authority, access));
    }

    /**
     * Check a permission on each of the given rows, as they are or were: the positions the caller has it on, and until when that stays true by time alone.
     *
     * Every permission decides in memory from the grant trees the reader shares among callers, as the compiled predicate decides in SQL.
     * A row of a scope the caller's scope encloses is decided by the caller's access in that scope, given in `below`.
     */
    async checkRows(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        rows: readonly Readonly<Record<string, unknown>>[],
        reader = this.reader(snapshot, access.scopes),
        below: ReadonlyMap<string, Access> = new Map(),
    ): Promise<Admission> {
        // decide each row by the caller's access in the row's scope
        const mapping = this.mapping(permission);
        const accesses = rows.map((row) => below.get(TableMapping.scope(mapping, row)) ?? access);
        for (const enclosed of below.values()) {
            reader.cover(enclosed.scopes);
        }

        // read the rows' grant trees
        const decided = zip(zip(rows, accesses), await reader.trees(permission, rows));
        const until = earliest([
            access.until,
            ...decided.map(([[, evaluated], tree]) =>
                earliest([evaluated.until, grantsUntil(GrantTree.flatten(tree), evaluated)]),
            ),
        ]);

        // admit the rows with a tree that admits every authority of the caller
        const permitted = decided.flatMap(([[row, evaluated], tree], position) =>
            this.#gate(permission, mapping, row, evaluated, "listing") === undefined &&
            evaluated.authorities.every((authority) =>
                GrantTree.permits(tree, authority, evaluated),
            )
                ? [position]
                : [],
        );

        return { permitted: new Set(permitted), ...(until === undefined ? {} : { until }) };
    }

    /** Explain whether a caller has a permission on one object: the gate, then each grant and why it fails, per authority. */
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
                "explain a permission of the object's own type",
            );
        }
        const mapping = this.mapping(target);
        const reader = this.reader(snapshot, access.scopes);
        const found = await reader.row(mapping, target);
        if (found === undefined) {
            throw new AccessError("NOT_FOUND", `no ${target.type} ${target.id}`);
        }

        // decide each authority by the permission's tree, and each grant it reaches alone
        const blocked = this.#gate(permission, mapping, found, access, "object");
        const tree = await reader.tree(permission, found);
        const grants = GrantTree.flatten(tree);
        const decided = access.authorities.map((authority) => ({
            ...authority.delegation,
            isAllowed: GrantTree.permits(tree, authority, access),
            grants: grants.map((grant) => {
                const failed = authority.failure(grant, access);

                return {
                    path: [...grant.path],
                    object: grant.object,
                    subject: grant.subject,
                    ...(grant.role === undefined ? {} : { role: grant.role.id }),
                    ...(failed === undefined ? {} : { failure: failed }),
                };
            }),
        }));

        return {
            permission,
            object: target,
            isAllowed: blocked === undefined && decided.every((entry) => entry.isAllowed),
            ...(blocked === undefined ? {} : { gate: blocked }),
            authorities: decided,
        };
    }

    /** Refuse a caller a permission on one object, challenging it for the authentication that would admit it. */
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
                "authenticate again at the required assurance",
                { stepUp },
            );
        }
        throw new AccessError("FORBIDDEN", `permission denied: ${permission.name}`);
    }

    /** Read the gate a request fails on a row before any grant: its credential, elevation or suspension, or a listed row outside the scope. */
    #gate(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        access: Access,
        lookup: Lookup,
    ): ReturnType<Access["gate"]> | "outside" {
        // refuse the request its credential, elevation or suspension refuses
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
        const own = access.own(mapping);
        const isOwn =
            own !== undefined && scope === own.scope && String(row[mapping.id]) === own.id;

        return (scope !== access.scope && !isOwn) ||
            (mapping.isShared === true &&
                (row["packageId"] !== definition.packageId || row["type"] !== definition.name))
            ? "outside"
            : undefined;
    }

    /** Require a mapping to supply the trees and reference columns its policy's expressions need. */
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
                expression.kind === "grants" &&
                mapping.references?.[expression.reference] === undefined
            ) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `unmapped reference: ${expression.reference}`,
                );
            }
        }
    }

    /** Read the one subject type a relation accepts, refusing a relation accepting several. */
    onlySubject(type: Policy, name: string): SubjectType {
        const [only, ...others] = this.relation(type, name).subjects;
        if (only === undefined || others.length > 0) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `relation ${type.definition.name}#${name} accepts no single subject type`,
            );
        }

        return only;
    }

    /** Require a relation to the scope holding each object to accept one scope type alone, kept by the scope chain rather than a field. */
    #validateScopeRelation(type: Policy, name: string, relation: RelationDefinition): void {
        // require one scope type, kept by the scope chain
        const [scope, ...others] = relation.subjects;
        const isScopeType =
            scope !== undefined &&
            scope.relation === undefined &&
            scope.wildcard !== true &&
            this.policy(scope).definition.scope === true;
        if (others.length > 0 || !isScopeType) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${type.name}.${name} is the scope holding each object, so it accepts one scope type alone`,
            );
        }
        if (
            this.mappingOf({ packageId: type.definition.packageId, type: type.name })?.relations[
                name
            ]
        ) {
            throw new AccessError(
                "INVALID_DECLARATION",
                `${type.name}.${name} is the scope holding each object, so no field keeps it`,
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

    /**
     * Read what decides a permission along a scope chain: relations on the object, and permissions of its enclosing scopes.
     *
     * Subject sets and permissions read through enclosing scopes take only such permissions, since relationships on the chain decide them without the object's row.
     */
    deciding(reference: PermissionReference): {
        readonly relations: readonly string[];
        readonly enclosing: readonly PermissionReference[];
    } {
        // collect the relations and enclosing sets the permission's unions and named permissions reach
        const policy = this.policy(reference);
        const relations = new Set<string>();
        const enclosing: PermissionReference[] = [];
        const seen = new Set<string>();
        const pending = [this.expression(reference)];
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            if (expression.kind === "none") {
                continue;
            }
            // take a relation relationships keep, refusing one a field or the scope chain keeps
            else if (expression.kind === "relation") {
                const isScope = policy.definition.relations[expression.name]?.isScope === true;
                const isField = this.mappingOf(reference)?.relations[expression.name] !== undefined;
                if (isScope || isField) {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        `${reference.type}.${expression.name} is kept ${isScope ? "by the scope chain" : "in a field"}, so relationships on the chain cannot decide ${reference.name}`,
                    );
                }
                relations.add(expression.name);
            } else if (expression.kind === "union") {
                pending.push(...expression.expressions);
            } else if (expression.kind === "permission") {
                if (!seen.has(expression.name)) {
                    seen.add(expression.name);
                    pending.push(this.expression(policy.permission(expression.name)));
                }
            }
            // follow the scope enclosing the object
            else if (
                expression.kind === "through" &&
                policy.definition.relations[expression.relation]?.isScope === true
            ) {
                const scope = this.onlySubject(policy, expression.relation);
                enclosing.push({
                    packageId: scope.packageId,
                    type: scope.type,
                    name: expression.permission,
                });
            }
            // refuse what relationships on the chain cannot decide
            else {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `${reference.type}.${reference.name} is decided along a scope chain, so it follows relations alone`,
                );
            }
        }

        return { relations: [...relations], enclosing };
    }

    /** List the permissions accepted as subject sets on enclosed scopes of a type that an enclosing scope's set makes its members members of. */
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

    /** List the permissions accepted as subject sets that a relationship of a type's relation makes its subject a member of. */
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
        const definition = type.definition;
        const pending = [root];
        for (let expression = pending.pop(); expression !== undefined; expression = pending.pop()) {
            switch (expression.kind) {
                case "none":
                case "grants":
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
                case "condition":
                    validateCondition(expression.condition, type);
                    break;
                case "through": {
                    // follow the relation to plain objects of types declaring the permission
                    const relation = this.relation(type, expression.relation);
                    for (const subject of relation.subjects) {
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
                    break;
                }
            }
        }
    }
}

/** Take the next moment time alone changes whether any of some grants passes, absent when it never does. */
function grantsUntil(grants: readonly Grant[], access: Access): number | undefined {
    const context = access.context;

    return earliest(
        grants.flatMap((grant) => [
            grant.condition?.boundary(context),
            ...grant.arrows.map((arrow) => arrow.boundary(context)),
        ]),
    );
}

/** Require a condition to read declared attributes, ordering numbers only and comparing literals of their type. */
function validateCondition(condition: Condition, type: Policy): void {
    // refuse conditions that follow relations
    const [relation] = Condition.relations(condition);
    if (relation !== undefined) {
        throw new AccessError(
            "INVALID_DECLARATION",
            `policy conditions follow no relations: ${relation.via}`,
        );
    }

    // require each attribute declared
    const attributes = type.definition.attributes;
    for (const name of Condition.columns(condition)) {
        if (!Object.hasOwn(attributes, name)) {
            throw new AccessError("INVALID_DECLARATION", `undeclared object attribute: ${name}`);
        }
    }

    // require literals of the attribute's type, and numbers for ordered comparisons
    if (condition.kind === "compare") {
        const [attribute, other] =
            condition.left.kind === "column"
                ? [condition.left.name, condition.right]
                : condition.right.kind === "column"
                  ? [condition.right.name, condition.left]
                  : [undefined, undefined];
        const declared = attribute === undefined ? undefined : attributes[attribute];
        const isOrdered = condition.operator !== "eq" && condition.operator !== "ne";
        if (
            (isOrdered && declared !== undefined && declared !== "number") ||
            (other?.kind === "literal" && declared !== undefined && typeof other.value !== declared)
        ) {
            throw new AccessError(
                "INVALID_DECLARATION",
                "ordered comparisons require numbers and equality requires matching types",
            );
        }
    }
    // validate through combinations
    else if (condition.kind === "all" || condition.kind === "any" || condition.kind === "not") {
        const nested = condition.kind === "not" ? [condition.condition] : condition.conditions;
        for (const entry of nested) {
            validateCondition(entry, type);
        }
    }
}

/** Collect the keys of the permissions policies list as reserved or administrative. */
function listedPermissions(
    types: readonly Policy[],
    list: "reserved" | "administration",
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
function policyKey(policy: Policy): string {
    return ObjectTypeReference.key({
        packageId: policy.definition.packageId,
        type: policy.definition.name,
    });
}

/** Read the members of subject sets: the subjects their objects' relationships of the set's relation have. */
async function members(snapshot: Snapshot, sets: readonly Subject[]): Promise<Subject[]> {
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
