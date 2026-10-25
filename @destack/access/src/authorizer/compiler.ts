import { ObjectReference, type ObjectTypeReference, Scope } from "@destack/sync";
import {
    alias,
    textElements,
    type CommonTable,
    from,
    hashName,
    inArray,
    jsonElements,
    listed,
    sql,
    type SQL,
    type SQLWrapper,
    type Table,
    Predicate,
    type Binding,
} from "@destack/db";
import { nth } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";
import {
    CONTAINED,
    type AccessExpression,
    type ConditionExpression,
} from "../declare/expression.ts";
import { type RelationDescription, type SubjectType } from "../declare/subject.ts";
import { AccessContext } from "../context/context.ts";
import { GrantPermission } from "../grant/permission.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship, type RelationshipColumnMap } from "../relationship/table.ts";
import { type Access, OWNING } from "./access.ts";
import type { Authority, SubjectColumns } from "./authority.ts";
import type { Authorizer } from "./authorizer.ts";
import { TupleCondition } from "./condition.ts";
import { ANY_SCOPE, TableMapping } from "./mapping.ts";

/** The scope chain of the rows a predicate decides: known objects, or read from a scope column. */
type Chain =
    | {
          readonly kind: "known";
          readonly scope: string;
          readonly objects: readonly ObjectReference[];
      }
    | { readonly kind: "column"; readonly scope: SQLWrapper };

/** The resolved caller, compiled authority and alias allocation one predicate shares. */
interface Compilation {
    /** The caller's resolved access. */
    readonly access: Access;
    /** The scope chain of the rows the predicate decides, nearest first. */
    readonly chain: Chain;
    /** The authority whose tuples the predicate matches: the represented subject. */
    readonly authority: Authority;
    /** The next free alias number. */
    readonly aliases: { next: number };
    /** The common table of each related permission the statement decides, by name. */
    readonly permitted: Map<string, CommonTable>;
}

/** The name of the list element a binding's role is read from. */
const GRANTED = "access_granted";

/** A relation stored in a field of a mapped table. */
type Field = TableMapping["relations"][string];

/** An expression following a relation to a related object's permission. */
type ThroughExpression = Extract<AccessExpression, { kind: "through" }>;

/** A relation's target type, with its table alias and its permission compiled against it. */
interface RelationTarget {
    /** The related subject type. */
    readonly subject: SubjectType;
    /** The mapping of the related type. */
    readonly target: TableMapping;
    /** The alias of the related type's table. */
    readonly parent: Table;
    /** The related permission compiled against the alias. */
    readonly predicate: SQL;
}

/** Compile policies to SQL over their objects' tables, for a caller resolved in one scope. */
export class Compiler {
    /** The authorizer whose policies and mappings the compiler reads. */
    readonly #authorizer: Authorizer;

    /** Compile the policies an authorizer registered. */
    constructor(authorizer: Authorizer) {
        this.#authorizer = authorizer;
    }

