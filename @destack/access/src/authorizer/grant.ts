import type { Snapshot } from "@destack/db/log";
import { Condition, type Match } from "@destack/db/query";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { requireAttribute } from "../context/context.ts";
import { AccessError } from "../error/index.ts";
import type { AccessExpression } from "../policy/expression.ts";
import { objectKey, type ObjectReference, type PermissionReference } from "../policy/policy.ts";
import { accepts, keySubject, type Subject } from "../policy/subject.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { RelationshipRow } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { Authority } from "./authority.ts";
import type { Authorizer } from "./authorizer.ts";
import { GrantCondition } from "./condition.ts";
import { TableMapping } from "./mapping.ts";

/** The most identifiers one bulk read names: well within the parameter budget, and a page of rows. */
const PREFETCH_CHUNK = 500;

/** One way a row admits callers: its subject, the conditions it holds under and how it grants. */
export interface Grant {
    /** The subject: a principal, a wildcard of a type, or a subject set by its relation. */
    readonly subject: Subject;
    /** The relationship's conditions, absent for a subject a field of the row holds, which lends nothing to delegates. */
    readonly condition?: GrantCondition;
    /** The bound role and the permission it must grant, for role bindings; absent, it must grant everything, as owners' roles do. */
    readonly role?: { readonly id: string; readonly permission?: PermissionReference };
    /** The conditions of the arrows followed to reach the grant, each of which must hold without delegation. */
    readonly arrows: readonly GrantCondition[];
    /** The object the relationship, field or role binding sits on. */
    readonly object: ObjectReference;
    /** How the permission reaches the grant: permissions, arrows and the final relation or role, in order. */
    readonly path: readonly string[];
}

/** The grants an expression reaches on a row, combined as the expression combines its branches. */
export type GrantTree =
    | {
          /** Admit a caller any of the grants admits: a relation, a field or a role binding. */
          readonly kind: "any";
          /** The grants. */
          readonly grants: readonly Grant[];
      }
    | {
          /** Admit a caller some branch admits: a union, or the objects a relation leads to. */
          readonly kind: "some";
          /** The branches. */
          readonly trees: readonly GrantTree[];
      }
    | {
          /** Admit a caller every branch admits: an intersection. */
          readonly kind: "all";
          /** The branches. */
          readonly trees: readonly GrantTree[];
      }
    | {
          /** Admit a caller one branch admits and the other does not: an exclusion. */
          readonly kind: "except";
          /** The admitting branch. */
          readonly include: GrantTree;
          /** The refusing branch. */
          readonly exclude: GrantTree;
      }
    | {
          /** Admit every caller whose request meets a condition over the row's attributes. */
          readonly kind: "condition";
          /** The condition over the row's columns. */
          readonly match: Match;
          /** The row the condition reads. */
          readonly row: Readonly<Record<string, unknown>>;
      };

/** How a row reaches a decision: listed within the scope that contains it, or read as one object by its key in its own scope. */
export type Lookup = "listing" | "object";

/** The arrows followed and the path taken while collecting grants. */
interface Trail {
    /** The conditions of the arrows followed. */
    readonly arrows: readonly GrantCondition[];
    /** The steps taken. */
    readonly path: readonly string[];
}

/** Collect, once for every caller, the grant trees a permission reaches on rows of one mapped type. */
export class GrantReader {
    /** The authorizer whose policies define the grants. */
    readonly #authorizer: Authorizer;
    /** The database as the grants are read from it: live, or at a position. */
    readonly #snapshot: Snapshot;
    /** The objects of the rows' scope and every scope enclosing it, nearest first. */
    readonly #scopes: readonly ObjectReference[];
    /** The relationships of each object read so far, by object key. */
    readonly #read = new Map<string, Promise<RelationshipRow[]>>();
    /** The proper ancestors of each node read so far, by tree, scope and node. */
    readonly #ancestry = new Map<string, Promise<string[]>>();

    /** Read grants from a snapshot of a database for rows within a scope chain. */
    constructor(authorizer: Authorizer, snapshot: Snapshot, scopes: readonly ObjectReference[]) {
        this.#authorizer = authorizer;
        this.#snapshot = snapshot;
        this.#scopes = scopes;
    }

