import { type ObjectReference } from "@destack/sync";
import { alias, inArray, sql, type SQL, type SQLWrapper, type Table } from "@destack/db";
import { Condition, type Binding } from "@destack/db/query";
import { AccessError } from "../error/index.ts";
import { objectKey, type PermissionReference } from "../policy/policy.ts";
import type { AccessExpression } from "../policy/expression.ts";
import { accepts, subjectKey, type RelationDefinition } from "../policy/subject.ts";
import { requireAttribute, type AccessContext } from "../context/context.ts";
import { Restriction } from "../context/restriction.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship, type RelationshipColumnMap } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { Authority } from "./authority.ts";
import type { Authorizer } from "./authorizer.ts";
import { GrantCondition } from "./condition.ts";
import { column, from, TableMapping } from "./mapping.ts";

/** The resolved caller, the authority compiled and the alias allocation one predicate shares. */
interface Compilation {
    /** The caller's resolved access in the query's scope. */
    readonly access: Access;
    /** The authority whose grants the predicate matches: the represented subject or one delegate. */
    readonly authority: Authority;
    /** The next free alias number. */
    readonly aliases: { next: number };
}

/**
 * Compile policies to SQL over the tables their objects live in, for a caller resolved in one scope.
 *
 * Every predicate requires each of the caller's authorities to hold the permission, as the grants decide in memory.
 */
export class Compiler {
    /** The authorizer whose policies and mappings the compiler reads. */
    readonly #authorizer: Authorizer;

    /** Compile the policies an authorizer registered. */
    constructor(authorizer: Authorizer) {
        this.#authorizer = authorizer;
    }

    /** Restrict the rows of a table, the mapped one by default, to those the caller holds a permission on in its scope. */
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
        if (mapping.isShared) {
            const definition = mapping.policy.definition;
            predicates.push(
                sql`${column(table, "packageId")} = ${definition.packageId} AND ${column(table, "type")} = ${definition.name}`,
            );
        }

        // select the rows of the scope, and the scope's own row in the scope containing it
        const own = access.own(mapping);
        const scope = TableMapping.scopeColumn(table, mapping);
        const located =
            own === undefined
                ? sql`${scope} = ${access.scope}`
                : sql`(${scope} = ${access.scope} OR (${scope} = ${own.scope} AND ${column(table, mapping.id)} = ${own.id}))`;

