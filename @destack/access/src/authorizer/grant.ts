import { PackageId } from "@destack/package";
import { jsonElements, sql, Statement, type DatabaseConnection } from "@destack/db";
import { schema } from "@destack/schema";
import { objectKey, type ObjectReference, type PermissionReference } from "../policy/policy.ts";
import type { AccessExpression } from "../policy/expression.ts";
import { accepts, keySubject, type Subject } from "../policy/subject.ts";
import { accessRelationship, type RelationshipRow } from "../relationship/table.ts";
import type { Authorizer } from "./authorizer.ts";
import { GrantCondition } from "./condition.ts";
import { columnsOf, TableMapping } from "./mapping.ts";

/** The most identifiers one bulk read names: well within the parameter budget, and a page of rows. */
const PREFETCH_CHUNK = 500;

/** One way a row admits callers: its subject, the conditions it holds under and how it grants. */
export interface Grant {
    /** The subject: a principal, a wildcard of a type, or a subject set by its relation. */
    readonly subject: Subject;
    /** The relationship's conditions, absent for a subject a field of the row holds, which lends nothing to delegates. */
    readonly condition?: GrantCondition;
    /** The bound role and the permission it must grant, for role bindings. */
    readonly role?: { readonly id: string; readonly permission: PermissionReference };
    /** The conditions of the arrows followed to reach the grant, each of which must hold without delegation. */
    readonly arrows: readonly GrantCondition[];
    /** The object the relationship, field or role binding sits on. */
    readonly object: ObjectReference;
    /** How the permission reaches the grant: permissions, arrows and the final relation or role, in order. */
    readonly path: readonly string[];
}

/** How a row reaches a decision: listed within the scope that contains it, or read as one object by its key in its own scope. */
export type Lookup = "listing" | "object";

/** The arrows followed and the path taken while collecting grants. */
interface Trail {
    /** The conditions of the arrows followed. */
    readonly arrows: readonly GrantCondition[];
    /** The steps taken. */
    readonly path: readonly string[];
}

/**
 * Collect, once for every caller, the grants a permission reaches on rows of one mapped type.
 *
 * A union of relations, permissions and arrows admits exactly its grants in memory; the grants of any other permission bound when the decision next changes by time.
 */
export class GrantReader {
    /** The authorizer whose policies define the grants. */
    readonly #authorizer: Authorizer;
    /** The database the grants are read from. */
    readonly #database: DatabaseConnection;
    /** The objects of the rows' scope and every scope enclosing it, nearest first. */
    readonly #scopes: readonly ObjectReference[];
    /** The relationships of each object read so far, by object key. */
    readonly #read = new Map<string, Promise<RelationshipRow[]>>();
    /** The proper ancestors of each node read so far, by tree, scope and node. */
    readonly #ancestry = new Map<string, Promise<string[]>>();

    /** Read grants from a database for rows within a scope chain. */
    constructor(
        authorizer: Authorizer,
        database: DatabaseConnection,
        scopes: readonly ObjectReference[],
    ) {
        this.#authorizer = authorizer;
        this.#database = database;
        this.#scopes = scopes;
    }

    /**
     * Collect the grants a permission reaches on each row, rows of its own type by default.
     *
     * On rows of another type, only the roles bound on or above them grant the permission.
     */
    async read(
        permission: PermissionReference,
        rows: readonly Readonly<Record<string, unknown>>[],
        mapping = this.#authorizer.mapping(permission),
    ): Promise<Grant[][]> {
        // read the rows' relationships and ancestry in bulk per scope
        for (const [scope, within] of Map.groupBy(rows, (row) =>
            TableMapping.scope(mapping, row),
        )) {
            await this.#prefetch(mapping, scope, within);
        }

        // collect each row's grants, through the permission's expression on rows of its own type
        const isOwn = mapping.policy === this.#authorizer.policy(permission);
        const trail = { arrows: [], path: [] };

        return Promise.all(
            rows.map((row) =>
                isOwn
                    ? this.#permission(permission, mapping, row, trail)
                    : this.#bound(permission, mapping, row, trail),
            ),
        );
    }

