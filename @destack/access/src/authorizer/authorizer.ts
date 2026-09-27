import {
    alias,
    and,
    asc,
    eq,
    gt,
    jsonElements,
    or,
    PARAMETER_BUDGET,
    sql,
    Statement,
    TABLE,
    type DatabaseConnection,
    type SQL,
    type Table,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import type { PackageId } from "@destack/package";
import { identifier } from "@destack/schema";
import { Replica, type Watch } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import {
    permissionKey,
    relationsOf,
    type ObjectReference,
    type PermissionReference,
    type Policy,
    type TypeReference,
} from "../policy/policy.ts";
import type { AccessExpression } from "../policy/expression.ts";
import {
    accepts,
    type RelationDefinition,
    type Subject,
    type SubjectType,
} from "../policy/subject.ts";
import { INTRINSIC_POLICIES } from "../policy/principal.ts";
import { GLOBAL_SCOPE, type AccessContext } from "../context/context.ts";
import { accessRelationship } from "../relationship/table.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRole } from "../role/table.ts";
import { accessScope } from "../scope/table.ts";
import { ACCESS_TABLES, COPY_NAME, DECISION_TABLES } from "../replica/replica.ts";
import { earliest, Access, type SetRow } from "./access.ts";
import { Compiler } from "./compiler.ts";
import type { Decision, Explanation } from "./decision.ts";
import { GrantReader, type Grant, type Lookup } from "./grant.ts";
import { column, columnsOf, TableMapping, type FieldRelation } from "./mapping.ts";

/**
 * How long a copy of access may go without hearing from its home before decisions refuse it, by default, in milliseconds.
 *
 * Feeds repeat their position every ten seconds, so a copy three beats late has lost its source rather than lagging behind it.
 */
const LAG_MILLISECONDS = 30_000;

/** The rows of a set a caller holds a permission on, by position, and the next moment time alone may change that. */
export interface Admission {
    /** The positions of the rows the caller holds the permission on. */
    readonly held: ReadonlySet<number>;
    /** The next moment time alone may change which rows the caller holds it on, absent when only changes to access do. */
    readonly until?: number;
}

/** One scope of a chain: its object, the scope containing it, and whether it is suspended. */
export interface ScopeLink {
    /** The scope's object. */
    readonly object: ObjectReference;
    /** The scope containing this one, the global scope for users and organisations. */
    readonly parent: string;
    /** Whether the scope is suspended. */
    readonly isSuspended: boolean;
}

/** Decide policies over the tables their objects live in: registered and validated once, compiled to SQL, or decided in memory from grants. */
export class Authorizer {
    /** The keys of permissions only their expressions grant, which universal roles leave out. */
    readonly reserved: ReadonlySet<string>;
    /** The keys of permissions that apply only to elevated requests. */
    readonly elevated: ReadonlySet<string>;
    /** The keys of permissions that stay available while a scope is suspended. */
    readonly administration: ReadonlySet<string>;
    /** Field-held relations some relation names as a subject set. */
    readonly fields: FieldRelation[] = [];
    /** The subject sets some relation accepts and the relations of scopes: the memberships resolving a caller expands, and the sets roles may bind to. */
    readonly memberships: SubjectType[] = [];
    /** Read the memberships a JSON list of subjects holds through relationships. */
    readonly sets: Statement<SetRow>;
    /** How long a copy of access may go without hearing from its home before decisions refuse it, in milliseconds. */
    readonly lag: number;
    /** The object types this database holds the access rows of: those it maps a table of, other than access's own. */
    readonly held: readonly TypeReference[];
    /** The policies indexed by package and type. */
    readonly #policies = new Map<string, Policy>();
    /** The table mappings indexed by package and type. */
    readonly #mappings = new Map<string, TableMapping>();
    /** Read the rows of each mapped type by a JSON list of identifiers in a scope, by mapping. */
    readonly #rows = new Map<TableMapping, Statement>();
    /** Whether each permission is a union of relations, permissions and arrows, which grants decide in memory, by permission key. */
    readonly #indexable = new Map<string, boolean>();
    /** The compiler of the registered policies to SQL. */
    readonly #compiler: Compiler;

    /** Validate the policies, with access's own and every policy they name, and the tables of this database their objects live in. */
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
                ...(definition.elevated ?? []),
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
        this.elevated = listedPermissions(types, "elevated");
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
                table: accessScope,
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
                this.policy({ packageId: definition.packageId, type: definition.name }) !==
                mapping.policy
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
            this.#rows.set(mapping, TableMapping.rows(mapping));
        }

        // list the types whose access rows this database holds
        this.held = [...this.#mappings.values()]
            .filter((mapping) => !ACCESS_TABLES.includes(mapping.table))
            .map((mapping) => ({
                packageId: mapping.policy.definition.packageId,
                type: mapping.policy.definition.name,
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
            this.memberships.push({ packageId: packageId as PackageId, type, relation });
        }
        this.sets = Access.memberships(this.memberships);

        // expand subject sets through the fields holding them, which resolving looks up by an index leading with the column
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
                        sets: Access.fieldSets({ mapping, field }),
                    });
                }
            }
        }

        // require every declared relation of a mapped type to decide a permission: in an expression, as a subject set, or as the parent covering its objects
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

        return type.definition.relations[name]!;
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

    /** Read rows of a registered mapping's type by a JSON list of identifiers in a scope. */
    rows(mapping: TableMapping): Statement {
        return this.#rows.get(mapping)!;
    }

    /** Name the scope whose chain decides access to an object: the scope a scope object is, else the scope containing the object. */
    governingScope(target: ObjectReference): string {
        return this.policy(target).definition.scope === true ? target.id : target.scope;
    }

    /** Read the objects of a scope and every scope enclosing it, nearest first, with whether each is suspended. */
    static async chain(database: DatabaseConnection, scope: string): Promise<ScopeLink[]> {
        // read the scope, then the scopes its row lists, then any those rows list beyond them, each by key
        const rows = new Map<string, ScopeRow>();
        for (let wanted = [scope]; wanted.length > 0;) {
            const read = await SCOPES.all(database, {
                ids: JSON.stringify(wanted.map((id) => [id])),
            });
            for (const row of read) {
                rows.set(row.scope, row);
            }
            wanted = [...new Set(read.flatMap((row) => ancestorsOf(row)))].filter(
                (id) => !rows.has(id) && !wanted.includes(id),
            );
        }

        // order the scopes from the scope up through their parents
        const links: ScopeLink[] = [];
        for (let current = rows.get(scope); current !== undefined;) {
            if (links.some((link) => link.object.id === current!.scope)) {
                throw new AccessError("INVALID_CONTEXT", `cyclic scope: ${current.scope}`);
            }
            links.push({
                object: {
                    packageId: current.packageId,
                    type: current.type,
                    scope: current.parent,
                    id: current.scope,
                },
                parent: current.parent,
                isSuspended: current.suspendedAt !== null,
            });
            current = current.parent === GLOBAL_SCOPE ? undefined : rows.get(current.parent);
        }

        return links;
    }

    /** Read a scope's own object, which lives in the scope containing it, failing when the database knows no such scope. */
    static async scope(database: DatabaseConnection, id: string): Promise<ObjectReference> {
        const [link] = await Authorizer.chain(database, id);
        if (link === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${id}`);
        }

        return link.object;
    }

    /**
     * Decide whether this database holds an object's access rows: those of objects of a type it maps a table of.
     *
     * A role, and so its inclusions, lives with the scope object defining it.
     */
    async isHeld(database: DatabaseConnection, object: ObjectReference): Promise<boolean> {
        // follow a role to the scope object defining it
        if (this.mappingOf(object)?.table === accessRole) {
            return this.isHeld(database, await Authorizer.scope(database, object.scope));
        }

        return this.held.some(
            (type) => type.packageId === object.packageId && type.type === object.type,
        );
    }

    /** Refuse writing the access rows of an object another database holds, whose copies this database follows. */
    async requireHeld(database: DatabaseConnection, object: ObjectReference): Promise<void> {
        if (!(await this.isHeld(database, object))) {
            throw new AccessError(
                "FORBIDDEN",
                `access of ${object.type} ${object.id} is written in the database holding it`,
            );
        }
    }

    /** Name the copy of a scope's access this database follows: every decision row except those about objects it holds itself. */
    replica(scope: string): Replica {
        return Authorizer.replicaOf(scope, this.held);
    }

    /** Name the copy of a scope's access a database holding some object types follows, as its source serves it. */
    static replicaOf(scope: string, held: readonly TypeReference[]): Replica {
        const where = Condition.not(
            Condition.any(
                ...held.map((type) =>
                    Condition.all(
                        Condition.eq("packageId", type.packageId),
                        Condition.eq("type", type.type),
                    ),
                ),
            ),
        );

        return new Replica({
            name: COPY_NAME,
            scope,
            tables: DECISION_TABLES,
            where: new Map([[accessRelationship, where]]),
        });
    }

    /** Name the access rows whose changes decide again what a caller of a scope chain may hold. */
    watch(chain: readonly string[]): Watch[] {
        return DECISION_TABLES.map((table) => ({ table, scopes: chain }));
    }

    /** Read a page of one object's relationships and role bindings, ordered by identifier. */
    async relationships(
        database: DatabaseConnection,
        object: ObjectReference,
        page: { readonly after?: string; readonly limit: number },
    ): Promise<Relationship[]> {
        const rows = await database
            .select()
            .from(accessRelationship)
            .where(
                and(
                    Relationship.on(object),
                    page.after === undefined
                        ? undefined
                        : gt(accessRelationship.id, identifier("relationship").parse(page.after)),
                ),
            )
            .orderBy(asc(accessRelationship.id))
            .limit(page.limit);

        return rows.map(Relationship.decode);
    }

    /** Report whether relationships name an identifier of an object type in any scope, as they may after the object was deleted. */
    async isRelated(database: DatabaseConnection, policy: Policy, id: string): Promise<boolean> {
        const [row] = await database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(
                and(
                    eq(accessRelationship.packageId, policy.definition.packageId),
                    eq(accessRelationship.type, policy.definition.name),
                    eq(accessRelationship.objectId, id),
                ),
            )
            .limit(1);

        return row !== undefined;
    }

    /** Require a relationship to name exactly one relation or role, a subject the relation or role accepts, and a future expiry. */
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

    /** Delete the relationships of an object this database held and no longer does. */
    async forget(database: DatabaseConnection, object: ObjectReference): Promise<void> {
        await this.requireHeld(database, object);
        await database.delete(accessRelationship).where(Relationship.on(object));
    }

    /** Remove the relationships bound to one request once it commits, so they apply exactly once, leaving copies to their home. */
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

    /** Resolve a caller in a scope inside its transaction: its subject sets, the scope chain and the roles along it. */
    resolve(database: DatabaseConnection, scope: string, context: AccessContext): Promise<Access> {
        return Access.resolve(database, scope, context, this);
    }

    /** Restrict the rows of a table to those the caller holds a permission on, before sorting, counting, pagination, or mutation. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        return this.#compiler.where(permission, access, source);
    }

    /** Match one object, in its own scope, if the caller holds a permission on it or through a role bound on or above it. */
    holds(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        return this.#compiler.holds(permission, target, access);
    }

    /** Start reading the grants of rows within a scope chain, shared by every caller of the scope. */
    reader(database: DatabaseConnection, scopes: readonly ObjectReference[]): GrantReader {
        return new GrantReader(this, database, scopes);
    }

    /** Decide whether a caller holds a permission on one object, and until when that holds by time alone. */
    async check(
        database: DatabaseConnection,
        permission: PermissionReference,
        target: ObjectReference,
        access: Access,
        reader?: GrantReader,
    ): Promise<Decision> {
        // decide a missing object, which nobody holds a permission on
        const mapping = this.mapping(target);
        const row = await this.#read(database, target);
        if (row === undefined) {
            return { isAllowed: false };
        }

        // read the grants the permission reaches on the object
        const [grants] = await (reader ?? this.reader(database, access.scopes)).read(
            permission,
            [row],
            mapping,
        );
        const until = earliest([access.until, grantsUntil(grants!, access)]);

        // decide from the grants in memory where they express the permission, and in one statement otherwise
        const isOwn = permission.packageId === target.packageId && permission.type === target.type;
        const isAllowed =
            isOwn && this.isIndexable(permission)
                ? this.admits(permission, row, grants!, access, "object")
                : (
                      await database.execute(
                          sql`SELECT 1 AS held WHERE ${this.holds(permission, target, access)}`,
                      )
                  ).length > 0;

        return { isAllowed, ...(until === undefined ? {} : { until }) };
    }

    /** Require the caller to hold every permission on one object, naming the first it lacks, in one statement on a networked database. */
    async require(
        database: DatabaseConnection,
        permissions: readonly PermissionReference[],
        target: ObjectReference,
        access: Access,
        reader?: GrantReader,
    ): Promise<void> {
        // decide in memory what grants decide on an embedded database, which answers their small reads cheaply
        const isOwn = (permission: PermissionReference) =>
            permission.packageId === target.packageId && permission.type === target.type;
        const inMemory =
            database.state.locality === "embedded"
                ? permissions.filter(
                      (permission) => isOwn(permission) && this.isIndexable(permission),
                  )
                : [];
        if (inMemory.length > 0) {
            const row = await this.#read(database, target);
            const read = reader ?? this.reader(database, access.scopes);
            for (const permission of inMemory) {
                const [grants] = row === undefined ? [[]] : await read.read(permission, [row]);
                if (row === undefined || !this.admits(permission, row, grants!, access, "object")) {
                    throw new AccessError("FORBIDDEN", `permission denied: ${permission.name}`);
                }
            }
        }

        // evaluate the rest as the columns of a single row
        const queried = permissions.filter((permission) => !inMemory.includes(permission));
        if (queried.length === 0) {
            return;
        }
        const columns = queried.map(
            (permission, index) =>
                sql`CASE WHEN ${this.holds(permission, target, access)} THEN 1 ELSE 0 END AS ${sql.identifier(`held_${index}`)}`,
        );
        const [row] = await database.execute<Record<string, number | string>>(
            sql`SELECT ${sql.join(columns, sql`, `)}`,
        );
        const denied = queried.find((_, index) => Number(row![`held_${index}`]) !== 1);
        if (denied) {
            throw new AccessError("FORBIDDEN", `permission denied: ${denied.name}`);
        }
    }

    /** Check whether the caller owns one object, holding a role that grants everything on or above it. */
    async owns(
        database: DatabaseConnection,
        target: ObjectReference,
        access: Access,
    ): Promise<boolean> {
        const [row] = await database.execute(
            sql`SELECT 1 AS held WHERE ${this.#compiler.owns(target, access)}`,
        );

        return row !== undefined;
    }

    /**
     * Check a permission on each of the given rows, as they are or were: the positions the caller holds it on, and until when that holds by time alone.
     *
     * A union of relations, permissions and arrows decides from the grants the reader shares among callers; statements decide the rest.
     */
    async checkRows(
        database: DatabaseConnection,
        permission: PermissionReference,
        access: Access,
        rows: readonly Readonly<Record<string, unknown>>[],
        reader = this.reader(database, access.scopes),
    ): Promise<Admission> {
        // read the rows' grants, which bound when the decisions next change by time
        const grants = await reader.read(permission, rows);
        const until = earliest([
            access.until,
            ...grants.map((entry) => grantsUntil(entry, access)),
        ]);

        // admit rows through their grants where they express the permission
        if (this.isIndexable(permission)) {
            const held = rows.flatMap((row, position) =>
                this.admits(permission, row, grants[position]!, access) ? [position] : [],
            );

            return { held: new Set(held), ...(until === undefined ? {} : { until }) };
        }

        // present each row as one row of a derived table standing in for the object's table
        const mapping = this.mapping(permission);
        const source = alias(mapping.table, "access_row");
        const dialect = database.dialect;
        const columns = Object.entries(mapping.table[TABLE].columns);
        const selected = rows.map(
            (row, position) =>
                sql`SELECT ${position} AS access_position, ${sql.join(
                    columns.map(([property, entry]) => {
                        // bind the value through its column, which encodes it for the dialect
                        const value = row[property];
                        const encoded =
                            value === null || value === undefined ? null : sql.param(value, entry);

                        return sql`CAST(${encoded} AS ${sql.raw(entry.definition.types[dialect])}) AS ${sql.identifier(entry.definition.name)}`;
                    }),
                    sql`, `,
                )}`,
        );

        // evaluate the permission over chunks of derived rows that stay within the parameter limit
        const predicate = this.where(permission, access, source);
        const chunk = Math.max(1, Math.floor(PARAMETER_BUDGET / (columns.length + 1)));
        const held = new Set<number>();
        for (let start = 0; start < selected.length; start += chunk) {
            const derived = sql.join(selected.slice(start, start + chunk), sql` UNION ALL `);
            const permitted = await database.execute<{ access_position: number | string }>(
                sql`SELECT ${source}.access_position FROM (${derived}) AS ${source} WHERE ${predicate}`,
            );
            for (const row of permitted) {
                held.add(Number(row.access_position));
            }
        }

        return { held, ...(until === undefined ? {} : { until }) };
    }

    /**
     * Decide in memory whether a caller holds a permission on a row through its grants, as the listing's or the object's statement does.
     *
     * Only a union of relations, permissions and arrows decides from grants; `isIndexable` tells which permissions are.
     */
    admits(
        permission: PermissionReference,
        row: Readonly<Record<string, unknown>>,
        grants: readonly Grant[],
        access: Access,
        lookup: Lookup = "listing",
    ): boolean {
        return (
            this.#gate(permission, row, access, lookup) === undefined &&
            access.authorities.every((authority) =>
                grants.some((grant) => authority.failure(grant, access) === undefined),
            )
        );
    }

    /** Explain whether a caller holds a permission on one object: the gate, then each grant and why it fails, per authority. */
    async explain(
        database: DatabaseConnection,
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
        const found = await this.#read(database, target);
        if (found === undefined) {
            throw new AccessError("NOT_FOUND", `no ${target.type} ${target.id}`);
        }

        // decide by statement where the permission's grants do not express it
        const blocked = this.#gate(permission, found, access, "object");
        if (!this.isIndexable(permission)) {
            const decision = await this.check(database, permission, target, access);

            return {
                permission,
                object: target,
                isAllowed: decision.isAllowed,
                ...(blocked === undefined ? {} : { gate: blocked }),
                isQueried: true,
                authorities: [],
            };
        }

        // decide each grant for each authority
        const [grants] = await this.reader(database, access.scopes).read(permission, [found]);
        const decided = access.authorities.map((authority) => {
            const decisions = grants!.map((grant) => {
                const failed = authority.failure(grant, access);

                return {
                    path: [...grant.path],
                    object: grant.object,
                    subject: grant.subject,
                    ...(grant.role === undefined ? {} : { role: grant.role.id }),
                    ...(failed === undefined ? {} : { failure: failed }),
                };
            });

            return {
                ...(authority.delegator === undefined
                    ? {}
                    : { delegate: authority.subjects[0]!, delegator: authority.delegator }),
                isAllowed: decisions.some((decision) => decision.failure === undefined),
                grants: decisions,
            };
        });

        return {
            permission,
            object: target,
            isAllowed: blocked === undefined && decided.every((entry) => entry.isAllowed),
            ...(blocked === undefined ? {} : { gate: blocked }),
            isQueried: false,
            authorities: decided,
        };
    }

    /** Decide whether a permission is a union of relations, permissions and arrows, which grants decide in memory. */
    isIndexable(permission: PermissionReference): boolean {
        // decide each permission once
        const key = permissionKey(permission);
        const known = this.#indexable.get(key);
        if (known !== undefined) {
            return known;
        }

        // assume a permission met again along a cycle expressible, then walk its expression
        this.#indexable.set(key, true);
        const policy = this.policy(permission);
        const pending = [this.expression(permission)];
        let isIndexable = true;
        while (pending.length > 0 && isIndexable) {
            const expression = pending.pop()!;
            switch (expression.kind) {
                case "none":
                case "relation":
                    break;
                case "union":
                    pending.push(...expression.expressions);
                    break;
                case "permission":
                    isIndexable = this.isIndexable(policy.permission(expression.name));
                    break;
                case "through":
                    isIndexable = this.relation(policy, expression.relation).subjects.every(
                        (subject) =>
                            this.isIndexable({
                                packageId: subject.packageId,
                                type: subject.type,
                                name: expression.permission,
                            }),
                    );
                    break;
                case "intersection":
                case "exclusion":
                case "condition":
                case "grants":
                    isIndexable = false;
                    break;
            }
        }
        this.#indexable.set(key, isIndexable);

        return isIndexable;
    }

    /** Read the gate a request fails on a row before any grant: its credential, elevation or suspension, or a listed row outside the scope. */
    #gate(
        permission: PermissionReference,
        row: Readonly<Record<string, unknown>>,
        access: Access,
        lookup: Lookup,
    ): ReturnType<Access["gate"]> | "outside" {
        // refuse the request its credential, elevation or suspension refuses
        const mapping = this.mapping(permission);
        const refused = access.gate(permission, String(row[mapping.id]));
        if (refused !== undefined) {
            return refused;
        }

        // take an object read by its key in the scope as it is
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

    /** Read one object as a row of its type. */
    async #read(
        database: DatabaseConnection,
        target: ObjectReference,
    ): Promise<Record<string, unknown> | undefined> {
        const [found] = await this.rows(this.mapping(target)).all(database, {
            scope: target.scope,
            ids: JSON.stringify([[target.id]]),
        });

        return found;
    }

    /** Require a mapping to supply what its policy's expressions compile against: trees for transitive arrows, columns for referenced objects. */
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
        // resolve the relation of a subject set, which cannot be a wildcard
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

/** Collect the keys of the permissions policies list as reserved, elevated or administrative. */
function listedPermissions(
    types: readonly Policy[],
    list: "reserved" | "elevated" | "administration",
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

/** Read the scopes a scope row lists, as SQLite returns JSON in text and PostgreSQL parsed. */
function ancestorsOf(row: ScopeRow): string[] {
    return typeof row.ancestors === "string" ? JSON.parse(row.ancestors) : row.ancestors;
}

/** A scope row as a statement returns it. */
type ScopeRow = {
    /** The scope. */
    readonly scope: string;
    /** The containing scope. */
    readonly parent: string;
    /** The containing scopes the row lists, in JSON. */
    readonly ancestors: string | string[];
    /** The package declaring the scope object's type. */
    readonly packageId: PackageId;
    /** The scope object's type. */
    readonly type: string;
    /** When the scope was suspended. */
    readonly suspendedAt: number | string | null;
};

/** The scope rows by a JSON list of keys. */
const SCOPES = new Statement<ScopeRow>(
    (
        value,
    ) => sql`SELECT ${columnsOf(accessScope)} FROM ${jsonElements(value("ids"), "listed_scope")}
        JOIN ${accessScope} ON ${accessScope.scope} = listed_scope.value ->> 0`,
);