    /** Collect the tree of the role bindings granting everything on or above a row, which its owners hold. */
    async ownership(
        row: Readonly<Record<string, unknown>>,
        mapping: TableMapping,
    ): Promise<GrantTree> {
        await this.#prefetch(mapping, TableMapping.scope(mapping, row), [row]);

        return any(await this.#bound(undefined, mapping, row, { arrows: [], path: ["ownership"] }));
    }

    /**
     * Collect the grant tree of a permission on each row, rows of its own type by default, which decides callers exactly.
     *
     * On rows of another type, only the roles bound on or above them grant the permission.
     */
    async trees(
        permission: PermissionReference,
        rows: readonly Readonly<Record<string, unknown>>[],
        mapping = this.#authorizer.mapping(permission),
    ): Promise<GrantTree[]> {
        // read the rows' relationships and ancestry in bulk per scope
        for (const [scope, within] of Map.groupBy(rows, (row) =>
            TableMapping.scope(mapping, row),
        )) {
            await this.#prefetch(mapping, scope, within);
        }

        // collect each row's tree, through the permission's expression on rows of its own type
        const isOwn = mapping.policy === this.#authorizer.policy(permission);
        const trail = { arrows: [], path: [] };

        return Promise.all(
            rows.map(async (row) =>
                isOwn
                    ? this.#permission(permission, mapping, row, trail)
                    : any(await this.#bound(permission, mapping, row, trail)),
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
                for (const entry of await tree.ancestry(this.#snapshot, scope, chunk)) {
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
            for (const entry of await Relationship.readByObject(this.#snapshot, chunk)) {
                found.get(objectKey(relatedObject(entry)))!.push(entry);
            }
        }
        for (const [key, entries] of found) {
            this.#read.set(key, Promise.resolve(entries));
        }
    }

    /** Collect a permission's tree: its expression's, or the roles bound on the row, above it, or on its scopes. */
    async #permission(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<GrantTree> {
        const expression = this.#authorizer.expression(permission);
        const step = {
            ...trail,
            path: [...trail.path, `${permission.type} ${permission.name}`],
        };

        return some([
            await this.#expression(expression, mapping, row, step),
            any(await this.#bound(permission, mapping, row, step)),
        ]);
    }

    /** Collect the tree one expression reaches, combining its branches as it does. */
    async #expression(
        expression: AccessExpression,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<GrantTree> {
        switch (expression.kind) {
            case "none":
                return some([]);
            case "condition":
                return {
                    kind: "condition",
                    match: Condition.compile(
                        Condition.rename(expression.condition, (name) => mapping.attributes[name]!),
                        mapping.table,
                    ),
                    row,
                };
            case "union":
            case "intersection": {
                const branches = await Promise.all(
                    expression.expressions.map((child) =>
                        this.#expression(child, mapping, row, trail),
                    ),
                );

                return expression.kind === "union"
                    ? some(branches)
                    : { kind: "all", trees: branches };
            }
            case "exclusion":
                return {
                    kind: "except",
                    include: await this.#expression(expression.include, mapping, row, trail),
                    exclude: await this.#expression(expression.exclude, mapping, row, trail),
                };
            case "permission":
                return this.#permission(
                    mapping.policy.permission(expression.name),
                    mapping,
                    row,
                    trail,
                );
            case "relation":
                return any(await this.#relation(expression.name, mapping, row, trail));
            case "through":
                return this.#through(expression, mapping, row, trail);
            case "grants":
                return this.#grants(expression.reference, mapping, row, trail);
        }
    }

    /** Collect the tree of the grant permission of the object a row references, where this database holds it. */
    async #grants(
        reference: string,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<GrantTree> {
        // read the referenced object where its type grants and lives in this database
        const columns = mapping.references![reference]!;
        const referenced = {
            packageId: PackageId.parse(row[columns.packageId]),
            type: schema.string().parse(row[columns.type]),
        };
        const target = this.#authorizer.mappingOf(referenced);
        const grantedBy = target?.policy.definition.grantedBy;
        if (target === undefined || grantedBy === undefined) {
            return some([]);
        }
        const [related] = await this.#rows(target, schema.string().parse(row[columns.scope]), [
            schema.string().parse(row[columns.id]),
        ]);

        return related === undefined
            ? some([])
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
            const heldRelation =
                columns?.relation === undefined ? undefined : row[columns.relation];
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

    /** Collect the trees of a related object's permission, through a field, the ancestors, or relationships. */
    async #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        row: Readonly<Record<string, unknown>>,
        trail: Trail,
    ): Promise<GrantTree> {
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
            const trees: GrantTree[] = [];
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
                    trees.push(
                        await this.#permission(
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
                        ),
                    );
                }
            }

            return some(trees);
        }

        // follow the parent a field holds, of whichever type the row's columns name when it holds several
        const typed = field.subject;
        const subject =
            typed === undefined
                ? relation.subjects[0]
                : relation.subjects.find(
                      (type) =>
                          type.packageId === row[typed.packageId] && type.type === row[typed.type],
                  );
        if (subject === undefined) {
            return some([]);
        }
        const target = this.#authorizer.mapping(subject);
        const ids = expression.transitive
            ? await this.#ancestors(mapping, expression.relation, row)
            : row[field.column] === null || row[field.column] === undefined
              ? []
              : [String(row[field.column])];
        const trees: GrantTree[] = [];
        for (const related of await this.#rows(target, scope, ids)) {
            trees.push(
                await this.#permission(
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
                ),
            );
        }

