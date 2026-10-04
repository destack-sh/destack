import { ObjectReference, Subject } from "@destack/sync";
import {
    alias,
    from,
    inArray,
    sql,
    type SQL,
    type SQLWrapper,
    type Table,
    Predicate,
    type Binding,
} from "@destack/db";
import { aligned } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { type PermissionReference } from "../policy/policy.ts";
import {
    CONTAINED,
    type AccessExpression,
    type ConditionExpression,
} from "../policy/expression.ts";
import { accepts, type RelationDefinition, type SubjectType } from "../policy/subject.ts";
import { AccessContext } from "../context/context.ts";
import { Restriction } from "../context/restriction.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship, type RelationshipColumnMap } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { Authority } from "./authority.ts";
import type { Authorizer } from "./authorizer.ts";
import { GrantCondition } from "./condition.ts";
import { ANY_SCOPE, column, TableMapping } from "./mapping.ts";

/** The resolved caller, the authority compiled and the alias allocation one predicate shares. */
interface Compilation {
    /** The caller's resolved access in the query's scope. */
    readonly access: Access;
    /** The authority whose grants the predicate matches: the represented subject or one delegate. */
    readonly authority: Authority;
    /** The next free alias number. */
    readonly aliases: { next: number };
}

/** A relation kept in a field of a mapped table. */
type Field = TableMapping["relations"][string];

/** An expression following a relation to a related object's permission. */
type ThroughExpression = Extract<AccessExpression, { kind: "through" }>;

/** A type a relation relates, with an alias of its table and its permission compiled against that alias. */
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

/**
 * Compile policies to SQL over the tables their objects live in, for a caller resolved in one scope.
 *
 * Every predicate requires each of the caller's authorities to have the permission, as the grants decide in memory.
 */
export class Compiler {
    /** The authorizer whose policies and mappings the compiler reads. */
    readonly #authorizer: Authorizer;

    /** Compile the policies an authorizer registered. */
    constructor(authorizer: Authorizer) {
        this.#authorizer = authorizer;
    }

    /** Restrict the rows of a table, the mapped one by default, to those the caller has a permission on in its scope. */
    where(permission: PermissionReference, access: Access, source?: Table): SQL {
        // deny a request its gate refuses
        const mapping = this.#authorizer.mapping(permission);
        const table = source ?? mapping.table;
        if (access.blocked(permission) !== undefined) {
            return sql`false`;
        }

        // evaluate every authority, and constrain restricted credentials to the objects they allow
        const predicates = this.#authorities(access, (compilation) =>
            this.#permission(permission, mapping, table, compilation),
        );
        if (access.context.permissions !== undefined) {
            predicates.push(
                this.#restricted(permission, column(table, mapping.id), access, { mapping, table }),
            );
        }

        // select the mapped type's rows of a table several types share
        if (mapping.isShared === true) {
            const definition = mapping.policy.definition;
            predicates.push(
                sql`${column(table, "packageId")} = ${definition.packageId} AND ${column(table, "type")} = ${definition.name}`,
            );
        }

        // select the rows of the scope, and the scope's own row in the scope containing it
        const object = access.scopeObject(mapping);
        const scope = TableMapping.scopeColumn(table, mapping);
        const located =
            object === undefined
                ? sql`${scope} = ${access.scope}`
                : sql`(${scope} = ${access.scope} OR (${scope} = ${object.scope} AND ${column(table, mapping.id)} = ${object.id}))`;