    /** Restrict a table's rows, the mapped one by default, to the caller's permitted ones. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        // deny a request its gate refuses
        const mapping = this.#authorizer.requireMapping(permission);
        const table = source ?? mapping.table;
        if (access.blocked(permission) !== undefined) {
            return sql`false`;
        }

        // decide the rows of the scope with its chain
        const scope = TableMapping.scopeColumn(table, mapping);
        const inside = sql`(${scope} = ${access.scope} AND ${this.#match(permission, mapping, table, access, access.scope, access.scopes)})`;
        const object = access.scopeObject(mapping);
        if (object === undefined) {
            return inside;
        }

        // decide the scope's row in the scope containing it with that scope's chain
        const itself = sql`(${scope} = ${object.scope} AND ${TableMapping.column(table, mapping.id)} = ${object.id} AND ${this.#match(permission, mapping, table, access, object.scope, access.scopes.slice(1))})`;

        return sql`(${inside} OR ${itself})`;
    }

    /** Match a table's rows of one scope chain that the caller has a permission on. */
    #match(
        permission: PermissionReference,
        mapping: TableMapping,
        table: Table,
        access: Access,
        scope: string,
        chain: readonly ObjectReference[],
    ): SQL {
        // evaluate the authority, and constrain a grant to the objects it allows
        const compilation = Compiler.#compilation(access, scope, chain);
        const predicates = [this.#permission(permission, mapping, table, compilation)];
        if (access.context.grants.length > 0) {
            predicates.push(
                this.#restricted(permission, TableMapping.column(table, mapping.id), access, {
                    mapping,
                    table,
                }),
            );
        }

        // select the mapped type's rows of a table several types share
        if (mapping.isShared === true) {
            const definition = mapping.policy.definition;
            predicates.push(
                sql`${TableMapping.column(table, "packageId")} = ${definition.packageId} AND ${TableMapping.column(table, "type")} = ${definition.name}`,
            );
        }

        return sql`(${sql.join(predicates, sql` AND `)})`;
    }

    /** Match one object in its scope, or the scope's object, if the caller has a permission. */
    permits(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        // deny a request that its elevation, suspension or credential refuses
        const isSameType =
            permission.packageId === target.packageId && permission.type === target.type;
        if (
            access.blocked(permission) !== undefined ||
            (!isSameType && !access.allows(permission, target.id))
        ) {
            return sql`false`;
        }

        // evaluate another type's permission through roles
        return this.#target(target, access, (mapping, row, compilation) => {
            if (!isSameType) {
                return this.#bound(permission, mapping, row, compilation);
            }

            // evaluate the target's permission where the credential allows it
            const allowed = this.#restricted(
                permission,
                TableMapping.column(row, mapping.id),
                access,
                {
                    mapping,
                    table: row,
                },
            );
            const granted = this.#permission(permission, mapping, row, compilation);

            return sql`(${allowed} AND ${granted})`;
        });
    }

    /** Match one object of the resolved scope when the authority meets a row predicate. */
    #target(
        target: ObjectReference,
        access: Access,
        evaluate: (mapping: TableMapping, row: Table, compilation: Compilation) => SQL,
    ): SQL {
        // require the target in the resolved scope or to be the scope, and evaluate the authority on its row
        const object = access.scopes[0];
        const isScope =
            object !== undefined && ObjectReference.key(object) === ObjectReference.key(target);
        if (target.scope !== access.scope && !isScope) {
            throw AccessError.of("TARGET_OUTSIDE_SCOPE", "target is outside the resolved scope");
        }
        const mapping = this.#authorizer.requireMapping(target);
        const chain = isScope ? access.scopes.slice(1) : access.scopes;
        const compilation = Compiler.#compilation(access, target.scope, chain);
        const row = alias(mapping.table, "access_target");
        const isTarget = (
            source: Table,
        ) => sql`${TableMapping.column(source, mapping.id)} = ${target.id}
            AND ${TableMapping.scopeColumn(source, mapping)} = ${target.scope}`;
        const local = sql`EXISTS (
            SELECT 1 FROM ${from(row)}
            WHERE ${isTarget(row)} AND ${evaluate(mapping, row, compilation)}
        )`;
        if (mapping.policy.definition.scope !== true) {
            return local;
        }

        // decide a scope whose object row lives elsewhere on its ancestry row, which its chain stores
        const missing = alias(mapping.table, "access_missing");
        const ancestry = TableMapping.ancestry(mapping.policy);
        const standing = alias(Scope.table, "access_ancestry");

        return sql`(${local} OR (
            NOT EXISTS (SELECT 1 FROM ${from(missing)} WHERE ${isTarget(missing)})
            AND EXISTS (
                SELECT 1 FROM ${from(standing)}
                WHERE ${TableMapping.column(standing, ancestry.id)} = ${target.id}
                    AND ${TableMapping.scopeColumn(standing, ancestry)} = ${target.scope}
                    AND ${evaluate(ancestry, standing, compilation)}
            )
        ))`;
    }

    /** Match the objects the grant allows a permission on, directly or through its source. */
    #restricted(
        permission: PermissionReference,
        id: SQLWrapper,
        access: Access,
        stored?: { readonly mapping: TableMapping; readonly table: Table },
    ): SQL {
        // match the objects the grant lists
        const selected = GrantPermission.select(permission, access.scope, access.permissions);
        if (selected === "every") {
            return sql`true`;
        }
        const named =
            selected.length === 0 ? sql`false` : listed(id, sql`${JSON.stringify(selected)}`);

        // match a derived permission when its source matches, on the same object or the related one
        const source = this.#authorizer.source(permission);
        if (source === undefined) {
            return named;
        } else if (source.relation === undefined) {
            const derived = { ...permission, name: source.permission };

            return sql`(${named} OR ${this.#restricted(derived, id, access, stored)})`;
        } else if (stored === undefined) {
            return named;
        }

        // match each type the relation accepts through the related object's column
        const { mapping, table } = stored;
        const field = TableMapping.field(mapping, source.relation);
        const typed = field.subject;
        const subjects = this.#authorizer.relation(mapping.policy, source.relation).subjects;
        const arrows = subjects.map((subject) => {
            const derived = {
                packageId: subject.packageId,
                type: subject.type,
                name: source.permission,
            };
            const allowed = this.#restricted(
                derived,
                TableMapping.column(table, field.column),
                access,
            );

            // accept a single-typed relation in the caller's scope
            if (typed === undefined) {
                return field.scope === undefined || field.scope === access.scope
                    ? allowed
                    : sql`false`;
            }
            // accept a typed relation to this type in the caller's scope
            else {
                const conditions = [
                    sql`${TableMapping.column(table, typed.packageId)} = ${subject.packageId}`,
                    sql`${TableMapping.column(table, typed.type)} = ${subject.type}`,
                    sql`${TableMapping.column(table, typed.scope)} = ${access.scope}`,
                    allowed,
                ];

                return sql`(${sql.join(conditions, sql` AND `)})`;
            }
        });

        return sql`(${sql.join([named, ...arrows], sql` OR `)})`;
    }

    /** Start compiling for a caller's authority on rows of a scope with a known chain. */
    static #compilation(
        access: Access,
        scope: string,
        chain: readonly ObjectReference[],
    ): Compilation {
        return {
            access,
            chain: { kind: "known", scope, objects: chain },
            authority: access.authority,
            aliases: { next: 0 },
            permitted: new Map(),
        };
    }

    /** Continue compiling for rows whose chain their scope column reads. */
    static #within(compilation: Compilation, scope: SQLWrapper): Compilation {
        return { ...compilation, chain: { kind: "column", scope } };
    }

    /** Permit by the expression or a role bound on the row, an ancestor or an enclosing scope. */
    #permission(
        permission: PermissionReference,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        const expression = this.#authorizer.expression(permission);
        const compiled = this.#compile(expression, mapping, source, compilation);

        return sql`(${compiled} OR ${this.#bound(permission, mapping, source, compilation)})`;
    }

    /** Match a row the caller has a permission on, deciding each permission once per statement. */
    #permitted(
        permission: PermissionReference,
        target: TableMapping,
        row: Table,
        compilation: Compilation,
    ): SQL {
        // read the permission's common table for this caller, compiling it at its first use
        const { access, authority } = compilation;
        const key = JSON.stringify([
            PermissionReference.key(permission),
            TupleCondition.bindings(access.context),
            authority.subjects,
            access.context.attributes,
            access.grantedRoles,
        ]);
        const name = `access_permitted_${hashName(key)}`;
        const table =
            compilation.permitted.get(name) ??
            this.#permittedTable(name, permission, target, compilation);

        // match the row by its identifier and scope
        const matched = sql.identifier(`access_matched_${compilation.aliases.next++}`);

        return sql`EXISTS (
            SELECT 1 FROM ${table} AS ${matched}
            WHERE ${matched}.id = ${TableMapping.column(row, target.id)}
                AND ${matched}.scope = ${TableMapping.scopeColumn(row, target)}
        )`;
    }

    /** Compile the common table of a type's rows that the caller has a permission on. */
    #permittedTable(
        name: string,
        permission: PermissionReference,
        target: TableMapping,
        compilation: Compilation,
    ): CommonTable {
        // decide each row with the chain of the scope it lives in
        const row = alias(target.table, "access_decided");
        const scope = TableMapping.scopeColumn(row, target);
        const decided = this.#permission(permission, target, row, {
            ...compilation,
            chain: { kind: "column", scope },
            aliases: { next: 0 },
        });
        const table = sql.common(
            name,
            sql`SELECT ${TableMapping.column(row, target.id)} AS id, CAST(${scope} AS TEXT) AS scope
                FROM ${from(row)}
                WHERE ${decided}`,
        );
        compilation.permitted.set(name, table);

        return table;
    }

    /** Match a current role binding granting the permission to the authority on a cover. */
    #bound(
        permission: PermissionReference,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // match a cover, leaving a binding inside the row, which covers what the row contains
        const id = TableMapping.column(source, mapping.id);

        return Compiler.#matchBinding(
            permission,
            compilation,
            (relationship) => sql`(
                ${this.#covers(relationship, mapping, source, compilation)}
                AND NOT (${Relationship.insideCondition(relationship)} AND ${relationship.objectId} = ${id})
            )`,
        );
    }

    /** Match a binding on the row, an owner ancestor, or a scope enclosing the query's scope. */
    #covers(
        binding: RelationshipColumnMap,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // match the row itself or an object of the scope chain
        const id = TableMapping.column(source, mapping.id);
        const scope = TableMapping.scopeColumn(source, mapping);
        const definition = mapping.policy.definition;
        const typed = sql`
            ${binding.objectScope} = ${scope}
            AND ${binding.objectPackageId} = ${definition.packageId}
            AND ${binding.objectType} = ${definition.name}
        `;
        const covering = [
            sql`(${typed} AND ${binding.objectId} = ${id})`,
            Compiler.#chainCovers(binding, compilation),
        ];
        const tree = mapping.parent === undefined ? undefined : mapping.trees?.[mapping.parent];

        // match tree ancestors through their closure
        if (tree !== undefined) {
            const ancestor = alias(tree.ancestors, `access_ancestor_${compilation.aliases.next++}`);
            covering.push(sql`(${typed} AND EXISTS (
                SELECT 1 FROM ${from(ancestor)}
                WHERE ${ancestor.scope} = ${scope} AND ${ancestor.descendant} = ${id} AND ${ancestor.ancestor} = ${binding.objectId}
            ))`);
        }
        // climb to the owner parent and match it and its ancestors
        else if (mapping.parent !== undefined) {
            const subject = this.#authorizer.onlySubject(mapping.policy, mapping.parent);
            const target = this.#authorizer.requireMapping(subject);
            const parent = alias(target.table, `access_owner_${compilation.aliases.next++}`);
            covering.push(sql`EXISTS (
                SELECT 1 FROM ${from(parent)}
                WHERE ${TableMapping.column(parent, target.id)} = ${TableMapping.column(source, TableMapping.field(mapping, mapping.parent).column)}
                    AND ${TableMapping.scopeColumn(parent, target)} = ${scope}
                    AND ${this.#covers(binding, target, parent, compilation)}
            )`);
        }

        // match the owner, the owner's coverage and the scopes enclosing it
        if (mapping.owner !== undefined) {
            covering.push(
                this.#ownerCovering(binding, mapping, source, mapping.owner, compilation),
            );
        }

        return sql`(${sql.join(covering, sql` OR `)})`;
    }

    /** Match a binding on a row's owner, covering the owner, or on a scope enclosing the owner. */
    #ownerCovering(
        binding: RelationshipColumnMap,
        mapping: TableMapping,
        source: Table,
        relation: string,
        compilation: Compilation,
    ): SQL {
        // find the owner the relation's field refers to
        const subject = this.#authorizer.onlySubject(mapping.policy, relation);
        const target = this.#authorizer.requireMapping(subject);
        const field = TableMapping.field(mapping, relation);
        const referred = TableMapping.column(source, field.column);

        // match a scope owner's chain from the owner up
        if (target.policy.definition.scope === true) {
            return Compiler.#chainCovers(binding, Compiler.#within(compilation, referred));
        }

        // match any other owner's coverage with the chain of the owner's scope
        const owner = alias(target.table, `access_owner_${compilation.aliases.next++}`);
        const within = Compiler.#within(compilation, TableMapping.scopeColumn(owner, target));

        return sql`EXISTS (
            SELECT 1 FROM ${from(owner)}
            WHERE ${TableMapping.column(owner, target.id)} = ${referred}
                AND ${Compiler.#inScope(field, mapping, source, target, owner)}
                AND ${this.#covers(binding, target, owner, within)}
        )`;
    }

    /** Translate one expression using bound values and generated SQL aliases. */
    #compile(
        expression: AccessExpression,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        switch (expression.kind) {
            case "none":
                return sql`false`;
            case "union":
            case "intersection": {
                const children = expression.expressions.map((child) =>
                    this.#compile(child, mapping, source, compilation),
                );
                const separator = expression.kind === "union" ? sql` OR ` : sql` AND `;

                return sql`(${sql.join(children, separator)})`;
            }
            case "exclusion": {
                const include = this.#compile(expression.include, mapping, source, compilation);
                const exclude = this.#compile(expression.exclude, mapping, source, compilation);

                return sql`(${include} AND NOT ${exclude})`;
            }
            case "permission":
                return this.#permission(
                    mapping.policy.permission(expression.name),
                    mapping,
                    source,
                    compilation,
                );
            case "resource":
            case "context": {
                // grant nothing by attributes when the database has only the object's scope row
                const predicate = mapping.policy.resolve(expression);
                if (expression.kind === "resource" && !TableMapping.decides(mapping, predicate)) {
                    return sql`false`;
                }
                const binding = Compiler.#bindAttributes(
                    expression,
                    mapping,
                    source,
                    compilation.access.context,
                );

                return sql`coalesce(${Predicate.render(predicate, binding)}, false)`;
            }
            case "relation":
                return this.#relation(expression.name, mapping, source, compilation);
            case "through":
                return this.#through(expression, mapping, source, compilation);
            case "granters":
                return this.#granters(expression.reference, mapping, source, compilation);
            case "readers":
                return this.#readers(expression, mapping, source, compilation);
            case "contained":
                return Compiler.#contained(expression, mapping, source, compilation);
            case "subjectSet":
                return Compiler.#subjectSet(expression, mapping, source, compilation);
            case "self":
                return Compiler.#self(mapping, source, compilation);
        }
    }

    /** Match a row when the authority belongs to one object's subject set in the row's scope. */
    static #subjectSet(
        expression: Extract<AccessExpression, { kind: "subjectSet" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        return compilation.authority.match({
            packageId: sql`${expression.object.packageId}`,
            type: sql`${expression.object.type}`,
            scope: TableMapping.scopeColumn(source, mapping),
            id: sql`${expression.id}`,
            relation: sql`${expression.relation}`,
        });
    }

    /** Match the principal a row represents, a global one by its universe reference. */
    static #self(mapping: TableMapping, source: Table, compilation: Compilation): SQL {
        const definition = mapping.policy.definition;

        return compilation.authority.match({
            packageId: sql`${definition.packageId}`,
            type: sql`${definition.name}`,
            scope:
                definition.isGlobal === true
                    ? sql`${Scope.universe.id}`
                    : TableMapping.scopeColumn(source, mapping),
            id: TableMapping.column(source, mapping.id),
        });
    }

    /** Decide whether an authority may have a containment grant, of the named types when given. */
    static #allows(
        authority: Compilation["authority"],
        principals: readonly ObjectTypeReference[] | undefined,
    ): boolean {
        const isTyped =
            principals === undefined ||
            principals.some((type) => authority.isMember({ ...type, scope: "*", id: "*" }));

        return isTyped;
    }

    /** Match a row when the authority lives in the row's scope, or in the row itself as a scope. */
    static #contained(
        expression: Extract<AccessExpression, { kind: "contained" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // allow only the named principal types when given
        const { authority } = compilation;
        if (!Compiler.#allows(authority, expression.principals)) {
            return sql`false`;
        }

        // match a scope's row by its containment set, a global scope's also by its universe reference
        const definition = mapping.policy.definition;
        if (definition.scope === true) {
            const set = (scope: SubjectColumns["scope"]) =>
                authority.match({
                    packageId: sql`${definition.packageId}`,
                    type: sql`${definition.name}`,
                    scope,
                    id: TableMapping.column(source, mapping.id),
                    relation: sql`${CONTAINED}`,
                });
            const scoped = set(TableMapping.scopeColumn(source, mapping));

            return definition.isGlobal === true
                ? sql`(${scoped} OR ${set(sql`${Scope.universe.id}`)})`
                : scoped;
        }

        // match another row by the containment set of the scope it lives in, known or read
        const { chain } = compilation;
        if (chain.kind === "known") {
            const [nearest] = chain.objects;
            const isInside =
                nearest !== undefined && authority.isMember({ ...nearest, relation: CONTAINED });

            return isInside ? sql`true` : sql`false`;
        }

        return Compiler.#nearest(compilation, (scope) =>
            authority.match({ ...scope, relation: sql`${CONTAINED}` }),
        );
    }

    /** Match the authority by a current relationship or containment on a chain object. */
    #enclosing(
        expression: ThroughExpression,
        mapping: TableMapping,
        compilation: Compilation,
    ): SQL {
        // decide a known chain's steps at compile time, else match them on the read chain
        const { chain, authority } = compilation;
        if (chain.kind === "known") {
            return this.#enclosingKnown(expression, mapping, chain.objects, compilation);
        }

        // follow every declared scope type of a one-type relation, else only the nearest scope's
        const { relation, permission } = expression;
        const isNearest = this.#authorizer.relation(mapping.policy, relation).subjects.length !== 1;
        const scopes = this.#authorizer.scopeTypes(mapping.policy, relation, permission);

        // match each step on the nearest chain object of its type, by relationships or containment
        const steps = scopes.flatMap((scope) =>
            this.#authorizer
                .inherited({ packageId: scope.packageId, type: scope.type, name: permission })
                .flatMap(({ type, relations, contained }) => {
                    // skip a step that grants nothing to this authority
                    const isContained =
                        contained !== undefined &&
                        Compiler.#allows(authority, contained.principals);
                    if (relations.length === 0 && !isContained) {
                        return [];
                    }
                    const followed = isNearest ? scope : undefined;

                    return [
                        Compiler.#nearestOfType(
                            chain.scope,
                            type,
                            followed,
                            compilation,
                            (object) => {
                                const related =
                                    relations.length === 0
                                        ? []
                                        : [this.#related(object, relations, compilation)];
                                const inside = isContained
                                    ? [authority.match({ ...object, relation: sql`${CONTAINED}` })]
                                    : [];

                                return sql`(${sql.join([...related, ...inside], sql` OR `)})`;
                            },
                        ),
                    ];
                }),
        );

        return steps.length === 0 ? sql`false` : sql`(${sql.join(steps, sql` OR `)})`;
    }

    /** Match the authority by a relationship or containment on a known chain's deciding objects. */
    #enclosingKnown(
        expression: ThroughExpression,
        mapping: TableMapping,
        chain: readonly ObjectReference[],
        compilation: Compilation,
    ): SQL {
        // grant at once to a chain scope with the authority in its containment set, for allowed types
        const steps = this.#authorizer.chainSteps(mapping.policy, expression, chain);
        const { authority } = compilation;
        const isContained = steps.some(
            ({ object, contained }) =>
                contained !== undefined &&
                authority.isMember({ ...object, relation: CONTAINED }) &&
                Compiler.#allows(authority, contained.principals),
        );
        if (isContained) {
            return sql`true`;
        }

        // match one current relationship of the authority on any deciding object
        const matches = steps
            .filter(({ relations }) => relations.length > 0)
            .map(({ object, relations }) =>
                this.#related(Compiler.#constant(object), relations, compilation),
            );

        return matches.length === 0 ? sql`false` : sql`(${sql.join(matches, sql` OR `)})`;
    }

    /** Match a current relationship of the authority on an object through some relations. */
    #related(object: SubjectColumns, relations: readonly string[], compilation: Compilation): SQL {
        const relationship = alias(
            accessRelationship,
            `access_enclosing_${compilation.aliases.next++}`,
        );

        return sql`EXISTS (
            SELECT 1 FROM ${Compiler.#current(compilation)} AS ${relationship}
            WHERE ${Compiler.#isOn(object, relationship)}
                AND ${inArray(relationship.relation, [...relations])}
        )`;
    }

    /** Match rows with a referenced object the caller has the grant permission on. */
    #granters(
        reference: string,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // compile each granting type's grant permission against a new alias of the referenced row
        const columns = TableMapping.reference(mapping, reference);
        const branches = this.#authorizer.mappings().flatMap((target) => {
            // skip a type nothing grants
            const definition = target.policy.definition;
            if (definition.grantedBy === undefined) {
                return [];
            }

            // alias the referenced row and read the type's grant permission
            const row = alias(target.table, `access_referenced_${compilation.aliases.next++}`);
            const grant = target.policy.permission(definition.grantedBy);

            return [
                sql`(
                    ${TableMapping.column(source, columns.packageId)} = ${definition.packageId}
                    AND ${TableMapping.column(source, columns.type)} = ${definition.name}
                    AND EXISTS (
                        SELECT 1 FROM ${from(row)}
                        WHERE ${TableMapping.column(row, target.id)} = ${TableMapping.column(source, columns.id)}
                            AND ${TableMapping.scopeColumn(row, target)} = ${TableMapping.column(source, columns.scope)}
                            AND ${this.#permitted(grant, target, row, compilation)}
                    )
                )`,
            ];
        });

        return branches.length === 0 ? sql`false` : sql`(${sql.join(branches, sql` OR `)})`;
    }

    /** Match who sees the relationships of a row's referenced object. */
    #readers(
        expression: Extract<AccessExpression, { kind: "readers" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // compile each type's visibility against a separate alias of the referenced row
        const columns = TableMapping.reference(mapping, expression.reference);
        const related = TableMapping.column(source, expression.relation);
        const branches = this.#authorizer.mappings().flatMap((target) => {
            // skip the row's type, whose rows relate other objects
            if (target.policy === mapping.policy) {
                return [];
            }

            // see a type's open relations through its relationship read or grant permission
            const row = alias(target.table, `access_referenced_${compilation.aliases.next++}`);
            const permitted = (name: string | undefined) =>
                name === undefined
                    ? sql`false`
                    : this.#permitted(target.policy.permission(name), target, row, compilation);
            const visibility = target.policy.visibility;
            const concealed = Object.entries(target.policy.definition.relations).filter(
                ([, relation]) => relation.concealed === true,
            );
            const open =
                concealed.length === 0
                    ? sql`true`
                    : sql`(${related} IS NULL OR ${related} NOT IN (${sql.join(
                          concealed.map(([name]) => sql`${name}`),
                          sql`, `,
                      )}))`;
            const seen = sql`(${open} AND (${permitted(visibility)} OR ${permitted(target.policy.definition.grantedBy)}))`;

            // see a concealed relation's relationships only through its grant permission
            const hidden = concealed.map(
                ([name, relation]) =>
                    sql`(${related} = ${name} AND ${permitted(relation.grantedBy)})`,
            );

            return [
                sql`(
                    ${TableMapping.column(source, columns.packageId)} = ${target.policy.definition.packageId}
                    AND ${TableMapping.column(source, columns.type)} = ${target.policy.definition.name}
                    AND EXISTS (
                        SELECT 1 FROM ${from(row)}
                        WHERE ${TableMapping.column(row, target.id)} = ${TableMapping.column(source, columns.id)}
                            AND ${TableMapping.scopeColumn(row, target)} = ${TableMapping.column(source, columns.scope)}
                            AND (${sql.join([seen, ...hidden], sql` OR `)})
                    )
                )`,
            ];
        });

        return branches.length === 0 ? sql`false` : sql`(${sql.join(branches, sql` OR `)})`;
    }

    /** Match the subject in a field, or current relationships, for one relation. */
    #relation(name: string, mapping: TableMapping, source: Table, compilation: Compilation): SQL {
        // read the relation and whether a field stores it
        const relation = this.#authorizer.relation(mapping.policy, name);
        const field = mapping.relations[name];

        // match the subject in a field
        if (field) {
            return this.#field(name, field, mapping, source, compilation.authority);
        }

        // match current relationships of subject types the relation accepts
        const relationship = alias(
            accessRelationship,
            `access_relationship_${compilation.aliases.next++}`,
        );
        const definition = mapping.policy.definition;

        // match the relation sets of the authority
        const isHolding = compilation.authority.subjects.some(
            (subject) =>
                subject.relation === name &&
                subject.packageId === definition.packageId &&
                subject.type === definition.name,
        );
        const isMember = isHolding
            ? compilation.authority.match({
                  packageId: sql`${definition.packageId}`,
                  type: sql`${definition.name}`,
                  scope: TableMapping.scopeColumn(source, mapping),
                  id: TableMapping.column(source, mapping.id),
                  relation: sql`${name}`,
              })
            : sql`false`;

        return sql`(EXISTS (
            SELECT 1 FROM ${Compiler.#current(compilation)} AS ${relationship}
            WHERE ${relationship.objectScope} = ${TableMapping.scopeColumn(source, mapping)}
                AND ${relationship.objectPackageId} = ${definition.packageId}
                AND ${relationship.objectType} = ${definition.name}
                AND ${relationship.objectId} = ${TableMapping.column(source, mapping.id)}
                AND ${relationship.relation} = ${name}
                AND ${Compiler.#accepted(relationship, relation)}
        ) OR ${isMember})`;
    }

    /** Match a field's subject: typed columns, or an identifier of the one type. */
    #field(
        name: string,
        field: Field,
        mapping: TableMapping,
        source: Table,
        authority: Authority,
    ): SQL {
        // match a subject by the type, scope and relation in the row's columns
        if (field.subject !== undefined) {
            return Compiler.#matchTypedField(field, field.subject, source, authority);
        }

        // match the one subject type of a field
        const type = this.#authorizer.onlySubject(mapping.policy, name);
        const subject = {
            packageId: sql`${type.packageId}`,
            type: sql`${type.type}`,
            scope:
                field.scope === undefined
                    ? TableMapping.scopeColumn(source, mapping)
                    : sql`${field.scope}`,
            id: TableMapping.column(source, field.column),
            ...(type.relation === undefined ? {} : { relation: sql`${type.relation}` }),
        };

        return sql`coalesce(${authority.match(subject)}, false)`;
    }

    /** Correlate a related object's permission through a field, ancestor index or relationships. */
    #through(
        expression: ThroughExpression,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // decide a permission of the enclosing scope by the relationships on the scope chain's objects
        if (mapping.policy.definition.relations[expression.relation]?.isScope === true) {
            return this.#enclosing(expression, mapping, compilation);
        }

        // compile the target permission against a distinct alias of each related type
        const related = this.#targets(expression, mapping, compilation);
        const field = mapping.relations[expression.relation];

        // follow relationships to plain objects, each in its own scope
        if (!field) {
            return Compiler.#followRelationships(expression, mapping, source, related, compilation);
        }
        // correlate the parent of the type in the row's columns, for a field with several types
        else if (field.subject !== undefined && !expression.transitive) {
            return Compiler.#followTypedField(field, field.subject, mapping, source, related);
        }
        // correlate the direct parent a field refers to
        else if (!expression.transitive) {
            return Compiler.#followField(field, mapping, source, nth(related, 0));
        }

        // join the indexed ancestors to their protected application rows
        return Compiler.#followAncestors(expression, mapping, source, nth(related, 0), compilation);
    }

    /** Compile the target permission against a distinct alias of each type a relation relates. */
    #targets(
        expression: ThroughExpression,
        mapping: TableMapping,
        compilation: Compilation,
    ): RelationTarget[] {
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);

        return relation.subjects.map((subject) => {
            // alias the related type's table, and match the rows the caller has the permission on
            const target = this.#authorizer.requireMapping(subject);
            const parent = alias(target.table, `access_parent_${compilation.aliases.next++}`);
            const permission = target.policy.permission(expression.permission);
            const predicate = this.#permitted(permission, target, parent, compilation);

            return { subject, target, parent, predicate };
        });
    }

    /** Match a subject by the type, scope and relation in a field's columns. */
    static #matchTypedField(
        field: Field,
        columns: NonNullable<Field["subject"]>,
        source: Table,
        authority: Authority,
    ): SQL {
        // match a plain subject when the columns have no relation
        const subject = {
            packageId: TableMapping.column(source, columns.packageId),
            type: TableMapping.column(source, columns.type),
            scope: TableMapping.column(source, columns.scope),
            id: TableMapping.column(source, field.column),
        };
        if (columns.relation === undefined) {
            return sql`coalesce(${authority.match(subject)}, false)`;
        }

        // match a plain subject or a subject set, as the relation column says
        const relationColumn = TableMapping.column(source, columns.relation);

        return sql`coalesce((
            (${relationColumn} IS NULL AND ${authority.match(subject)})
            OR (${relationColumn} IS NOT NULL AND ${authority.match({ ...subject, relation: relationColumn })})
        ), false)`;
    }

    /** Follow relationships to plain objects, each decided in its own scope. */
    static #followRelationships(
        expression: ThroughExpression,
        mapping: TableMapping,
        source: Table,
        related: readonly RelationTarget[],
        compilation: Compilation,
    ): SQL {
        // match each related type's row the relationship references, in the scope it names
        const scope = TableMapping.scopeColumn(source, mapping);
        const relationship = alias(
            accessRelationship,
            `access_relationship_${compilation.aliases.next++}`,
        );
        const arrows = related.map(
            ({ subject, target, parent, predicate }) => sql`(
                ${relationship.subjectPackageId} = ${subject.packageId}
                AND ${relationship.subjectType} = ${subject.type}
                AND EXISTS (
                    SELECT 1 FROM ${from(parent)}
                    WHERE ${TableMapping.scopeColumn(parent, target)} = ${relationship.subjectScope}
                        AND ${TableMapping.column(parent, target.id)} = ${relationship.subjectId}
                        AND ${Compiler.#reaches(target, parent, compilation)}
                        AND ${predicate}
                )
            )`,
        );

        // read the current relationships of the row through the relation
        const definition = mapping.policy.definition;

        return sql`EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE ${relationship.objectScope} = ${scope}
                AND ${relationship.objectPackageId} = ${definition.packageId}
                AND ${relationship.objectType} = ${definition.name}
                AND ${relationship.objectId} = ${TableMapping.column(source, mapping.id)}
                AND ${relationship.relation} = ${expression.relation}
                AND ${relationship.subjectRelation} IS NULL
                AND ${TupleCondition.where(relationship, TupleCondition.values(compilation.access.context))}
                AND (${sql.join(arrows, sql` OR `)})
        )`;
    }

    /** Correlate the parent of the row's typed columns' type, for a field with several types. */
    static #followTypedField(
        field: Field,
        typed: NonNullable<Field["subject"]>,
        mapping: TableMapping,
        source: Table,
        related: readonly RelationTarget[],
    ): SQL {
        const arrows = related.map(
            ({ subject, target, parent, predicate }) => sql`(
                ${TableMapping.column(source, typed.packageId)} = ${subject.packageId}
                AND ${TableMapping.column(source, typed.type)} = ${subject.type}
                AND EXISTS (
                    SELECT 1 FROM ${from(parent)}
                    WHERE ${Compiler.#inScope(field, mapping, source, target, parent)}
                        AND ${TableMapping.column(parent, target.id)} = ${TableMapping.column(source, field.column)}
                        AND ${predicate}
                )
            )`,
        );

        return arrows.length === 0 ? sql`false` : sql`(${sql.join(arrows, sql` OR `)})`;
    }

    /** Match a field's parent where it lives: anywhere, the universe, or a correlated scope. */
    static #inScope(
        field: Field,
        mapping: TableMapping,
        source: Table,
        target: TableMapping,
        parent: Table,
    ): SQL {
        // match a parent in any scope, and a global type's parent at the universe it lives in
        if (field.scope === ANY_SCOPE || target.scope === undefined) {
            return sql`true`;
        }

        // match the parent in the field's scope, else in the source object's scope
        const scope =
            field.scope === undefined
                ? TableMapping.scopeColumn(source, mapping)
                : sql`${field.scope}`;

        return sql`${TableMapping.scopeColumn(parent, target)} = ${scope}`;
    }

    /** Match a related row in the decided rows' scope, or a handed down one above it. */
    static #reaches(target: TableMapping, parent: Table, compilation: Compilation): SQL {
        // reach a global row always, else a row of the rows' scope or a handed down one above
        if (target.scope === undefined) {
            return sql`true`;
        }
        const scope = TableMapping.scopeColumn(parent, target);
        const above = Compiler.#onSourceChain(scope, compilation);

        return sql`(${scope} = ${compilation.chain.scope} OR (${above} AND ${TableMapping.handedDown(target, parent)}))`;
    }

    /** Correlate a field's direct parent, in the source object's scope or the field's scope. */
    static #followField(
        field: Field,
        mapping: TableMapping,
        source: Table,
        related: RelationTarget,
    ): SQL {
        // match the parent where it lives
        const { target, parent, predicate } = related;

        return sql`EXISTS (
            SELECT 1 FROM ${from(parent)}
            WHERE ${Compiler.#inScope(field, mapping, source, target, parent)}
                AND ${TableMapping.column(parent, target.id)} = ${TableMapping.column(source, field.column)}
                AND ${predicate}
        )`;
    }

    /** Join the indexed ancestors of the source row to their protected application rows. */
    static #followAncestors(
        expression: ThroughExpression,
        mapping: TableMapping,
        source: Table,
        related: RelationTarget,
        compilation: Compilation,
    ): SQL {
        // alias the ancestor index of the relation
        const { target, parent, predicate } = related;
        const targetId = TableMapping.column(parent, target.id);
        const targetScope = TableMapping.scopeColumn(parent, target);
        const tree = TableMapping.tree(mapping, expression.relation);
        const ancestor = alias(tree.ancestors, `access_ancestor_${compilation.aliases.next++}`);

        return sql`EXISTS (
            SELECT 1 FROM ${from(ancestor)} JOIN ${from(parent)}
                ON ${targetId} = ${ancestor.ancestor} AND ${targetScope} = ${ancestor.scope}
            WHERE ${ancestor.scope} = ${TableMapping.scopeColumn(source, mapping)}
                AND ${ancestor.descendant} = ${TableMapping.column(source, mapping.id)}
                AND ${ancestor.depth} > 0
                AND ${predicate}
        )`;
    }

    /** Match a current binding to the authority of a role granting the permission. */
    static #matchBinding(
        permission: PermissionReference,
        compilation: Compilation,
        covers: (relationship: RelationshipColumnMap) => SQL,
    ): SQL {
        // list the roles that grant the permission, or every one but reserved ones
        const relationship = alias(
            accessRelationship,
            `access_binding_${compilation.aliases.next++}`,
        );
        const granted = sql.identifier(GRANTED);
        const key = sql`${granted}.value ->> 0`;
        const isNaming = sql`${key} = ${PermissionReference.key(permission)}`;
        const isGranting = compilation.access.isReserved(permission)
            ? isNaming
            : sql`(${isNaming} OR ${key} = ${OWNING})`;

        return sql`EXISTS (
            SELECT 1 FROM ${Compiler.#current(compilation)} AS ${relationship}
            WHERE EXISTS (
                    SELECT 1 FROM ${jsonElements(sql`${compilation.access.grantedRoles}`, GRANTED)}
                    WHERE ${isGranting}
                        AND ${granted}.value ->> 1 = ${relationship.roleId}
                        AND (${granted}.value ->> 2 IS NULL OR ${granted}.value ->> 2 = ${relationship.id})
                )
                AND ${covers(relationship)}
        )`;
    }

    /** Require a relationship's subject to be of a type its relation accepts. */
    static #accepted(relationship: RelationshipColumnMap, relation: RelationDescription): SQL {
        const types = relation.subjects.map((type) => {
            // match the subject set's relation, or none, and a wildcard only where the type accepts one
            const set =
                type.relation === undefined
                    ? sql`${relationship.subjectRelation} IS NULL`
                    : sql`${relationship.subjectRelation} = ${type.relation}`;
            const wildcard = type.wildcard
                ? sql`(${relationship.subjectId} = '*' OR ${relationship.subjectScope} = '*')`
                : sql`${relationship.subjectId} <> '*' AND ${relationship.subjectScope} <> '*'`;

            return sql`(
                ${relationship.subjectPackageId} = ${type.packageId}
                AND ${relationship.subjectType} = ${type.type}
                AND ${set}
                AND ${wildcard}
            )`;
        });

        return sql`(${sql.join(types, sql` OR `)})`;
    }

    /** Bind a condition's fields to columns or attributes, and placeholders to attributes. */
    static #bindAttributes(
        expression: ConditionExpression,
        mapping: TableMapping,
        source: Table,
        context: AccessContext,
    ): Binding<SQLWrapper> {
        return {
            field: (name) =>
                expression.kind === "resource"
                    ? TableMapping.column(source, TableMapping.attribute(mapping, name))
                    : sql`${AccessContext.attribute(context, name)}`,
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
    }

    /** Match a binding on any object of the decided rows' chain: known, or read from scope rows. */
    static #chainCovers(binding: RelationshipColumnMap, compilation: Compilation): SQL {
        // match each known object
        const { chain } = compilation;
        if (chain.kind === "known") {
            const covers = chain.objects.map((object) => Relationship.on(object, binding));

            return covers.length === 0 ? sql`false` : sql`(${sql.join(covers, sql` OR `)})`;
        }

        // match the universe, or the scope, an ancestor it lists, or the universe above it
        const universe = Relationship.on(Scope.universe, binding);
        const here = alias(Scope.table, `access_scope_${compilation.aliases.next++}`);

        return sql`((${chain.scope} = ${Scope.universe.id} AND ${universe})
            OR EXISTS (
                SELECT 1 FROM ${from(here)}
                WHERE ${here.scope} = ${chain.scope}
                    AND (${Compiler.#isOn(Compiler.#recorded(here), binding)}
                        OR ${listed(binding.objectId, here.ancestors)}
                        OR ${universe})
            ))`;
    }

    /** Match a predicate on the object of the decided rows' scope, known or read from its row. */
    static #nearest(compilation: Compilation, match: (scope: SubjectColumns) => SQL): SQL {
        // match the first known object, none for an empty chain
        const { chain } = compilation;
        if (chain.kind === "known") {
            const [nearest] = chain.objects;

            return nearest === undefined ? sql`false` : match(Compiler.#constant(nearest));
        }

        // match the universe, or the scope's recorded row
        const universe = match(Compiler.#constant(Scope.universe));
        const here = alias(Scope.table, `access_scope_${compilation.aliases.next++}`);

        return sql`((${chain.scope} = ${Scope.universe.id} AND ${universe})
            OR EXISTS (
                SELECT 1 FROM ${from(here)}
                WHERE ${here.scope} = ${chain.scope} AND ${match(Compiler.#recorded(here))}
            ))`;
    }

    /** Match a predicate on the nearest object of a type on a scope's chain. */
    static #nearestOfType(
        scope: SQLWrapper,
        type: Pick<ObjectReference, "packageId" | "type">,
        followed: Pick<ObjectReference, "packageId" | "type"> | undefined,
        compilation: Compilation,
        match: (object: SubjectColumns) => SQL,
    ): SQL {
        // require the scope itself to be of the followed type, when given
        const universe = Compiler.#constant(Scope.universe);
        const here = alias(Scope.table, `access_scope_${compilation.aliases.next++}`);
        const self = Compiler.#recorded(here);
        const isFollowed = (object: SubjectColumns) =>
            followed === undefined ? sql`true` : Compiler.#isType(object, followed);

        // match the universe, which ends every recorded chain
        if (type.packageId === Scope.universe.packageId && type.type === Scope.universe.type) {
            return sql`(((${scope} = ${Scope.universe.id} AND ${isFollowed(universe)})
                OR EXISTS (
                    SELECT 1 FROM ${from(here)}
                    WHERE ${here.scope} = ${scope} AND ${isFollowed(self)}
                ))
                AND ${match(universe)})`;
        }

        // name the ancestors of the scope and the ones nearer than each
        const linkName = `access_link_${compilation.aliases.next++}`;
        const beforeName = `access_before_${compilation.aliases.next++}`;
        const link = sql.identifier(linkName);
        const before = sql.identifier(beforeName);
        const above = alias(Scope.table, `access_above_${compilation.aliases.next++}`);
        const nearer = alias(Scope.table, `access_nearer_${compilation.aliases.next++}`);
        const ancestor = Compiler.#recorded(above);
        const isNearer = sql`NOT EXISTS (
            SELECT 1 FROM ${textElements(here.ancestors, beforeName)}
                JOIN ${from(nearer)} ON ${nearer.scope} = ${before}.value
            WHERE ${before}.key < ${link}.key
                AND ${Compiler.#isType(Compiler.#recorded(nearer), type)}
        )`;

        // match the scope itself, else its nearest ancestor of the type
        return sql`EXISTS (
            SELECT 1 FROM ${from(here)}
            WHERE ${here.scope} = ${scope}
                AND ${isFollowed(self)}
                AND ((${Compiler.#isType(self, type)} AND ${match(self)})
                    OR (NOT ${Compiler.#isType(self, type)} AND EXISTS (
                        SELECT 1 FROM ${textElements(here.ancestors, linkName)}
                            JOIN ${from(above)} ON ${above.scope} = ${link}.value
                        WHERE ${Compiler.#isType(ancestor, type)} AND ${isNearer} AND ${match(ancestor)}
                    )))
        )`;
    }

    /** Match a scope on the decided rows' chain: theirs, an enclosing one, or the universe. */
    static #onSourceChain(scope: SQLWrapper, compilation: Compilation): SQL {
        // match the rows' scope and a known chain's scopes
        const { chain } = compilation;
        if (chain.kind === "known") {
            return inArray(scope, [
                ...new Set([chain.scope, ...chain.objects.map((object) => object.id)]),
            ]);
        }

        // match the rows' scope, the universe, or an ancestor its row lists
        const here = alias(Scope.table, `access_scope_${compilation.aliases.next++}`);

        return sql`(${scope} = ${chain.scope} OR ${scope} = ${Scope.universe.id} OR EXISTS (
            SELECT 1 FROM ${from(here)}
            WHERE ${here.scope} = ${chain.scope} AND ${listed(scope, here.ancestors)}
        ))`;
    }

    /** Read a scope's object from its recorded row. */
    static #recorded(
        row: Record<"packageId" | "type" | "parent" | "scope", SQLWrapper>,
    ): SubjectColumns {
        return { packageId: row.packageId, type: row.type, scope: row.parent, id: row.scope };
    }

    /** Read a known object as constant columns. */
    static #constant(object: ObjectReference): SubjectColumns {
        return {
            packageId: sql`${object.packageId}`,
            type: sql`${object.type}`,
            scope: sql`${object.scope}`,
            id: sql`${object.id}`,
        };
    }

    /** Match a relationship on an object. */
    static #isOn(object: SubjectColumns, relationship: RelationshipColumnMap): SQL {
        return sql`(
            ${relationship.objectPackageId} = ${object.packageId}
            AND ${relationship.objectType} = ${object.type}
            AND ${relationship.objectScope} = ${object.scope}
            AND ${relationship.objectId} = ${object.id}
        )`;
    }

    /** Match an object of a type. */
    static #isType(object: SubjectColumns, type: Pick<ObjectReference, "packageId" | "type">): SQL {
        return sql`(${object.packageId} = ${type.packageId} AND ${object.type} = ${type.type})`;
    }

    /** Read the relationships current for a compiled authority, once per statement. */
    static #current(compilation: Compilation): CommonTable {
        // key the query by everything it reads of the request and the authority
        const { access, authority } = compilation;
        const bindings = TupleCondition.bindings(access.context);
        const name = `access_current_${hashName(JSON.stringify([bindings, authority.subjects]))}`;

        // select the relationships with passing conditions and the authority as subject
        const relationship = alias(accessRelationship, "access_current");

        return sql.common(
            name,
            sql`SELECT ${relationship}.* FROM ${from(relationship)}
                WHERE ${TupleCondition.where(relationship, TupleCondition.values(access.context))}
                    AND ${authority.member(relationship)}`,
        );
    }
}