        return some(trees);
    }

    /** Collect the role bindings on the row, its ancestors or owners, or the scope chain, whose roles must grant a permission, or everything when absent. */
    async #bound(
        permission: PermissionReference | undefined,
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
            role: { id: entry.roleId!, ...(permission === undefined ? {} : { permission }) },
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
        const read = Relationship.readByObject(this.#snapshot, [object]);
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
        const rows = await TableMapping.read(this.#snapshot, mapping, scope, ids);
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
                await tree.ancestry(this.#snapshot, TableMapping.scope(mapping, row), [
                    String(row[mapping.id]),
                ])
            ).map((entry) => entry.ancestor))();
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

/** Combine trees some of which admits. */
function some(trees: readonly GrantTree[]): GrantTree {
    return { kind: "some", trees };
}

/** Admit through any of some grants. */
function any(grants: readonly Grant[]): GrantTree {
    return { kind: "any", grants };
}

/** Read and decide grant trees. */
export const GrantTree = {
    /** List every grant a tree reaches, a superset of those admitting any one caller, for indexing and expiry. */
    flatten(tree: GrantTree): Grant[] {
        switch (tree.kind) {
            case "any":
                return [...tree.grants];
            case "some":
            case "all":
                return tree.trees.flatMap(GrantTree.flatten);
            case "except":
                return [...GrantTree.flatten(tree.include), ...GrantTree.flatten(tree.exclude)];
            case "condition":
                return [];
        }
    },

    /** Decide whether a tree admits one authority of a caller, as the compiled predicate does in SQL. */
    holds(tree: GrantTree, authority: Authority, access: Access): boolean {
        switch (tree.kind) {
            case "any":
                return tree.grants.some((grant) => authority.failure(grant, access) === undefined);
            case "some":
                return tree.trees.some((branch) => GrantTree.holds(branch, authority, access));
            case "all":
                return tree.trees.every((branch) => GrantTree.holds(branch, authority, access));
            case "except":
                return (
                    GrantTree.holds(tree.include, authority, access) &&
                    !GrantTree.holds(tree.exclude, authority, access)
                );
            case "condition":
                return (
                    tree.match({
                        column: (name) => tree.row[name],
                        parameter: (name) => requireAttribute(access.context, name),
                        exists: (via) => {
                            throw new AccessError(
                                "INVALID_DECLARATION",
                                `policy conditions follow no relations: ${via}`,
                            );
                        },
                    }) === true
                );
        }
    },
};