        return sql`(${located} AND (${sql.join(predicates, sql` AND `)}))`;
    }

    /** Match one object in its scope, or the scope's own object, if the caller has a permission on it. */
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
            const allowed = this.#restricted(permission, column(row, mapping.id), access, {
                mapping,
                table: row,
            });
            const granted = this.#permission(permission, mapping, row, compilation);

            return sql`(${allowed} AND ${granted})`;
        });
    }

    /** Match one object of the resolved scope when every authority satisfies a predicate on its row. */
    #target(
        target: ObjectReference,
        access: Access,
        evaluate: (mapping: TableMapping, row: Table, compilation: Compilation) => SQL,
    ): SQL {
        // require the target in the resolved scope or to be the scope, and evaluate each authority on its row
        const object = access.scopes[0];
        const isScope =
            object !== undefined && ObjectReference.key(object) === ObjectReference.key(target);
        if (target.scope !== access.scope && !isScope) {
            throw new AccessError("INVALID_CONTEXT", "target lies outside the resolved scope");
        }
        const mapping = this.#authorizer.mapping(target);
        const row = alias(mapping.table, "access_target");
        const predicates = this.#authorities(access, (compilation) =>
            evaluate(mapping, row, compilation),
        );

        return sql`EXISTS (
            SELECT 1 FROM ${from(row)}
            WHERE ${column(row, mapping.id)} = ${target.id}
                AND ${TableMapping.scopeColumn(row, mapping)} = ${target.scope}
                AND ${sql.join(predicates, sql` AND `)}
        )`;
    }

    /**
     * Match the objects the credential allows a permission on, directly or through its source.
     *
     * A derivation through a relation needs the objects' rows to find the related objects.
     */
    #restricted(
        permission: PermissionReference,
        id: SQLWrapper,
        access: Access,
        stored?: { readonly mapping: TableMapping; readonly table: Table },
    ): SQL {
        // match the objects the credential names
        const selected = Restriction.select(permission, access.scope, access.context);
        if (selected === "every") {
            return sql`true`;
        }
        const named = selected.length === 0 ? sql`false` : inArray(id, selected);

        // match a derivation as its source is matched: on the same object, or on the related one each row names
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
            const allowed = this.#restricted(derived, column(table, field.column), access);

            // accept a single-typed relation in the caller's scope
            if (typed === undefined) {
                return field.scope === undefined || field.scope === access.scope
                    ? allowed
                    : sql`false`;
            }
            // accept a typed relation naming this type in the caller's scope
            else {
                const conditions = [
                    sql`${column(table, typed.packageId)} = ${subject.packageId}`,
                    sql`${column(table, typed.type)} = ${subject.type}`,
                    sql`${column(table, typed.scope)} = ${access.scope}`,
                    allowed,
                ];

                return sql`(${sql.join(conditions, sql` AND `)})`;
            }
        });

        return sql`(${sql.join([named, ...arrows], sql` OR `)})`;
    }

    /** Evaluate every authority of the caller with one alias allocation, the represented subject first. */
    #authorities(access: Access, evaluate: (compilation: Compilation) => SQL): SQL[] {
        const aliases = { next: 0 };

        return access.authorities.map((authority) => evaluate({ access, authority, aliases }));
    }

    /** Permit through the permission's expression or a role bound on the row, an ancestor or an enclosing scope. */
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

    /** Match a current role binding granting the permission to the authority on a covering object. */
    #bound(
        permission: PermissionReference,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        return matchBinding(compilation.access.granting(permission), compilation, (relationship) =>
            this.#covers(relationship, mapping, source, compilation),
        );
    }

    /** Match a binding on the row itself, an owning ancestor, or a scope enclosing the query's scope. */
    #covers(
        binding: RelationshipColumnMap,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // match the row itself or an object of the scope chain
        const id = column(source, mapping.id);
        const scope = TableMapping.scopeColumn(source, mapping);
        const definition = mapping.policy.definition;
        const typed = sql`
            ${binding.objectScope} = ${scope}
            AND ${binding.packageId} = ${definition.packageId}
            AND ${binding.type} = ${definition.name}
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
            const target = this.#authorizer.mapping(subject);
            const parent = alias(target.table, `access_owner_${compilation.aliases.next++}`);
            covering.push(sql`EXISTS (
                SELECT 1 FROM ${from(parent)}
                WHERE ${column(parent, target.id)} = ${column(source, TableMapping.field(mapping, mapping.parent).column)}
                    AND ${TableMapping.scopeColumn(parent, target)} = ${scope}
                    AND ${this.#covers(binding, target, parent, compilation)}
            )`);
        }

        return sql`(${sql.join(covering, sql` OR `)})`;
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
                // grant nothing by attributes a database with only the object's scope row cannot read
                const predicate = mapping.policy.resolve(expression);
                if (expression.kind === "resource" && !TableMapping.decides(mapping, predicate)) {
                    return sql`false`;
                }
                const binding = bindAttributes(
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
        }
    }

    /** Match a row when the authority lives inside its scope, or inside the scope it is, lending nothing to delegates. */
    static #contained(
        expression: Extract<AccessExpression, { kind: "contained" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // lend nothing to delegates, and admit only the named principal types when given
        const { authority, access } = compilation;
        const isTyped =
            expression.principals === undefined ||
            expression.principals.some((type) =>
                authority.isMember({ ...type, scope: "*", id: "*" }),
            );
        if (authority.delegator !== undefined || !isTyped) {
            return sql`false`;
        }

        // match a scope's row by its own containment set
        const definition = mapping.policy.definition;
        if (definition.scope === true) {
            return authority.match({
                packageId: sql`${definition.packageId}`,
                type: sql`${definition.name}`,
                scope: TableMapping.scopeColumn(source, mapping),
                id: column(source, mapping.id),
                relation: sql`${CONTAINED}`,
            });
        }

        // match another row living in the resolved scope by that scope's containment set
        const [scope] = access.scopes;
        const isInside =
            scope !== undefined && authority.isMember({ ...scope, relation: CONTAINED });

        return isInside
            ? sql`${TableMapping.scopeColumn(source, mapping)} = ${scope.id}`
            : sql`false`;
    }

    /** Match a current relationship of the authority on an object of the scope chain granting the enclosing scope's permission. */
    #enclosing(
        expression: ThroughExpression,
        mapping: TableMapping,
        compilation: Compilation,
    ): SQL {
        // find each type along the chain and the relations granting the permission on its object
        const scope = this.#authorizer.onlySubject(mapping.policy, expression.relation);
        const steps = this.#authorizer
            .inherited({
                packageId: scope.packageId,
                type: scope.type,
                name: expression.permission,
            })
            .flatMap(({ type, relations }) => {
                const object = compilation.access.scopes.find(
                    (link) => link.packageId === type.packageId && link.type === type.type,
                );

                return object === undefined || relations.length === 0
                    ? []
                    : [{ object, relations }];
            });
        if (steps.length === 0) {
            return sql`false`;
        }

        // match one current relationship of the authority on any of them
        const relationship = alias(
            accessRelationship,
            `access_enclosing_${compilation.aliases.next++}`,
        );
        const { access, authority } = compilation;
        const matches = steps.map(
            ({ object, relations }) =>
                sql`(${Relationship.on(object, relationship)} AND ${inArray(relationship.relation, [...relations])})`,
        );

        return sql`EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE (${sql.join(matches, sql` OR `)})
                AND ${GrantCondition.where(relationship, GrantCondition.values(access.context, authority.delegator))}
                AND ${authority.member(relationship)}
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
                    ${column(source, columns.packageId)} = ${definition.packageId}
                    AND ${column(source, columns.type)} = ${definition.name}
                    AND EXISTS (
                        SELECT 1 FROM ${from(row)}
                        WHERE ${column(row, target.id)} = ${column(source, columns.id)}
                            AND ${TableMapping.scopeColumn(row, target)} = ${column(source, columns.scope)}
                            AND ${this.#permission(grant, target, row, compilation)}
                    )
                )`,
            ];
        });

        return branches.length === 0 ? sql`false` : sql`(${sql.join(branches, sql` OR `)})`;
    }

    /** Match whoever sees the relationships of the object a row references, a concealed relation's only through its grant permission. */
    #readers(
        expression: Extract<AccessExpression, { kind: "readers" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // compile each type's visibility against its own alias of the referenced row
        const columns = TableMapping.reference(mapping, expression.reference);
        const related = column(source, expression.relation);
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
                    ${column(source, columns.packageId)} = ${target.policy.definition.packageId}
                    AND ${column(source, columns.type)} = ${target.policy.definition.name}
                    AND EXISTS (
                        SELECT 1 FROM ${from(row)}
                        WHERE ${column(row, target.id)} = ${column(source, columns.id)}
                            AND ${TableMapping.scopeColumn(row, target)} = ${column(source, columns.scope)}
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
            return this.#field(name, field, relation, mapping, source, compilation.authority);
        }

        // match current relationships of subject types the relation accepts
        const relationship = alias(
            accessRelationship,
            `access_relationship_${compilation.aliases.next++}`,
        );
        const definition = mapping.policy.definition;
        const authority = compilation.authority;

        return sql`EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE ${relationship.objectScope} = ${TableMapping.scopeColumn(source, mapping)}
                AND ${relationship.packageId} = ${definition.packageId}
                AND ${relationship.type} = ${definition.name}
                AND ${relationship.objectId} = ${column(source, mapping.id)}
                AND ${relationship.relation} = ${name}
                AND ${GrantCondition.where(relationship, GrantCondition.values(compilation.access.context, authority.delegator))}
                AND ${accepted(relationship, relation)}
                AND ${authority.member(relationship)}
        )`;
    }

    /** Match the subject a field keeps: a subject key, typed columns, or an identifier of the relation's one type. */
    #field(
        name: string,
        field: Field,
        relation: RelationDefinition,
        mapping: TableMapping,
        source: Table,
        authority: Authority,
    ): SQL {
        // lend nothing in a field to a delegate
        if (authority.delegator !== undefined) {
            return sql`false`;
        }
        // match a subject key in the column against the authority's subjects the relation accepts
        else if (field.isKey) {
            const keys = authority.subjects
                .filter((subject) => accepts(relation, subject))
                .map((subject) => Subject.key(subject));

            return keys.length === 0
                ? sql`false`
                : sql`coalesce(${inArray(column(source, field.column), keys)}, false)`;
        }
        // match a subject by the type, scope and relation in the row's columns
        else if (field.subject !== undefined) {
            return matchTypedField(field, field.subject, source, authority);
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
            id: column(source, field.column),
            ...(type.relation === undefined ? {} : { relation: sql`${type.relation}` }),
        };

        return sql`coalesce(${authority.match(subject)}, false)`;
    }

    /** Correlate a related object's permission through a field, an ancestor index, or relationships. */
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

        // follow relationships to plain objects in the same scope without delegation
        if (!field) {
            return followRelationships(expression, mapping, source, related, compilation);
        }
        // correlate the parent of whichever type the row's columns name, for a field with several types
        else if (field.subject !== undefined && !expression.transitive) {
            return followTypedField(field, field.subject, mapping, source, related);
        }
        // correlate the direct parent a field refers to
        else if (!expression.transitive) {
            return followField(field, mapping, source, aligned(related, 0));
        }

        // join the indexed ancestors to their protected application rows
        return followAncestors(expression, mapping, source, aligned(related, 0), compilation);
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
            const target = this.#authorizer.mapping(subject);
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
}

/** Match a subject by the type, scope and relation a field's columns keep. */
function matchTypedField(
    field: Field,
    columns: NonNullable<Field["subject"]>,
    source: Table,
    authority: Authority,
): SQL {
    // match a plain subject when the columns keep no relation
    const subject = {
        packageId: column(source, columns.packageId),
        type: column(source, columns.type),
        scope: column(source, columns.scope),
        id: column(source, field.column),
    };
    if (columns.relation === undefined) {
        return sql`coalesce(${authority.match(subject)}, false)`;
    }

    // match a plain subject or a subject set, as the relation column says
    const relationColumn = column(source, columns.relation);

    return sql`coalesce((
        (${relationColumn} IS NULL AND ${authority.match(subject)})
        OR (${relationColumn} IS NOT NULL AND ${authority.match({ ...subject, relation: relationColumn })})
    ), false)`;
}

/** Follow relationships to plain objects in the same scope without delegation. */
function followRelationships(
    expression: ThroughExpression,
    mapping: TableMapping,
    source: Table,
    related: readonly RelationTarget[],
    compilation: Compilation,
): SQL {
    // match each related type's row the relationship names
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
                    AND ${column(parent, target.id)} = ${relationship.subjectId}
                    AND ${predicate}
            )
        )`,
    );

    // read the current relationships of the row through the relation
    const definition = mapping.policy.definition;

    return sql`EXISTS (
        SELECT 1 FROM ${from(relationship)}
        WHERE ${relationship.objectScope} = ${scope}
            AND ${relationship.packageId} = ${definition.packageId}
            AND ${relationship.type} = ${definition.name}
            AND ${relationship.objectId} = ${column(source, mapping.id)}
            AND ${relationship.relation} = ${expression.relation}
            AND ${relationship.subjectScope} = ${scope}
            AND ${relationship.subjectRelation} IS NULL
            AND ${GrantCondition.where(relationship, GrantCondition.values(compilation.access.context))}
            AND (${sql.join(arrows, sql` OR `)})
    )`;
}