        return sql`(${located} AND (${sql.join(predicates, sql` AND `)}))`;
    }

    /** Match one object in its scope, or the scope's own object, if the caller holds a permission on it. */
    holds(permission: PermissionReference, target: ObjectReference, access: Access): SQL {
        // deny a request that its elevation, suspension or credential refuses
        const isOwn = permission.packageId === target.packageId && permission.type === target.type;
        if (
            access.blocked(permission) !== undefined ||
            (!isOwn && !access.admits(permission, target.id))
        ) {
            return sql`false`;
        }

        // evaluate another type's permission through roles
        return this.#target(target, access, (mapping, row, compilation) => {
            if (!isOwn) {
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
        const own = access.scopes[0];
        const isScope = own !== undefined && objectKey(own) === objectKey(target);
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
        const field = mapping.relations[source.relation]!;
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
        return binding(compilation.access.granting(permission), compilation, (relationship) =>
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
            const [subject] = this.#authorizer.relation(mapping.policy, mapping.parent).subjects;
            const target = this.#authorizer.mapping(subject!);
            const parent = alias(target.table, `access_owner_${compilation.aliases.next++}`);
            covering.push(sql`EXISTS (
                SELECT 1 FROM ${from(parent)}
                WHERE ${column(parent, target.id)} = ${column(source, mapping.relations[mapping.parent]!.column)}
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
            case "condition": {
                const binding = bindAttributes(mapping, source, compilation.access.context);

                return sql`coalesce(${Condition.render(expression.condition, binding)}, false)`;
            }
            case "relation":
                return this.#relation(expression.name, mapping, source, compilation);
            case "through":
                return this.#through(expression, mapping, source, compilation);
            case "grants":
                return this.#grants(expression.reference, mapping, source, compilation);
        }
    }

    /** Match rows with a referenced object the caller holds the grant permission on. */
    #grants(
        reference: string,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // compile each granting type's grant permission against its own alias of the referenced row
        const columns = mapping.references![reference]!;
        const branches = this.#authorizer
            .mappings()
            .filter((target) => target.policy.definition.grantedBy !== undefined)
            .map((target) => {
                // alias the referenced row and read the type's grant permission
                const row = alias(target.table, `access_referenced_${compilation.aliases.next++}`);
                const definition = target.policy.definition;
                const grant = target.policy.permission(definition.grantedBy!);

                return sql`(
                    ${column(source, columns.packageId)} = ${definition.packageId}
                    AND ${column(source, columns.type)} = ${definition.name}
                    AND EXISTS (
                        SELECT 1 FROM ${from(row)}
                        WHERE ${column(row, target.id)} = ${column(source, columns.id)}
                            AND ${TableMapping.scopeColumn(row, target)} = ${column(source, columns.scope)}
                            AND ${this.#permission(grant, target, row, compilation)}
                    )
                )`;
            });

        return branches.length === 0 ? sql`false` : sql`(${sql.join(branches, sql` OR `)})`;
    }

    /** Match the subject a field holds, or current relationships, for one relation. */
    #relation(name: string, mapping: TableMapping, source: Table, compilation: Compilation): SQL {
        // read the relation and whether a field holds it
        const relation = this.#authorizer.relation(mapping.policy, name);
        const scope = TableMapping.scopeColumn(source, mapping);
        const field = mapping.relations[name];
        const authority = compilation.authority;

        // lend nothing a field holds to a delegate
        if (field && authority.delegator !== undefined) {
            return sql`false`;
        }
        // match a subject key the column holds against the authority's subjects the relation accepts
        else if (field?.isKey) {
            const keys = authority.subjects
                .filter((subject) => accepts(relation, subject))
                .map((subject) => subjectKey(subject));

            return keys.length === 0
                ? sql`false`
                : sql`coalesce(${inArray(column(source, field.column), keys)}, false)`;
        }
        // match a subject by the type, scope and relation in the row's columns
        else if (field?.subject !== undefined) {
            const subject = {
                packageId: column(source, field.subject.packageId),
                type: column(source, field.subject.type),
                scope: column(source, field.subject.scope),
                id: column(source, field.column),
            };
            if (field.subject.relation === undefined) {
                return sql`coalesce(${authority.match(subject)}, false)`;
            }
            const relationColumn = column(source, field.subject.relation);

            return sql`coalesce((
                (${relationColumn} IS NULL AND ${authority.match(subject)})
                OR (${relationColumn} IS NOT NULL AND ${authority.match({ ...subject, relation: relationColumn })})
            ), false)`;
        }
        // match the one subject type a field holds
        else if (field) {
            const [type] = relation.subjects;
            const subject = {
                packageId: sql`${type!.packageId}`,
                type: sql`${type!.type}`,
                scope: field.scope === undefined ? scope : sql`${field.scope}`,
                id: column(source, field.column),
                ...(type!.relation === undefined ? {} : { relation: sql`${type!.relation}` }),
            };

            return sql`coalesce(${authority.match(subject)}, false)`;
        }

        // match current relationships of subject types the relation accepts
        const relationship = alias(
            accessRelationship,
            `access_relationship_${compilation.aliases.next++}`,
        );
        const definition = mapping.policy.definition;

        return sql`EXISTS (
            SELECT 1 FROM ${from(relationship)}
            WHERE ${relationship.objectScope} = ${scope}
                AND ${relationship.packageId} = ${definition.packageId}
                AND ${relationship.type} = ${definition.name}
                AND ${relationship.objectId} = ${column(source, mapping.id)}
                AND ${relationship.relation} = ${name}
                AND ${GrantCondition.where(relationship, GrantCondition.values(compilation.access.context, authority.delegator))}
                AND ${accepted(relationship, relation)}
                AND ${authority.member(relationship)}
        )`;
    }

    /** Correlate a related object's permission through a field, an ancestor index, or relationships. */
    #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        source: Table,
        compilation: Compilation,
    ): SQL {
        // read the relation and whether a field holds it
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);
        const scope = TableMapping.scopeColumn(source, mapping);
        const field = mapping.relations[expression.relation];

        // compile the target permission against a distinct alias of each related type
        const related = relation.subjects.map((subject) => {
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

        // follow relationships to plain objects in the same scope without delegation
        if (!field) {
            const relationship = alias(
                accessRelationship,
                `access_relationship_${compilation.aliases.next++}`,
            );
            const definition = mapping.policy.definition;
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

        // correlate the parent of whichever type the row's columns name, for a field holding several types
        if (field.subject !== undefined && !expression.transitive) {
            const typed = field.subject;
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

        // correlate the direct parent a field holds within the source object's scope
        const [{ target, parent, predicate }] = related as [(typeof related)[number]];
        const targetId = column(parent, target.id);
        const targetScope = TableMapping.scopeColumn(parent, target);
        if (!expression.transitive) {
            return sql`EXISTS (
                SELECT 1 FROM ${from(parent)}
                WHERE ${targetScope} = ${scope}
                    AND ${targetId} = ${column(source, field.column)}
                    AND ${predicate}
            )`;
        }

        // join the indexed ancestors to their protected application rows
        const tree = mapping.trees![expression.relation]!;
        const ancestor = alias(tree.ancestors, `access_ancestor_${compilation.aliases.next++}`);

        return sql`EXISTS (
            SELECT 1 FROM ${from(ancestor)} JOIN ${from(parent)}
                ON ${targetId} = ${ancestor.ancestor} AND ${targetScope} = ${ancestor.scope}
            WHERE ${ancestor.scope} = ${scope}
                AND ${ancestor.descendant} = ${column(source, mapping.id)}
                AND ${ancestor.depth} > 0
                AND ${predicate}
        )`;
    }
}

/** Match a current binding of one of the roles to the compiled authority on a covered object. */
function binding(
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

/** Bind a condition's columns to a mapping's attributes, and its parameters to trusted request attributes. */
function bindAttributes(
    mapping: TableMapping,
    source: Table,
    context: AccessContext,
): Binding<SQLWrapper> {
    return {
        exists: (via) => {
            throw new AccessError(
                "INVALID_DECLARATION",
                `policy conditions follow no relations: ${via}`,
            );
        },
        column: (name) => column(source, mapping.attributes[name]!),
        parameter: (name) => requireAttribute(context, name),
    };
}
