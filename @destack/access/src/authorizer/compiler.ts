import { ObjectReference, type ObjectTypeReference, Scope } from "@destack/sync";
import {
    alias,
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

/** The resolved caller, compiled authority and alias allocation one predicate shares. */
interface Compilation {
    /** The caller's resolved access in the query's scope. */
    readonly access: Access;
    /** The authority whose tuples the predicate matches: the represented subject. */
    readonly authority: Authority;
    /** The next free alias number. */
    readonly aliases: { next: number };
}

/** The name of the list element a binding's role is read from. */
const GRANTED = "access_granted";

/** A relation kept in a field of a mapped table. */
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

        // evaluate the authority, and constrain a grant to the objects it allows
        const predicates = [
            this.#permission(permission, mapping, table, Compiler.#compilation(access)),
        ];
        if (access.context.grant !== undefined) {
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

        // select the rows of the scope, and the scope's own row in the scope containing it
        const object = access.scopeObject(mapping);
        const scope = TableMapping.scopeColumn(table, mapping);
        const located =
            object === undefined
                ? sql`${scope} = ${access.scope}`
                : sql`(${scope} = ${access.scope} OR (${scope} = ${object.scope} AND ${TableMapping.column(table, mapping.id)} = ${object.id}))`;

        return sql`(${located} AND (${sql.join(predicates, sql` AND `)}))`;
    }

    /** Match one object in its scope, or the scope's object, if the caller has a permission. */
    permits(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        // deny a request that its elevation, suspension or credential refuses
        const isSameType =
            permission.packageId === target.packageId && permission.type === target.type;
        if (
            access.blocked(permission) !== undefined ||
            (!isSameType && !access.admits(permission, target.id))
        ) {
            return sql`false`;
        }

        // evaluate another type's permission through roles
        return this.#target(target, access, (mapping, row, compilation) => {
            if (!isSameType) {
                return this.#bound(permission, mapping, row, compilation);
            }

            // evaluate the target's own permission where the credential allows it
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
            throw new AccessError("INVALID_CONTEXT", "target is outside the resolved scope");
        }
        const mapping = this.#authorizer.requireMapping(target);
        const compilation = Compiler.#compilation(access);
        const row = alias(mapping.table, "access_target");
        const isTarget = (
            source: Table,
        ) => sql`${TableMapping.column(source, mapping.id)} = ${target.id}
            AND ${TableMapping.scopeColumn(source, mapping)} = ${target.scope}`;
        const kept = sql`EXISTS (
            SELECT 1 FROM ${from(row)}
            WHERE ${isTarget(row)} AND ${evaluate(mapping, row, compilation)}
        )`;
        if (mapping.policy.definition.scope !== true) {
            return kept;
        }

        // decide a scope whose object row lives elsewhere on its ancestry row, which its chain keeps
        const missing = alias(mapping.table, "access_missing");
        const ancestry = TableMapping.ancestry(mapping.policy);
        const standing = alias(Scope.table, "access_ancestry");

        return sql`(${kept} OR (
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
        const selected = GrantPermission.select(
            permission,
            access.scope,
            access.context.grant?.permissions,
        );
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

    /** Start compiling for a caller's authority with a fresh alias allocation. */
    static #compilation(access: Access): Compilation {
        return { access, authority: access.authority, aliases: { next: 0 } };
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

    /** Match a current role binding granting the permission to the authority on a cover. */
    #bound(
        permission: PermissionReference,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        return Compiler.#matchBinding(permission, compilation, (relationship) =>
            this.#covers(relationship, mapping, source, compilation),
        );
    }

    /** Match a binding on the row, an owning ancestor, or a scope enclosing the query's scope. */
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
            ...compilation.access.scopes.map((object) => Relationship.on(object, binding)),
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
        // climb to the owning parent and match it and its own ancestors
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

        // match the owner, its own coverage and the scopes enclosing it
        if (mapping.owner !== undefined) {
            covering.push(this.#owned(binding, mapping, source, mapping.owner, compilation));
        }

        return sql`(${sql.join(covering, sql` OR `)})`;
    }

    /** Match a binding on a row's owner, covering the owner, or on a scope enclosing the owner. */
    #owned(
        binding: RelationshipColumnMap,
        mapping: TableMapping,
        source: Table,
        relation: string,
        compilation: Compilation,
    ): SQL {
        // find the owner the relation's field refers to
        const subject = this.#authorizer.onlySubject(mapping.policy, relation);
        const target = this.#authorizer.requireMapping(subject);
        const referred = TableMapping.column(source, TableMapping.field(mapping, relation).column);
        const enclosing = alias(Scope.table, `access_enclosing_${compilation.aliases.next++}`);

        // match a scope owner and its enclosing scopes by its scope row only
        if (target.policy.definition.scope === true) {
            return sql`(
                ${binding.objectId} = ${referred}
                OR EXISTS (
                    SELECT 1 FROM ${from(enclosing)}
                    WHERE ${enclosing.scope} = ${referred}
                        AND ${listed(binding.objectId, enclosing.ancestors)}
                )
            )`;
        }

        // match any other owner's coverage, its scope and the scopes enclosing that
        const owner = alias(target.table, `access_owner_${compilation.aliases.next++}`);
        const governing = TableMapping.scopeColumn(owner, target);

        return sql`EXISTS (
            SELECT 1 FROM ${from(owner)}
            WHERE ${TableMapping.column(owner, target.id)} = ${referred}
                AND (
                    ${this.#covers(binding, target, owner, compilation)}
                    OR ${binding.objectId} = ${governing}
                    OR EXISTS (
                        SELECT 1 FROM ${from(enclosing)}
                        WHERE ${enclosing.scope} = ${governing}
                            AND ${listed(binding.objectId, enclosing.ancestors)}
                    )
                )
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
            case "self":
                return Compiler.#self(mapping, source, compilation);
        }
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
    static #admits(
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
        // admit only the named principal types when given
        const { authority, access } = compilation;
        if (!Compiler.#admits(authority, expression.principals)) {
            return sql`false`;
        }

        // match a scope's row by its own containment set, a global scope's also by its universe reference
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
            const own = set(TableMapping.scopeColumn(source, mapping));

            return definition.isGlobal === true
                ? sql`(${own} OR ${set(sql`${Scope.universe.id}`)})`
                : own;
        }

        // match another row living in the resolved scope by that scope's containment set
        const [scope] = access.scopes;
        const isInside =
            scope !== undefined && authority.isMember({ ...scope, relation: CONTAINED });

        return isInside
            ? sql`${TableMapping.scopeColumn(source, mapping)} = ${scope.id}`
            : sql`false`;
    }

    /** Match the authority by a current relationship or containment on a chain object. */
    #enclosing(
        expression: ThroughExpression,
        mapping: TableMapping,
        compilation: Compilation,
    ): SQL {
        // find the objects of the chain deciding the enclosing scope's permission
        const steps = this.#authorizer.chainSteps(
            mapping.policy,
            expression,
            compilation.access.scopes,
        );

        // match a chain scope with the authority in its containment set, for admitted types
        const { authority } = compilation;
        const isContained = steps.some(
            ({ object, contained }) =>
                contained !== undefined &&
                authority.isMember({ ...object, relation: CONTAINED }) &&
                Compiler.#admits(authority, contained.principals),
        );
        if (isContained) {
            return sql`true`;
        }

        // match one current relationship of the authority on any of them
        const relationship = alias(
            accessRelationship,
            `access_enclosing_${compilation.aliases.next++}`,
        );
        const matches = steps
            .filter(({ relations }) => relations.length > 0)
            .map(
                ({ object, relations }) =>
                    sql`(${Relationship.on(object, relationship)} AND ${inArray(relationship.relation, [...relations])})`,
            );
        if (matches.length === 0) {
            return sql`false`;
        }

        return sql`EXISTS (
            SELECT 1 FROM ${Compiler.#current(compilation)} AS ${relationship}
            WHERE (${sql.join(matches, sql` OR `)})
        )`;
    }

    /** Match rows with a referenced object the caller has the grant permission on. */
    #granters(
        reference: string,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // compile each granting type's grant permission against its own alias of the referenced row
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
                            AND ${this.#permission(grant, target, row, compilation)}
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
        // compile each type's visibility against its own alias of the referenced row
        const columns = TableMapping.reference(mapping, expression.reference);
        const related = TableMapping.column(source, expression.relation);
        const branches = this.#authorizer.mappings().flatMap((target) => {
            // skip the row's own type, whose rows relate other objects
            if (target.policy === mapping.policy) {
                return [];
            }

            // see a type's open relations through its relationship read or grant permission
            const row = alias(target.table, `access_referenced_${compilation.aliases.next++}`);
            const permitted = (name: string | undefined) =>
                name === undefined
                    ? sql`false`
                    : this.#permission(target.policy.permission(name), target, row, compilation);
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
        // read the relation and whether a field keeps it
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

        // match the relation sets the authority holds
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
        const related = this.#related(expression, mapping, compilation);
        const field = mapping.relations[expression.relation];

        // follow relationships to plain objects in the same scope
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
        return Compiler.#followAncestors(
            expression,
            mapping,
            source,
            nth(related, 0),
            compilation,
        );
    }

    /** Compile the target permission against a distinct alias of each type a relation relates. */
    #related(
        expression: ThroughExpression,
        mapping: TableMapping,
        compilation: Compilation,
    ): RelationTarget[] {
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);

        return relation.subjects.map((subject) => {
            // alias the related type's table for its own correlated permission
            const target = this.#authorizer.requireMapping(subject);
            const parent = alias(target.table, `access_parent_${compilation.aliases.next++}`);
            const predicate = this.#permission(
                target.policy.permission(expression.permission),
                target,
                parent,
                compilation,
            );

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

    /** Follow relationships to plain objects in the same scope. */
    static #followRelationships(
        expression: ThroughExpression,
        mapping: TableMapping,
        source: Table,
        related: readonly RelationTarget[],
        compilation: Compilation,
    ): SQL {
        // match each related type's row the relationship references
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
                    WHERE ${TableMapping.scopeColumn(parent, target)} = ${scope}
                        AND ${TableMapping.column(parent, target.id)} = ${relationship.subjectId}
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
                AND ${relationship.subjectScope} = ${scope}
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
                throw new AccessError(
                    "INVALID_DECLARATION",
                    `policy conditions cannot follow relations: ${via}`,
                );
            },
            search: () => {
                throw new AccessError("INVALID_DECLARATION", "policy conditions cannot search");
            },
        };
    }

    /** Read the relationships current for a compiled authority, once per statement. */
    static #current(compilation: Compilation): CommonTable {
        // key the query by everything it reads of the request and the authority
        const { access, authority } = compilation;
        const bindings = TupleCondition.bindings(access.context);
        const name = `access_current_${hashName(JSON.stringify([bindings, authority.subjects]))}`;

        // keep the relationships with passing conditions and the authority as subject
        const relationship = alias(accessRelationship, "access_current");

        return sql.common(
            name,
            sql`SELECT ${relationship}.* FROM ${from(relationship)}
                WHERE ${TupleCondition.where(relationship, TupleCondition.values(access.context))}
                    AND ${authority.member(relationship)}`,
        );
    }
}