/** Correlate the parent of whichever type the row's columns name, for a field with several types. */
function followTypedField(
    field: Field,
    typed: NonNullable<Field["subject"]>,
    mapping: TableMapping,
    source: Table,
    related: readonly RelationTarget[],
): SQL {
    const scope = TableMapping.scopeColumn(source, mapping);
    const arrows = related.map(
        ({ subject, target, parent, predicate }) => sql`(
            ${column(source, typed.packageId)} = ${subject.packageId}
            AND ${column(source, typed.type)} = ${subject.type}
            AND EXISTS (
                SELECT 1 FROM ${from(parent)}
                WHERE ${TableMapping.scopeColumn(parent, target)} = ${scope}
                    AND ${column(parent, target.id)} = ${column(source, field.column)}
                    AND ${predicate}
            )
        )`,
    );

    return arrows.length === 0 ? sql`false` : sql`(${sql.join(arrows, sql` OR `)})`;
}

/** Correlate the direct parent a field refers to, in the source object's scope or the scope the field gives. */
function followField(
    field: Field,
    mapping: TableMapping,
    source: Table,
    related: RelationTarget,
): SQL {
    // match the parent in any scope, the field's scope or the source object's scope
    const { target, parent, predicate } = related;
    const scope = TableMapping.scopeColumn(source, mapping);
    const targetScope = TableMapping.scopeColumn(parent, target);
    const inScope =
        field.scope === ANY_SCOPE
            ? sql`true`
            : sql`${targetScope} = ${field.scope === undefined ? scope : sql`${field.scope}`}`;

    return sql`EXISTS (
        SELECT 1 FROM ${from(parent)}
        WHERE ${inScope}
            AND ${column(parent, target.id)} = ${column(source, field.column)}
            AND ${predicate}
    )`;
}

