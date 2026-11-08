import { Scope } from "@destack/sync";
import {
    alias,
    from,
    listed,
    Predicate,
    textElements,
    type Binding,
    type Snapshot,
    sql,
    type SQL,
    type SQLWrapper,
    type Table,
} from "@destack/db";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";
import {
    CONTAINED,
    type AccessExpression,
    type ConditionExpression,
} from "../declare/expression.ts";
import { AccessContext } from "../context/context.ts";
import { accessRelationship } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { Authorizer } from "./authorizer.ts";
import { TableMapping } from "./mapping.ts";

/** The most identifiers one hop's set carries: 10,000 × ~45 bytes ≈ 450 KB in one parameter. */
const MAX_HOP_IDS = 10_000;

/** The rows of a table a lookup reaches: every row, or the rows a condition matches. */
type Reach = "all" | SQL;

/** An object a role binding sits on. */
interface BoundObject {
    /** The object's package. */
    readonly packageId: PackageId;
    /** The object's type. */
    readonly type: string;
    /** The object's identifier. */
    readonly id: string;
}

/** An expression following a relation to a related object's permission. */
type ThroughExpression = Extract<AccessExpression, { kind: "through" }>;

/** A superset of the rows a caller may hold a permission on, walked back from the caller. */
export class Lookup {
    /** The authorizer whose policies and mappings the walk follows. */
    readonly #authorizer: Authorizer;
    /** The snapshot the hops read. */
    readonly #snapshot: Snapshot;
    /** The resolved caller. */
    readonly #access: Access;
    /** The objects a role binding to the caller's authority sits on, read once per lookup. */
    #bindings: Promise<"all" | readonly BoundObject[]> | undefined;
    /** The identifiers each hop reached, by type and permission, read once per lookup. */
    readonly #hops = new Map<string, Promise<"all" | readonly string[]>>();

    /** Look up rows for a caller resolved in a scope, reading hops from a snapshot. */
    constructor(authorizer: Authorizer, snapshot: Snapshot, access: Access) {
        this.#authorizer = authorizer;
        this.#snapshot = snapshot;
        this.#access = access;
    }

    /** Match a table's rows the caller may hold a permission on, the mapped table by default. */
    async where(permission: PermissionReference, source?: Table): Promise<SQL> {
        // match every row of a past or layered snapshot, whose hops the live database does not have
        const mapping = this.#authorizer.requireMapping(permission);
        const table = source ?? mapping.table;
        if (!this.#snapshot.isCurrent) {
            return sql`true`;
        }

        // match the reached rows of the mapped type
        const reach = await this.#permission(permission, mapping, table, new Set());
        const typed = Lookup.#typed(mapping, table);

        return reach === "all" ? typed : sql`(${typed} AND ${reach})`;
    }