    /** Read the relationships of a scope's rows and of their tree ancestors with one query per chunk. */
    async #prefetch(
        mapping: TableMapping,
        scope: string,
        rows: readonly Readonly<Record<string, unknown>>[],
    ): Promise<void> {
        // read the ancestry of the rows in every tree of their type
        const ids = rows.map((row) => String(row[mapping.id]));
        const ancestors = new Set<string>();
        for (const [relation, tree] of Object.entries(mapping.trees ?? {})) {
            const found = new Map<string, string[]>(ids.map((id) => [id, []]));
            for (const chunk of chunks(ids)) {
                const entries = await tree.ancestry.all(this.#database, {
                    scope,
                    ids: JSON.stringify(chunk.map((id) => [id])),
                });
                for (const entry of entries) {
                    found.get(String(entry.descendant))!.push(String(entry.ancestor));
                    ancestors.add(String(entry.ancestor));
                }
            }
            for (const [id, above] of found) {
                this.#ancestry.set(ancestryKey(relation, scope, id), Promise.resolve(above));
            }
        }

        // read in one query the relationships of the rows, their ancestors, and the scope chain
        const definition = mapping.policy.definition;
        const typed = (id: string, within: string): ObjectReference => ({
            packageId: definition.packageId,
            type: definition.name,
            scope: within,
            id,
        });
        const objects = [
            ...[...new Set([...ids, ...ancestors])].map((id) => typed(id, scope)),
            ...this.#scopes,
        ].filter((object) => !this.#read.has(objectKey(object)));
        const found = new Map<string, RelationshipRow[]>(
            objects.map((object) => [objectKey(object), []]),
        );
        for (const chunk of chunks(objects)) {
            const entries = await RELATIONSHIPS.all(this.#database, {
                objects: JSON.stringify(
                    chunk.map((object) => [object.scope, object.packageId, object.type, object.id]),
                ),
            });
            for (const entry of entries) {
                found.get(objectKey(relatedObject(entry)))!.push(entry);
            }
        }
        for (const [key, entries] of found) {
            this.#read.set(key, Promise.resolve(entries));
        }
    }

    /** Collect a permission's grants through its expression and the roles bound on the row, above it, or on its scopes. */
    async #permission(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        const expression = this.#authorizer.expression(permission);
        const step = { ...trail, path: [...trail.path, `${permission.type} ${permission.name}`] };

        return [
            ...(await this.#expression(expression, mapping, row, step)),
            ...(await this.#bound(permission, mapping, row, step)),
        ];
    }

    /** Collect the grants one expression reaches: every branch of a combination, none for an attribute condition. */
    async #expression(
        expression: AccessExpression,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        switch (expression.kind) {
            case "none":
            case "condition":
                return [];
            case "union":
            case "intersection": {
                const branches = await Promise.all(
                    expression.expressions.map((child) =>
                        this.#expression(child, mapping, row, trail),
                    ),
                );

                return branches.flat();
            }
            case "exclusion":
                return [
                    ...(await this.#expression(expression.include, mapping, row, trail)),
                    ...(await this.#expression(expression.exclude, mapping, row, trail)),
                ];
            case "permission":
                return this.#permission(
                    mapping.policy.permission(expression.name),
                    mapping,
                    row,
                    trail,
                );
            case "relation":
                return this.#relation(expression.name, mapping, row, trail);
            case "through":
                return this.#through(expression, mapping, row, trail);
            case "grants":
                return this.#grants(expression.reference, mapping, row, trail);
        }
    }

    /** Collect the grants of the grant permission of the object a row references, where this database holds it. */
    async #grants(
        reference: string,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        // read the referenced object where its type grants and lives in this database
        const columns = mapping.references![reference]!;
        const referenced = {
            packageId: PackageId.parse(row[columns.packageId]),
            type: schema.string().parse(row[columns.type]),
        };
        const target = this.#authorizer.mappingOf(referenced);
        const grantedBy = target?.policy.definition.grantedBy;
        if (target === undefined || grantedBy === undefined) {
            return [];
        }
        const [related] = await this.#rows(target, schema.string().parse(row[columns.scope]), [
            schema.string().parse(row[columns.id]),
        ]);

        return related === undefined
            ? []
            : this.#permission(target.policy.permission(grantedBy), target, related, {
                  ...trail,
                  path: [...trail.path, `grants on ${referenced.type} ${String(row[columns.id])}`],
              });
    }

    /** Collect the subject a field holds, or the subjects of the relation's relationships. */
    async #relation(
        name: string,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        // grant to the subject a field holds
        const relation = this.#authorizer.relation(mapping.policy, name);
        const field = mapping.relations[name];
        if (field) {
            const held = row[field.column];
            if (held === null || held === undefined) {
                return [];
            }
            const [type] = relation.subjects;
            const columns = field.subject;
            const heldRelation = columns === undefined ? undefined : row[columns.relation];
            const subject: Subject = field.isKey
                ? keySubject(schema.string().parse(held))
                : columns === undefined
                  ? {
                        packageId: type!.packageId,
                        type: type!.type,
                        scope: field.scope ?? TableMapping.scope(mapping, row),
                        id: schema.string().parse(held),
                        ...(type!.relation === undefined ? {} : { relation: type!.relation }),
                    }
                  : {
                        packageId: PackageId.parse(row[columns.packageId]),
                        type: schema.string().parse(row[columns.type]),
                        scope: schema.string().parse(row[columns.scope]),
                        id: schema.string().parse(held),
                        ...(heldRelation === null || heldRelation === undefined
                            ? {}
                            : { relation: schema.string().parse(heldRelation) }),
                    };

            return [
                {
                    subject,
                    object: objectOf(mapping, row),
                    path: [...trail.path, `field ${field.column} holding relation ${name}`],
                    arrows: trail.arrows,
                },
            ];
        }

        // grant to the accepted subjects of the relation's relationships
        const relationships = await this.#relationshipsOf(mapping, row);

        return relationships
            .filter((entry) => entry.relation === name && accepts(relation, subjectOf(entry)))
            .map((entry) => ({
                subject: subjectOf(entry),
                condition: new GrantCondition(entry),
                object: objectOf(mapping, row),
                path: [...trail.path, `relation ${name}`],
                arrows: trail.arrows,
            }));
    }

    /** Collect what a related object's permission grants, through a field, the ancestors, or relationships. */
    async #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        // read the relation and the scope its related objects share with the row
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);
        const field = mapping.relations[expression.relation];
        const scope = TableMapping.scope(mapping, row);

        // follow relationships to plain objects in the same scope, each arrow adding its conditions
        if (!field) {
            const relationships = (await this.#relationshipsOf(mapping, row)).filter(
                (entry) =>
                    entry.relation === expression.relation &&
                    entry.subjectScope === scope &&
                    entry.subjectRelation === null,
            );
            const grants: Grant[] = [];
            for (const entry of relationships) {
                const subject = relation.subjects.find(
                    (type) =>
                        type.packageId === entry.subjectPackageId &&
                        type.type === entry.subjectType,
                );
                if (subject === undefined) {
                    continue;
                }
                const target = this.#authorizer.mapping(subject);
                for (const related of await this.#rows(target, scope, [entry.subjectId])) {
                    grants.push(
                        ...(await this.#permission(
                            target.policy.permission(expression.permission),
                            target,
                            related,
                            {
                                arrows: [...trail.arrows, new GrantCondition(entry)],
                                path: [
                                    ...trail.path,
                                    `through relation ${expression.relation} to ${target.policy.definition.name} ${entry.subjectId}`,
                                ],
                            },
                        )),
                    );
                }
            }

            return grants;
        }

        // follow the parent a field holds, or every ancestor of a tree
        const [subject] = relation.subjects;
        const target = this.#authorizer.mapping(subject!);
        const ids = expression.transitive
            ? await this.#ancestors(mapping, expression.relation, row)
            : row[field.column] === null || row[field.column] === undefined
              ? []
              : [String(row[field.column])];
        const grants: Grant[] = [];
        for (const related of await this.#rows(target, scope, ids)) {
            grants.push(
                ...(await this.#permission(
                    target.policy.permission(expression.permission),
                    target,
                    related,
                    {
                        ...trail,
                        path: [
                            ...trail.path,
                            `through ${expression.relation} to ${target.policy.definition.name} ${schema.string().parse(related[target.id])}`,
                        ],
                    },
                )),
            );
        }

        return grants;
    }

    /** Collect the subjects of role bindings on the row, its ancestors or owners, or the scope chain. */
    async #bound(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<Grant[]> {
        const bindings = [
            ...(await this.#covering(mapping, row)),
            ...(await this.#scopeBindings()),
        ];

        return bindings.map((entry) => ({
            subject: subjectOf(entry),
            condition: new GrantCondition(entry),
            role: { id: entry.roleId!, permission },
            object: relatedObject(entry),
            path: [...trail.path, `role ${entry.roleId!} bound on ${entry.type} ${entry.objectId}`],
            arrows: trail.arrows,
        }));
    }

    /** Read the role bindings on the row itself, its tree ancestors, or its owning parent's chain. */
    async #covering(
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
    ): Promise<RelationshipRow[]> {
        // read the bindings on the row
        const definition = mapping.policy.definition;
        const own = (await this.#relationshipsOf(mapping, row)).filter(
            (entry) => entry.roleId !== null,
        );
        const tree = mapping.parent === undefined ? undefined : mapping.trees?.[mapping.parent];

        // read the bindings on tree ancestors
        if (tree !== undefined) {
            const ancestors = await this.#ancestors(mapping, mapping.parent!, row);
            const scope = TableMapping.scope(mapping, row);
            const above = await Promise.all(
                ancestors.map((ancestor) =>
                    this.#relationships({
                        scope,
                        packageId: definition.packageId,
                        type: definition.name,
                        id: ancestor,
                    }),
                ),
            );

            return [...own, ...above.flat().filter((entry) => entry.roleId !== null)];
        }
        // read the bindings covering the owning parent
        else if (mapping.parent !== undefined) {
            const [subject] = this.#authorizer.relation(mapping.policy, mapping.parent).subjects;
            const target = this.#authorizer.mapping(subject!);
            const parentId = row[mapping.relations[mapping.parent]!.column];
            const parents =
                parentId === null || parentId === undefined
                    ? []
                    : await this.#rows(target, TableMapping.scope(mapping, row), [
                          schema.string().parse(parentId),
                      ]);
            const covered = await Promise.all(
                parents.map((parent) => this.#covering(target, parent)),
            );

            return [...own, ...covered.flat()];
        }
        // read the bindings on the row alone
        else {
            return own;
        }
    }

    /** Read the role bindings on the objects of the scope chain. */
    async #scopeBindings(): Promise<RelationshipRow[]> {
        const bindings = await Promise.all(
            this.#scopes.map((object) => this.#relationships(object)),
        );

        return bindings.flat().filter((entry) => entry.roleId !== null);
    }

    /** Read the relationships on the object a mapped row is. */
    #relationshipsOf(
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
    ): Promise<RelationshipRow[]> {
        return this.#relationships(objectOf(mapping, row));
    }

    /** Read every relationship on one object once. */
    #relationships(object: ObjectReference): Promise<RelationshipRow[]> {
        // reuse the relationships read before
        const key = objectKey(object);
        const known = this.#read.get(key);
        if (known) {
            return known;
        }

        // read them all, leaving their conditions to each caller
        const read = RELATIONSHIPS.all(this.#database, {
            objects: JSON.stringify([[object.scope, object.packageId, object.type, object.id]]),
        });
        this.#read.set(key, read);

        return read;
    }

    /** Read rows of a mapped type in a scope by identifier, in the order of the identifiers. */
    async #rows(
        mapping: TableMapping,
        scope: string,
        ids: readonly string[],
    ): Promise<Record<string, unknown>[]> {
        // read nothing for no identifiers
        if (ids.length === 0) {
            return [];
        }

        // read the rows in the scope, keeping the order of the identifiers, nearest ancestors first
        const rows = await this.#authorizer.rows(mapping).all(this.#database, {
            scope,
            ids: JSON.stringify(ids.map((id) => [id])),
        });
        const byId = new Map(rows.map((row) => [String(row[mapping.id]), row]));

        return ids.flatMap((id): Record<string, unknown>[] => {
            const row = byId.get(id);

            return row === undefined ? [] : [row];
        });
    }

    /** Read the identifiers of a row's proper ancestors through a tree. */
    async #ancestors(
        mapping: TableMapping,
        relation: string,
        row: Readonly<Record<string, unknown>>,
    ): Promise<string[]> {
        // reuse the ancestry read before
        const tree = mapping.trees![relation]!;
        const key = ancestryKey(
            relation,
            TableMapping.scope(mapping, row),
            String(row[mapping.id]),
        );
        const known = this.#ancestry.get(key);
        if (known) {
            return known;
        }

        // read the node's proper ancestors
        const read = (async () =>
            (
                await tree.ancestry.all(this.#database, {
                    scope: TableMapping.scope(mapping, row),
                    ids: JSON.stringify([[String(row[mapping.id])]]),
                })
            ).map((entry) => String(entry.ancestor)))();
        this.#ancestry.set(key, read);

        return read;
    }
}