/** Join the indexed ancestors of the source row to their protected application rows. */
function followAncestors(
    expression: ThroughExpression,
    mapping: TableMapping,
    source: Table,
    related: RelationTarget,
    compilation: Compilation,
): SQL {
    // alias the ancestor index of the relation
    const { target, parent, predicate } = related;
    const targetId = column(parent, target.id);
    const targetScope = TableMapping.scopeColumn(parent, target);
    const tree = TableMapping.tree(mapping, expression.relation);
    const ancestor = alias(tree.ancestors, `access_ancestor_${compilation.aliases.next++}`);

    return sql`EXISTS (
        SELECT 1 FROM ${from(ancestor)} JOIN ${from(parent)}
            ON ${targetId} = ${ancestor.ancestor} AND ${targetScope} = ${ancestor.scope}
        WHERE ${ancestor.scope} = ${TableMapping.scopeColumn(source, mapping)}
            AND ${ancestor.descendant} = ${column(source, mapping.id)}
            AND ${ancestor.depth} > 0
            AND ${predicate}
    )`;
}

/** Match a current binding of one of the roles to the compiled authority on a covered object. */
function matchBinding(
    roles: readonly string[],
    compilation: Compilation,
    covers: (relationship: RelationshipColumnMap) => SQL,
): SQL {
    // skip bindings no role could satisfy
    if (roles.length === 0) {
        return sql`false`;
    }
    const relationship = alias(accessRelationship, `access_binding_${compilation.aliases.next++}`);
    const { access, authority } = compilation;

    return sql`EXISTS (
        SELECT 1 FROM ${from(relationship)}
        WHERE ${inArray(relationship.roleId, roles)}
            AND ${GrantCondition.where(relationship, GrantCondition.values(access.context, authority.delegator))}
            AND ${authority.member(relationship)}
            AND ${covers(relationship)}
    )`;
}

/** Require a relationship's subject to be of a type its relation accepts. */
function accepted(relationship: RelationshipColumnMap, relation: RelationDefinition): SQL {
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

/** Bind a condition's fields to a mapping's columns or the request's attributes, and its placeholders to request attributes. */
function bindAttributes(
    expression: ConditionExpression,
    mapping: TableMapping,
    source: Table,
    context: AccessContext,
): Binding<SQLWrapper> {
    return {
        field: (name) =>
            expression.kind === "resource"
                ? column(source, TableMapping.attribute(mapping, name))
                : sql`${AccessContext.attribute(context, name)}`,
        placeholder: (name) => AccessContext.attribute(context, name),
        exists: (via) => {
            throw new AccessError(
                "INVALID_DECLARATION",
                `policy conditions follow no relations: ${via}`,
            );
        },
    };
}
