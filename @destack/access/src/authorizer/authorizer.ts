import { v7 } from "uuid";
import { and, eq, or, sql, type DatabaseConnection, type SQL, type Table } from "@destack/db";
import { Condition } from "@destack/db/query";
import { Snapshot } from "@destack/db/log";
import type { PackageId } from "@destack/package";
import { identifier } from "@destack/schema";
import {
    ChainFollower,
    Replica,
    Scope,
    type ScopeLink,
    type Watch,
    type ChainRelay,
    type ObjectReference,
    type ObjectTypeReference,
} from "@destack/sync";
import { AccessError } from "../error/index.ts";
import {
    permissionKey,
    relationsOf,
    type PermissionReference,
    type Policy,
} from "../policy/policy.ts";
import type { AccessExpression } from "../policy/expression.ts";
import {
    accepts,
    type RelationDefinition,
    type Subject,
    subjectKey,
    type SubjectType,
} from "../policy/subject.ts";
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

/**
 * How long a copy of access may go without hearing from its home before decisions refuse it, by default, in milliseconds.
 *
 * Feeds repeat their position every ten seconds, and a copy three beats late has lost its source.
 */
const LAG_MILLISECONDS = 30_000;

/** The rows of a set a caller holds a permission on, by position, and the next moment time alone may change that. */
export interface Admission {
    /** The positions of the rows the caller holds the permission on. */
    readonly held: ReadonlySet<number>;
    /** The next moment time may change the rows the caller holds it on. */
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
    /** Field-held relations some relation names as a subject set. */
    readonly fields: FieldRelation[] = [];
    /** The subject sets that relations accept and roles may bind to. */
    readonly memberships: SubjectType[] = [];
    /** How long a copy of access may go without its home before decisions refuse it, in milliseconds. */
    readonly lag: number;
    /** The object types with access rows in this database. */
    readonly held: readonly ObjectTypeReference[];
    /** The object types this database copies inherited rows of from its containing scopes. */
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
        options: { readonly lag?: number } = {},
    ) {
        // include every policy the declared policies name
        const identity = (type: Policy) =>
            JSON.stringify([type.definition.packageId, type.definition.name]);
        const reachable = new Map<string, Policy>();
        const visited = new Set<Policy>();
        const pending = [...policies];
        while (pending.length > 0) {
            const type = pending.shift()!;
            if (visited.has(type)) {
                continue;
            }
            visited.add(type);
            pending.push(...type.references);

            // keep one policy per type, preferring a representing policy over the intrinsic one
            const existing = reachable.get(identity(type));
            if (existing === undefined || INTRINSIC_POLICIES.includes(existing)) {
                reachable.set(identity(type), type);
            }
            // refuse two distinct policies for one type
            else if (!INTRINSIC_POLICIES.includes(type)) {
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `duplicate object type: ${identity(type)}`,
                );
            }
        }
        const types = [
            ...INTRINSIC_POLICIES.filter((type) => !reachable.has(identity(type))),
            ...reachable.values(),
        ];

        // register each policy under its type
        for (const type of types) {
            this.#policies.set(identity(type), type);
        }

        // add each contributing type to the open relation it names
        for (const type of types) {
            for (const entry of type.definition.contributes ?? []) {
                const target = this.#policies.get(JSON.stringify([entry.packageId, entry.type]));
                if (target?.definition.relations[entry.relation]?.open !== true) {
                    throw new AccessError(
                        "INVALID_DECLARATION",
                        `${type.name} contributes to ${entry.type}.${entry.relation}, which is not an open relation`,
                    );
                }
                const key = JSON.stringify([entry.packageId, entry.type, entry.relation]);
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
                    permissionKey(type.permission(name)),
                    elevation,
                ]),
            ),
        );
        this.administration = listedPermissions(types, "administration");
        this.lag = options.lag ?? LAG_MILLISECONDS;
        this.#compiler = new Compiler(this);

        // map scope types this database holds no table of onto the ancestry every database holds
        const scopes = this.policies()
            .filter(
                (policy) =>
                    policy.definition.scope &&
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
            const key = JSON.stringify([definition.packageId, definition.name]);
            if (this.#mappings.has(key)) {
                throw new AccessError("INVALID_DECLARATION", `duplicate database mapping: ${key}`);
            }
            TableMapping.validate(this, mapping);
            this.#requireCompilable(mapping);
            this.#mappings.set(key, mapping);
        }

        // list the types whose access rows this database holds
        this.held = [...this.#mappings.values()]
            .filter((mapping) => !accessTables.includes(mapping.table))
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
            }));
        this.copied = [...this.#mappings.values()]
            .filter((mapping) => mapping.inherited !== undefined)
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
        const sets = new Set(
            this.policies()
                .flatMap((policy) => Object.values(policy.definition.relations))
                .flatMap((relation) => relation.subjects)
                .filter((subject) => subject.relation !== undefined)
                .map((subject) =>
                    JSON.stringify([subject.packageId, subject.type, subject.relation]),
                ),
        );
        const expanded = new Set([
            ...sets,
            ...this.policies()
                .filter((policy) => policy.definition.scope === true)
                .flatMap((policy) =>
                    Object.keys(policy.definition.relations).map((relation) =>
                        JSON.stringify([
                            policy.definition.packageId,
                            policy.definition.name,
                            relation,
                        ]),
                    ),
                ),
        ]);
        for (const key of expanded) {
            const [packageId, type, relation] = JSON.parse(key) as [string, string, string];
            this.memberships.push({
                packageId: packageId as PackageId,
                type,
                relation,
            });
        }

        // expand subject sets through the fields that hold them
        for (const mapping of this.#mappings.values()) {
            const definition = mapping.policy.definition;
            for (const [relation, field] of Object.entries(mapping.relations)) {
                if (sets.has(JSON.stringify([definition.packageId, definition.name, relation]))) {
                    const held = column(mapping.table, field.column);
                    const isIndexed = mapping.table
                        .constraints("sqlite")
                        .some(
                            (constraint) =>
                                (constraint.kind === "index" || constraint.kind === "unique") &&
                                constraint.columns[0] === held,
                        );
                    if (!isIndexed && !held.definition.primaryKey) {
                        throw new AccessError(
                            "INVALID_DECLARATION",
                            `subject set ${definition.name}#${relation} needs an index on ${field.column}`,
                        );
                    }
                    const [subject] = this.relation(mapping.policy, relation).subjects;
                    this.fields.push({
                        mapping,
                        relation,
                        field,
                        subject: subject!,
                    });
                }
            }
        }

        // require every declared relation of a mapped type to decide a permission
        const parents = new Set(
            [...this.#mappings.values()].map((mapping) =>
                JSON.stringify([
                    mapping.policy.definition.packageId,
                    mapping.policy.name,
                    mapping.parent,
                ]),
            ),
        );
        const intrinsic = new Set(INTRINSIC_POLICIES.map(identity));
        for (const type of types.filter(
            (policy) => !intrinsic.has(identity(policy)) && this.#mappings.has(identity(policy)),
        )) {
            const definition = type.definition;
            const decided = new Set(Object.values(definition.permissions).flatMap(relationsOf));
            for (const name of Object.keys(definition.relations)) {
                const key = JSON.stringify([definition.packageId, definition.name, name]);
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
        const type = this.#policies.get(JSON.stringify([reference.packageId, reference.type]));
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
        const definition = this.policy(reference).definition;
        if (!Object.hasOwn(definition.permissions, reference.name)) {
            throw new AccessError("INVALID_DECLARATION", `unknown permission: ${reference.name}`);
        }

        return definition.permissions[reference.name]!;
    }

    /** Resolve a declared relation without accepting inherited JavaScript properties. */
    relation(type: Policy, name: string): RelationDefinition {
        if (!Object.hasOwn(type.definition.relations, name)) {
            throw new AccessError("INVALID_DECLARATION", `unknown relation: ${name}`);
        }

        // add the types contributing to an open relation
        const relation = type.definition.relations[name]!;
        if (!relation.open) {
            return relation;
        }
        const key = JSON.stringify([type.definition.packageId, type.definition.name, name]);

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
        return this.#mappings.get(JSON.stringify([reference.packageId, reference.type]));
    }

    /** Return the table mappings in registration order. */
    mappings(): TableMapping[] {
        return [...this.#mappings.values()];
    }

    /** Read the permission another purely derives from, on its object or through a field-held relation. */
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

    /** Read the object a row's field-held relation names, absent when the row names none. */
    related(
        mapping: TableMapping,
        relation: string,
        row: Readonly<Record<string, unknown>>,
    ): ObjectReference | undefined {
        // read the related identifier
        const field = mapping.relations[relation]!;
        const id = row[field.column] as string | null | undefined;
        if (id === null || id === undefined) {
            return undefined;
        }

        // type it by the row's columns, or by the one type the relation accepts
        const [only] = this.relation(mapping.policy, relation).subjects;
        const typed = field.subject;

        return {
            packageId: (typed === undefined ? only!.packageId : row[typed.packageId]) as PackageId,
            type: String(typed === undefined ? only!.type : row[typed.type]),
            scope:
                typed === undefined
                    ? (field.scope ?? TableMapping.scope(mapping, row))
                    : String(row[typed.scope]),
            id,
        };
    }

    /** Read the scope with the chain that decides access to an object: a scope object itself, else its containing scope. */
    governingScope(target: ObjectReference): string {
        return this.policy(target).definition.scope === true ? target.id : target.scope;
    }

    /**
     * Decide whether this database holds an object's access rows: those of objects of a type it maps a table of.
     *
     * A role, and so its inclusions, lives with the scope object defining it.
     */
    async isHeld(database: DatabaseConnection, object: ObjectReference): Promise<boolean> {
        // follow a role to the scope object defining it
        if (this.mappingOf(object)?.table === accessRole) {
            const scope = await Scope.object(Snapshot.live(database), object.scope);

            return this.isHeld(database, scope);
        }

        return this.held.some(
            (type) => type.packageId === object.packageId && type.type === object.type,
        );
    }

    /** Refuse writing the access rows of an object another database holds. */
    async requireHeld(database: DatabaseConnection, object: ObjectReference): Promise<void> {
        if (!(await this.isHeld(database, object))) {
            throw new AccessError(
                "FORBIDDEN",
                `access of ${object.type} ${object.id} is written in the database holding it`,
            );
        }
    }

    /** Build the copy of a scope this database follows. */
    replica(scope: string): Replica {
        return this.replicaOf(scope, this.held, this.copied);
    }

    /** Build the copy of a scope that this source serves to a follower. */
    replicaOf(
        scope: string,
        held: readonly ObjectTypeReference[],
        copied: readonly ObjectTypeReference[],
    ): Replica {
        // leave out the access rows of the objects the follower holds, and of objects living in the universe
        const own = Condition.not(
            Condition.any(
                ...[...held, ...this.universal].map((type) =>
                    Condition.all(
                        Condition.eq("packageId", type.packageId),
                        Condition.eq("type", type.type),
                    ),
                ),
            ),
        );

        // add each copied type's inherited rows
        const inherited = copied.map((type) => {
            const mapping = this.#mappings.get(JSON.stringify([type.packageId, type.type]));
            if (mapping?.inherited === undefined) {
                throw new AccessError("NOT_FOUND", `no inherited rows of ${type.type}`);
            }

            return { table: mapping.table, where: mapping.inherited };
        });

        return new Replica({
            name: COPY_NAME,
            scope,
            tables: [...decisionTables, ...inherited.map((entry) => entry.table)],
            where: new Map([
                [accessRelationship, own],
                ...inherited.map((entry): [Table, Condition] => [entry.table, entry.where]),
            ]),
        });
    }

    /** Copy the chain a relay streams into a database this authorizer decides in. */
    follower(
        database: DatabaseConnection,
        held: readonly ObjectTypeReference[],
        relay: ChainRelay,
    ): ChainFollower {
        const copied = this.copied;

        return new ChainFollower(
            database,
            relay.scope,
            (scope) => this.replicaOf(scope, held, copied),
            (scope, after, signal) =>
                relay.watch(
                    { scope, held, copied, ...(after === undefined ? {} : { after }) },
                    signal,
                ),
        );
    }

    /** List the access rows with changes that affect what a caller of a scope chain may hold. */
    watch(chain: readonly string[]): Watch[] {
        return decisionTables.map((table) => ({ table, scopes: chain }));
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

        return rows.map((row) => Relationship.decode(row as RelationshipRow));
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

    /** Require a relationship with one relation or role, an accepted subject and a future expiry. */
    validate(
        request: {
            readonly object: ObjectReference;
            readonly relation?: string;
            readonly role?: string;
            readonly subject?: Subject;
            readonly expiresAt?: number | null;
        },
        now: number,
    ): void {
        const subject = request.subject;
        const expiresAt = request.expiresAt ?? null;

        // require exactly one relation or role
        if ((request.relation === undefined) === (request.role === undefined)) {
            throw new AccessError("FORBIDDEN", "a relationship names exactly one relation or role");
        }
        // require a subject the relation accepts
        else if (
            request.relation !== undefined &&
            subject !== undefined &&
            !accepts(this.relation(this.policy(request.object), request.relation), subject)
        ) {
            throw new AccessError("FORBIDDEN", `relation ${request.relation} rejects the subject`);
        }
        // require a role's subject to be named, since a wildcard would bind the role to everyone
        else if (
            request.role !== undefined &&
            subject !== undefined &&
            (subject.id === "*" || subject.scope === "*")
        ) {
            throw new AccessError("FORBIDDEN", "a role binds only to named subjects");
        }
        // require a role's subject set to be one resolving a caller expands
        else if (
            request.role !== undefined &&
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
        // delete the request's relationships about the types this database holds
        const types = this.held.map((held) =>
            and(
                eq(accessRelationship.packageId, held.packageId),
                eq(accessRelationship.type, held.type),
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

    /** Restrict the rows of a table to those the caller holds a permission on, before sorting, counting, pagination, or mutation. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        return this.#compiler.where(permission, access, source);
    }

    /** Match one object, in its own scope, if the caller holds a permission on it or through a role bound on or above it. */
    holds(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        return this.#compiler.holds(permission, target, access);
    }

    /** Start reading the grants of rows within a scope chain, as a snapshot shows them, shared by every caller of the scope. */
    reader(snapshot: Snapshot, scopes: readonly ObjectReference[]): GrantReader {
        return new GrantReader(this, snapshot, scopes);
    }

    /** Decide whether a caller holds a permission on one object, and until when that holds by time alone. */
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
        const [tree] = await reading.trees(permission, [row], mapping);
        const until = earliest([access.until, grantsUntil(GrantTree.flatten(tree!), access)]);

        // admit the caller when every authority holds the permission, past its gate for one object
        const isAllowed =
            this.#gate(permission, mapping, row, access, "object") === undefined &&
            access.authorities.every((authority) => GrantTree.holds(tree!, authority, access));

        return { isAllowed, ...(until === undefined ? {} : { until }) };
    }

    /**
     * Require the caller to hold every permission on one object, naming the first it lacks.
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
                sql`CASE WHEN ${this.holds(permission, target, access)} THEN 1 ELSE 0 END AS ${sql.identifier(`held_${index}`)}`,
        );
        const [row] = await database.execute<Record<string, number | string>>(
            sql`SELECT ${sql.join(columns, sql`, `)}`,
        );
        const denied = permissions.find((_, index) => Number(row![`held_${index}`]) !== 1);
        if (denied) {
            await this.#refuse(snapshot, denied, target, access);
        }
    }

    /**
     * Find the fresh authentication that would admit a refused caller: the lowest assurance level at which a decision holds, with the permission's elevation age.
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
        const elevation = this.elevated.get(permissionKey(permission));
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
     * List a page of the principals of a type holding a permission on one object now, in subject key order, through subject sets.
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
        const [tree] = await reader.trees(permission, [row], this.mapping(target));

        // expand the grants' subject sets into the principals of the type
        const candidates = new Map<string, Subject>();
        const seen = new Set<string>();
        for (
            let frontier = GrantTree.flatten(tree!).map((grant) => grant.subject);
            frontier.length > 0;
        ) {
            const sets: Subject[] = [];
            for (const subject of frontier) {
                const key = subjectKey(subject);
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
        const held: Subject[] = [];
        for (const [, subject] of ordered) {
            if (held.length === page.limit) {
                break;
            }
            const access = await this.resolveAssured(snapshot, scope, subject, now, links);
            if ((await this.check(snapshot, permission, target, access, reader)).isAllowed) {
                held.push(subject);
            }
        }

        return held;
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

        return access.authorities.every((authority) => GrantTree.holds(tree, authority, access));
    }

    /**
     * Check a permission on each of the given rows, as they are or were: the positions the caller holds it on, and until when that holds by time alone.
     *
     * Every permission decides in memory from the grant trees the reader shares among callers, as the compiled predicate decides in SQL.
     */
    async checkRows(
        snapshot: Snapshot,
        permission: PermissionReference,
        access: Access,
        rows: readonly Readonly<Record<string, unknown>>[],
        reader = this.reader(snapshot, access.scopes),
    ): Promise<Admission> {
        // read the rows' grant trees
        const trees = await reader.trees(permission, rows);
        const until = earliest([
            access.until,
            ...trees.map((tree) => grantsUntil(GrantTree.flatten(tree), access)),
        ]);

        // admit the rows with a tree that admits every authority of the caller
        const held = rows.flatMap((row, position) =>
            this.#gate(permission, this.mapping(permission), row, access, "listing") ===
                undefined &&
            access.authorities.every((authority) =>
                GrantTree.holds(trees[position]!, authority, access),
            )
                ? [position]
                : [],
        );

        return { held: new Set(held), ...(until === undefined ? {} : { until }) };
    }

    /** Explain whether a caller holds a permission on one object: the gate, then each grant and why it fails, per authority. */
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
        const [tree] = await reader.trees(permission, [found]);
        const grants = GrantTree.flatten(tree!);
        const decided = access.authorities.map((authority) => ({
            ...(authority.delegator === undefined
                ? {}
                : { delegate: authority.subjects[0]!, delegator: authority.delegator }),
            isAllowed: GrantTree.holds(tree!, authority, access),
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
            (mapping.isShared &&
                (row.packageId !== definition.packageId || row.type !== definition.name))
            ? "outside"
            : undefined;
    }

    /** Require a mapping to supply the trees and reference columns its policy's expressions need. */
    #requireCompilable(mapping: TableMapping): void {
        const pending = Object.values(mapping.policy.definition.permissions);
        while (pending.length > 0) {
            const expression = pending.pop()!;
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

    /** Require a subject type to name a registered type and one of its relations. */
    #validateSubject(subject: SubjectType): void {
        // resolve the relation of a subject set
        const type = this.policy(subject);
        if (subject.relation !== undefined) {
            if (subject.wildcard) {
                throw new AccessError("INVALID_DECLARATION", "a subject set cannot be a wildcard");
            }
            this.relation(type, subject.relation);
        }
    }

    /** Resolve all expressions and keep recursion in explicit ancestor traversal. */
    #validate(reference: PermissionReference, parents: ReadonlySet<string>): void {
        // detect recursion by the complete permission reference
        const key = permissionKey(reference);
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
        while (pending.length > 0) {
            const expression = pending.pop()!;
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

/** Take the next moment time alone changes whether any of some grants holds, absent when it never does. */
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
                permissionKey({
                    packageId: type.definition.packageId,
                    type: type.definition.name,
                    name,
                }),
            ),
        ),
    );
}

/** Read the members of subject sets: the subjects their objects' relationships of the set's relation hold. */
async function members(snapshot: Snapshot, sets: readonly Subject[]): Promise<Subject[]> {
    // read nothing for no sets
    if (sets.length === 0) {
        return [];
    }

    // read the relationships of each set's relation
    const wanted = new Set(sets.map((set) => subjectKey(set)));
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
        .filter((relationship) =>
            wanted.has(subjectKey({ ...relationship.object, relation: relationship.relation! })),
        )
        .map((relationship) => relationship.subject);
}