    /** Reach the rows a permission allows: its expression, or a role binding covering the row. */
    async #permission(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Table,
        seen: ReadonlySet<string>,
    ): Promise<Reach> {
        // reach no row a gate refuses, and every row a role on the scope chain covers
        const access = this.#access;
        if (access.blocked(permission) !== undefined) {
            return sql`false`;
        } else if (access.holds(permission)) {
            return "all";
        }

        // reach every row of a permission met again, as a cycle widens to all
        const key = PermissionReference.key(permission);
        if (seen.has(key)) {
            return "all";
        }
        const within = new Set([...seen, key]);

        // reach the expression's rows and the rows a role binding covers
        const expression = this.#authorizer.expression(permission);
        const reached = await this.#expression(expression, mapping, row, within);
        const covered = await this.#covered(permission, mapping, row, within);

        return Lookup.#union([reached, covered]);
    }

    /** Reach the rows one expression allows, widening to all where it cannot follow back. */
    async #expression(
        expression: AccessExpression,
        mapping: TableMapping,
        row: Table,
        seen: ReadonlySet<string>,
    ): Promise<Reach> {
        switch (expression.kind) {
            case "none":
                return sql`false`;
            case "union":
                return Lookup.#union(
                    await Promise.all(
                        expression.expressions.map((child) =>
                            this.#expression(child, mapping, row, seen),
                        ),
                    ),
                );
            case "intersection": {
                // reach the narrowest child's rows, which hold every row the intersection allows
                const children = await Promise.all(
                    expression.expressions.map((child) =>
                        this.#expression(child, mapping, row, seen),
                    ),
                );

                return children.find((child) => child !== "all") ?? "all";
            }
            case "exclusion":
                return this.#expression(expression.include, mapping, row, seen);
            case "permission":
                return this.#permission(
                    mapping.policy.permission(expression.name),
                    mapping,
                    row,
                    seen,
                );
            case "resource":
                return this.#resource(expression, mapping, row);
            case "relation":
                return this.#relation(expression.name, mapping, row);
            case "through":
                return this.#through(expression, mapping, row, seen);
            case "contained":
                return this.#hasSet(CONTAINED) ? "all" : sql`false`;
            case "subjectSet":
                return this.#hasSet(expression.relation) ? "all" : sql`false`;
            case "self":
                return this.#self(mapping, row);
            // reach every row by conditions on the request and by references to other objects
            case "context":
            case "granters":
            case "readers":
                return "all";
        }
    }

    /** Reach the rows meeting a condition over their attributes, all where the table lacks them. */
    #resource(expression: ConditionExpression, mapping: TableMapping, row: Table): Reach {
        // reach every row of a table without the attributes
        const predicate = mapping.policy.resolve(expression);
        if (!TableMapping.decides(mapping, predicate)) {
            return "all";
        }

        // match the condition over the row's columns and the request's attributes
        const context = this.#access.context;
        const binding: Binding<SQLWrapper> = {
            field: (name) => TableMapping.column(row, TableMapping.attribute(mapping, name)),
            placeholder: (name) => AccessContext.attribute(context, name),
            exists: (via) => {
                throw AccessError.of(
                    "DECLARATION_INVALID",
                    `policy conditions cannot follow relations: ${via}`,
                );
            },
            search: () => {
                throw AccessError.of("DECLARATION_INVALID", "policy conditions cannot search");
            },
        };

        return sql`coalesce(${Predicate.render(predicate, binding)}, false)`;
    }

    /** Reach the rows with the caller as a relation's subject, in a field or a relationship. */
    #relation(name: string, mapping: TableMapping, row: Table): Reach {
        // match the subject in a field, or a set the caller holds on the row
        const { authority } = this.#access;
        const definition = mapping.policy.definition;
        const field = mapping.relations[name];
        const sets = authority.subjects
            .filter(
                (subject) =>
                    subject.relation === name &&
                    subject.packageId === definition.packageId &&
                    subject.type === definition.name,
            )
            .map((subject) => subject.id);
        const held =
            sets.length === 0
                ? sql`false`
                : listed(TableMapping.column(row, mapping.id), sql`${JSON.stringify(sets)}`);
        if (field !== undefined) {
            return sql`(${this.#field(name, field, mapping, row)} OR ${held})`;
        }

        // match a relationship of the caller on the row through the relation
        this.#record(accessRelationship);
        const relationship = alias(accessRelationship, "access_candidate_relationship");

        return sql`(EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE ${relationship.objectScope} = ${TableMapping.scopeColumn(row, mapping)}
                AND ${relationship.objectPackageId} = ${definition.packageId}
                AND ${relationship.objectType} = ${definition.name}
                AND ${relationship.objectId} = ${TableMapping.column(row, mapping.id)}
                AND ${relationship.relation} = ${name}
                AND ${authority.member(relationship)}
        ) OR ${held})`;
    }

    /** Match a field's subject against the caller: typed columns, or an identifier of one type. */
    #field(
        name: string,
        field: TableMapping["relations"][string],
        mapping: TableMapping,
        row: Table,
    ): SQL {
        // match a subject by the type, scope and relation in the row's columns
        const { authority } = this.#access;
        const id = TableMapping.column(row, field.column);
        const typed = field.subject;
        if (typed !== undefined) {
            const subject = {
                packageId: TableMapping.column(row, typed.packageId),
                type: TableMapping.column(row, typed.type),
                scope: TableMapping.column(row, typed.scope),
                id,
            };
            const relation =
                typed.relation === undefined ? undefined : TableMapping.column(row, typed.relation);

            return relation === undefined
                ? sql`coalesce(${authority.match(subject)}, false)`
                : sql`coalesce((
                    (${relation} IS NULL AND ${authority.match(subject)})
                    OR (${relation} IS NOT NULL AND ${authority.match({ ...subject, relation })})
                ), false)`;
        }

        // match the one subject type of a field
        const type = this.#authorizer.onlySubject(mapping.policy, name);
        const subject = {
            packageId: sql`${type.packageId}`,
            type: sql`${type.type}`,
            scope:
                field.scope === undefined
                    ? TableMapping.scopeColumn(row, mapping)
                    : sql`${field.scope}`,
            id,
            ...(type.relation === undefined ? {} : { relation: sql`${type.relation}` }),
        };

        return sql`coalesce(${authority.match(subject)}, false)`;
    }

    /** Reach the rows whose related object has a permission, each hop its own statement. */
    async #through(
        expression: ThroughExpression,
        mapping: TableMapping,
        row: Table,
        seen: ReadonlySet<string>,
    ): Promise<Reach> {
        // reach the rows of the caller's scope when its enclosing object has the permission
        const { relation, permission } = expression;
        if (mapping.policy.definition.relations[relation]?.isScope === true) {
            return this.#enclosing(relation, permission, mapping, row);
        }

        // reach through each related type this database stores, by the identifiers its hop reached
        const subjects = this.#authorizer.relation(mapping.policy, relation).subjects;
        const field = mapping.relations[relation];
        const arrows = await Promise.all(
            subjects.map(async (subject): Promise<Reach> => {
                // reach nothing through a type this database stores none of
                const target = this.#authorizer.mapping(subject);
                if (target === undefined) {
                    return sql`false`;
                }

                // reach the rows related to the hop's identifiers or keys, all when the hop reached all
                const column = field?.key ?? target.id;
                const ids = await this.#hop(
                    target.policy.permission(permission),
                    target,
                    seen,
                    "permitted",
                    column,
                );
                if (ids === "all") {
                    return "all";
                }
                const values = sql`${JSON.stringify(ids)}`;

                // follow an ancestor index, a typed or a plain field, else the row's relationships
                if (field === undefined) {
                    return this.#related(relation, subject, mapping, row, values);
                } else if (expression.transitive) {
                    return this.#descending(relation, mapping, row, values);
                } else if (field.subject !== undefined) {
                    return sql`(${TableMapping.column(row, field.subject.packageId)} = ${subject.packageId}
                        AND ${TableMapping.column(row, field.subject.type)} = ${subject.type}
                        AND ${listed(TableMapping.column(row, field.column), values)})`;
                }

                return listed(TableMapping.column(row, field.column), values);
            }),
        );

        return Lookup.#union(arrows);
    }

    /** Reach the rows of the caller's scope when the scope's object has a permission, else none. */
    async #enclosing(
        relation: string,
        name: string,
        mapping: TableMapping,
        row: Table,
    ): Promise<Reach> {
        // reach every row where the scope's object is of no type the relation names or this database stores
        const access = this.#access;
        const [object] = access.scopes;
        const subjects = this.#authorizer.relation(mapping.policy, relation).subjects;
        const isNamed = subjects.some(
            (subject) => subject.packageId === object?.packageId && subject.type === object.type,
        );
        if (object === undefined || !isNamed || this.#authorizer.mapping(object) === undefined) {
            return "all";
        }

        // decide the permission on the scope's object, keeping the rows of enclosing scopes
        const permission = { packageId: object.packageId, type: object.type, name };
        const decision = await this.#authorizer.check(this.#snapshot, permission, object, access);
        const scope = TableMapping.scopeColumn(row, mapping);

        return decision.isAllowed ? "all" : sql`${scope} <> ${access.scope}`;
    }

    /** Reach the rows related through relationships to one of the hop's identifiers. */
    #related(
        relation: string,
        subject: { readonly packageId: string; readonly type: string },
        mapping: TableMapping,
        row: Table,
        values: SQL,
    ): SQL {
        // match a relationship from the row through the relation to one of the hop's identifiers
        this.#record(accessRelationship);
        const relationship = alias(accessRelationship, "access_candidate_arrow");
        const definition = mapping.policy.definition;

        return sql`EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE ${relationship.objectScope} = ${TableMapping.scopeColumn(row, mapping)}
                AND ${relationship.objectPackageId} = ${definition.packageId}
                AND ${relationship.objectType} = ${definition.name}
                AND ${relationship.objectId} = ${TableMapping.column(row, mapping.id)}
                AND ${relationship.relation} = ${relation}
                AND ${relationship.subjectPackageId} = ${subject.packageId}
                AND ${relationship.subjectType} = ${subject.type}
                AND ${listed(relationship.subjectId, values)}
        )`;
    }

    /** Reach the rows below one of the hop's identifiers in a relation's ancestor index. */
    #descending(relation: string, mapping: TableMapping, row: Table, values: SQL): SQL {
        // match an ancestor of the row in the relation's index among the hop's identifiers
        const tree = TableMapping.tree(mapping, relation);
        this.#record(tree.ancestors);
        const ancestor = alias(tree.ancestors, "access_candidate_ancestor");

        return sql`EXISTS (
            SELECT 1 FROM ${from(ancestor)}
            WHERE ${ancestor.scope} = ${TableMapping.scopeColumn(row, mapping)}
                AND ${ancestor.descendant} = ${TableMapping.column(row, mapping.id)}
                AND ${ancestor.depth} > 0
                AND ${listed(ancestor.ancestor, values)}
        )`;
    }

    /** Reach the rows a binding covers: on the row, its scopes, or an owner above it. */
    async #covered(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Table,
        seen: ReadonlySet<string>,
    ): Promise<Reach> {
        // reach every row once the caller's bindings pass the bound, else the rows with one on them or their tree ancestors
        const bindings = await this.#bound();
        if (bindings === "all") {
            return "all";
        }
        const id = TableMapping.column(row, mapping.id);
        const definition = mapping.policy.definition;
        const bound = bindings
            .filter(
                (object) =>
                    object.packageId === definition.packageId && object.type === definition.name,
            )
            .map((object) => object.id);
        const arrows: Reach[] = [];
        if (bound.length > 0) {
            const values = sql`${JSON.stringify(bound)}`;
            arrows.push(listed(id, values));
            for (const relation of Object.keys(mapping.trees ?? {})) {
                arrows.push(this.#descending(relation, mapping, row, values));
            }
        }

        // reach the rows of a scope a binding sits on, or below one, the caller's chain aside
        const scopes = bindings
            .filter(
                (object) =>
                    this.#authorizer.find({ packageId: object.packageId, type: object.type })
                        ?.definition.scope === true,
            )
            .map((object) => object.id);
        if (mapping.scope !== undefined && scopes.length > 0) {
            arrows.push(this.#within(TableMapping.scopeColumn(row, mapping), scopes));
        }

        // reach the rows whose owner parent or owner a binding covers
        for (const relation of [mapping.parent, mapping.owner]) {
            if (relation !== undefined) {
                arrows.push(
                    await this.#ownerCovered(permission, mapping, row, relation, scopes, seen),
                );
            }
        }

        return arrows.length === 0 ? sql`false` : Lookup.#union(arrows);
    }

    /** Reach the rows whose owner a binding covers: a scope's chain, else the owner's hop. */
    async #ownerCovered(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Table,
        relation: string,
        scopes: readonly string[],
        seen: ReadonlySet<string>,
    ): Promise<Reach> {
        // reach an owner scope by the bindings on it or the scopes enclosing it, stored here or not
        const subject = this.#authorizer.onlySubject(mapping.policy, relation);
        const field = TableMapping.field(mapping, relation);
        const owner = TableMapping.column(row, field.column);
        if (this.#authorizer.policy(subject).definition.scope === true) {
            return scopes.length === 0 ? sql`false` : this.#within(owner, scopes);
        }

        // reach another owner through its hop, nothing through a type this database stores none of
        const target = this.#authorizer.mapping(subject);
        if (target === undefined) {
            return sql`false`;
        }
        const ids = await this.#hop(
            { packageId: subject.packageId, type: subject.type, name: permission.name },
            target,
            seen,
            "covered",
            field.key ?? target.id,
        );

        return ids === "all"
            ? "all"
            : ids.length === 0
              ? sql`false`
              : listed(owner, sql`${JSON.stringify(ids)}`);
    }

    /** List the objects a role binding to the caller's authority sits on, all past the bound. */
    #bound(): Promise<"all" | readonly BoundObject[]> {
        this.#bindings ??= this.#readBound();

        return this.#bindings;
    }

    /** Read the objects the caller's bindings sit on: resolved ones, and this database's. */
    async #readBound(): Promise<"all" | readonly BoundObject[]> {
        // take the bindings the caller's resolution read
        const resolved = this.#access.bindings.map((binding) => ({
            packageId: binding.objectPackageId,
            type: binding.objectType,
            id: binding.objectId,
        }));

        // read this database's bindings to the authority, its wildcards, anyone and link secrets among them
        this.#record(accessRelationship);
        const relationship = alias(accessRelationship, "access_candidate_binding");
        const rows = await this.#snapshot.database.execute(
            sql`SELECT ${relationship.objectPackageId} AS package_id, ${relationship.objectType} AS type, ${relationship.objectId} AS id
                FROM ${from(relationship)}
                WHERE ${relationship.roleId} IS NOT NULL AND ${this.#access.authority.member(relationship)}
                LIMIT ${MAX_HOP_IDS + 1}`,
            schema.object({ package_id: PackageId, type: schema.string(), id: schema.string() }),
        );
        const stored = rows.map((each) => ({
            packageId: each.package_id,
            type: each.type,
            id: each.id,
        }));

        return rows.length > MAX_HOP_IDS ? "all" : [...resolved, ...stored];
    }

    /** Read the identifiers one hop reaches, once per type and permission, all past the bound. */
    #hop(
        permission: PermissionReference,
        target: TableMapping,
        seen: ReadonlySet<string>,
        reach: "permitted" | "covered",
        column: string,
    ): Promise<"all" | readonly string[]> {
        // reach every row of a hop met again on the walk, as a cycle widens to all
        const key = `${reach}:${PermissionReference.key(permission)}:${column}`;
        if (seen.has(key)) {
            return Promise.resolve("all");
        }

        // read each hop once per lookup
        const known = this.#hops.get(key);
        if (known !== undefined) {
            return known;
        }
        const read = this.#readHop(permission, target, new Set([...seen, key]), reach, column);
        this.#hops.set(key, read);

        return read;
    }

    /** Read a hop's identifiers in its own statement, all past the bound. */
    async #readHop(
        permission: PermissionReference,
        target: TableMapping,
        seen: ReadonlySet<string>,
        reach: "permitted" | "covered",
        column: string,
    ): Promise<"all" | readonly string[]> {
        // reach the target's rows, by its permission or by the bindings covering them
        this.#record(target.table);
        const row = alias(target.table, "access_candidate_hop");
        const reached =
            reach === "permitted"
                ? await this.#permission(permission, target, row, seen)
                : await this.#covered(permission, target, row, seen);
        if (reached === "all") {
            return "all";
        }

        // read the reached identifiers or keys one past the bound, widening a larger hop to all
        const id = TableMapping.column(row, column);
        const rows = await this.#snapshot.database.execute(
            sql`SELECT ${id} AS id FROM ${from(row)}
                WHERE ${Lookup.#typed(target, row)} AND ${reached}
                LIMIT ${MAX_HOP_IDS + 1}`,
            schema.object({ id: schema.string() }),
        );

        return rows.length > MAX_HOP_IDS ? "all" : rows.map((each) => each.id);
    }

    /** Reach the rows that are the caller's own principal. */
    #self(mapping: TableMapping, row: Table): SQL {
        const definition = mapping.policy.definition;

        return this.#access.authority.match({
            packageId: sql`${definition.packageId}`,
            type: sql`${definition.name}`,
            scope:
                definition.isGlobal === true
                    ? sql`${Scope.universe.id}`
                    : TableMapping.scopeColumn(row, mapping),
            id: TableMapping.column(row, mapping.id),
        });
    }

    /** Record a scan of a table the candidates read, so a commit conflicts with its changes. */
    #record(table: Table): void {
        this.#snapshot.reads?.rows(table, {});
    }

    /** Report whether the caller belongs to any subject set of a relation. */
    #hasSet(relation: string): boolean {
        return this.#access.authority.subjects.some((subject) => subject.relation === relation);
    }

    /** Match a scope that is one of some scopes, or lies below one by its scope row's ancestors. */
    #within(scope: SQLWrapper, scopes: readonly string[]): SQL {
        // match the scope, or an ancestor its scope row lists
        const values = sql`${JSON.stringify(scopes)}`;
        this.#record(Scope.table);
        const here = alias(Scope.table, "access_candidate_scope");
        const above = sql.identifier("access_candidate_above");

        return sql`(${listed(scope, values)} OR EXISTS (
            SELECT 1 FROM ${from(here)}
            WHERE ${here.scope} = ${scope}
                AND EXISTS (
                    SELECT 1 FROM ${textElements(here.ancestors, "access_candidate_above")}
                    WHERE ${listed(sql`${above}.value`, values)}
                )
        ))`;
    }

    /** Join reaches: all once any reaches all, else their conditions. */
    static #union(reaches: readonly Reach[]): Reach {
        // reach all once any child does
        if (reaches.some((reach) => reach === "all")) {
            return "all";
        }

        // match any child's rows
        const conditions = reaches.filter((reach): reach is SQL => reach !== "all");

        return conditions.length === 0 ? sql`false` : sql`(${sql.join(conditions, sql` OR `)})`;
    }

    /** Select a mapped type's rows of a table several types share, every row of its own table. */
    static #typed(mapping: TableMapping, row: Table): SQL {
        const definition = mapping.policy.definition;

        return mapping.isShared === true
            ? sql`(${TableMapping.column(row, "packageId")} = ${definition.packageId} AND ${TableMapping.column(row, "type")} = ${definition.name})`
            : sql`true`;
    }
}