/** Reference the object a mapped row is. */
function objectOf(mapping: TableMapping, row: Readonly<Record<string, unknown>>): ObjectReference {
    const definition = mapping.policy.definition;

    return {
        packageId: definition.packageId,
        type: definition.name,
        scope: TableMapping.scope(mapping, row),
        id: schema.string().parse(row[mapping.id]),
    } as ObjectReference;
}

/** Reference the object a relationship relates. */
function relatedObject(entry: RelationshipRow): ObjectReference {
    return {
        packageId: entry.packageId,
        type: entry.type,
        scope: entry.objectScope,
        id: entry.objectId,
    } as ObjectReference;
}

/** Read a relationship's subject. */
function subjectOf(entry: RelationshipRow): Subject {
    return {
        packageId: entry.subjectPackageId,
        type: entry.subjectType,
        scope: entry.subjectScope,
        id: entry.subjectId,
        ...(entry.subjectRelation === null ? {} : { relation: entry.subjectRelation }),
    } as Subject;
}

/** The relationships on a JSON list of objects, each as scope, package, type and identifier. */
const RELATIONSHIPS = new Statement<RelationshipRow>(
    (
        value,
    ) => sql`SELECT ${columnsOf(accessRelationship)} FROM ${jsonElements(value("objects"), "listed_object")}
        JOIN ${accessRelationship}
            ON ${accessRelationship.objectScope} = listed_object.value ->> 0
            AND ${accessRelationship.packageId} = listed_object.value ->> 1
            AND ${accessRelationship.type} = listed_object.value ->> 2
            AND ${accessRelationship.objectId} = listed_object.value ->> 3`,
);

/** Name a node's ancestry within one tree and scope. */
function ancestryKey(relation: string, scope: string, id: string): string {
    return JSON.stringify([relation, scope, id]);
}

/** Split values into chunks within the parameter budget. */
function chunks<Value>(values: readonly Value[]): Value[][] {
    const result: Value[][] = [];
    for (let start = 0; start < values.length; start += PREFETCH_CHUNK) {
        result.push(values.slice(start, start + PREFETCH_CHUNK));
    }

    return result;
}
