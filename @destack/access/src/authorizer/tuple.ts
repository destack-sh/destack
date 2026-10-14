import { AccessContext } from "../context/context.ts";
import { ObjectReference, type ObjectTypeReference, Scope, Subject } from "@destack/sync";
import { type Match, Predicate, type Row, type Snapshot } from "@destack/db";
import { PackageId } from "@destack/package";
import { found, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { CONTAINED, type AccessExpression } from "../declare/expression.ts";
import { type PermissionReference } from "../declare/policy.ts";
import { accepts, type RelationDescription } from "../declare/subject.ts";
import { Relationship } from "../relationship/relationship.ts";
import type { RelationshipRow, RoleBinding } from "../relationship/table.ts";
import type { Access } from "./access.ts";
import type { Authorizer } from "./authorizer.ts";
import { TupleCondition } from "./condition.ts";
import { TableMapping } from "./mapping.ts";

/** The most identifiers per bulk read: well within the parameter budget, and a page of rows. */
const PREFETCH_CHUNK = 500;

/** A relation tuple of a row: the subject it admits, its conditions, and the relation or role. */
export interface Tuple {
    /** The subject: a principal, a wildcard of a type, or a subject set by its relation. */
    readonly subject: Subject;
    /** The relationship's conditions, absent for a subject in a row field. */
    readonly condition?: TupleCondition;
    /** The bound role, its binding, and its required permission, null if it must grant all. */
    readonly role?: {
        readonly id: string;
        readonly binding: string;
        readonly permission: PermissionReference | null;
    };
    /** The conditions of the arrows to the tuple, which must pass. */
    readonly arrows: readonly TupleCondition[];
    /** The object the relationship, field or role binding sits on. */
    readonly object: ObjectReference;
    /** How the permission leads to the tuple: permissions, arrows, then the relation or role. */
    readonly path: readonly string[];
}

/** The tuples of an expression on a row, combined like the expression's branches. */
export type TupleTree =
    | {
          /** Admit a caller any of the tuples admits: a relation, a field or a role binding. */
          readonly kind: "any";
          /** The tuples. */
          readonly tuples: readonly Tuple[];
      }
    | {
          /** Admit a caller some branch admits: a union, or the objects a relation leads to. */
          readonly kind: "some";
          /** The branches. */
          readonly trees: readonly TupleTree[];
      }
    | {
          /** Admit a caller every branch admits: an intersection. */
          readonly kind: "all";
          /** The branches. */
          readonly trees: readonly TupleTree[];
      }
    | {
          /** Admit a caller one branch admits and the other does not: an exclusion. */
          readonly kind: "except";
          /** The admitting branch. */
          readonly include: TupleTree;
          /** The refusing branch. */
          readonly exclude: TupleTree;
      }
    | {
          /** Admit every caller whose request meets a condition over the row's attributes. */
          readonly kind: "resource";
          /** The condition over the row's columns. */
          readonly match: Match;
          /** The row the condition reads. */
          readonly row: Row;
      }
    | {
          /** Admit every caller whose request attributes meet a condition. */
          readonly kind: "context";
          /** The condition over the request's attributes. */
          readonly match: Match;
      };

/** How a decision reads a row: listed in its scope, or as one object by its key. */
export type Lookup = "listing" | "object";

/** The arrows followed and the path taken while collecting tuples. */
interface Trail {
    /** The conditions of the arrows followed. */
    readonly arrows: readonly TupleCondition[];
    /** The steps taken. */
    readonly path: readonly string[];
}

/** Collect the tuple trees of a permission on rows of one mapped type, once for all callers. */
export class TupleReader {
    /** The authorizer whose policies define the tuples. */
    readonly #authorizer: Authorizer;
    /** The database as the tuples are read from it: live, or at a position. */
    readonly #snapshot: Snapshot;
    /** The objects of the rows' scope and every scope enclosing it, nearest first. */
    readonly #scopes: readonly ObjectReference[];
    /** The chains of the enclosed scopes the reader also decides rows of, by scope. */
    readonly #chains: Map<string, readonly ObjectReference[]>;
    /** The relationships of each object read so far, by object key. */
    readonly #read: Map<string, Promise<RelationshipRow[]>>;
    /** The role bindings of each owner scope's chain read so far, by scope. */
    readonly #chainBound: Map<string, Promise<RoleBinding[]>>;
    /** The proper ancestors of each node read so far, by tree, scope and node. */
    readonly #ancestry: Map<string, Promise<string[]>>;
    /** The rows read so far, absent where none exists, by type, scope and identifier. */
    readonly #loaded: Map<string, Promise<Row | undefined>>;

    /** Read tuples from a snapshot of a database for rows within a scope chain. */
    constructor(authorizer: Authorizer, snapshot: Snapshot, scopes: readonly ObjectReference[]) {
        // start with empty state
        this.#chains = new Map<string, readonly ObjectReference[]>();
        this.#read = new Map<string, Promise<RelationshipRow[]>>();
        this.#chainBound = new Map<string, Promise<RoleBinding[]>>();
        this.#ancestry = new Map<string, Promise<string[]>>();
        this.#loaded = new Map<string, Promise<Row | undefined>>();

        // keep the authorizer, the snapshot and the scope chain
        this.#authorizer = authorizer;
        this.#snapshot = snapshot;
        this.#scopes = scopes;
    }

    /** Decide an enclosed scope's rows by that scope's own chain, nearest first. */
    cover(scopes: readonly ObjectReference[]): void {
        const [nearest] = scopes;
        if (nearest !== undefined) {
            this.#chains.set(nearest.id, scopes);
        }
    }

    /** Note an object a call creates and the relationships its creation writes. */
    creating(object: ObjectReference, relationships: readonly RelationshipRow[]): void {
        this.#read.set(ObjectReference.key(object), Promise.resolve([...relationships]));
    }

    /** Read one object as a row of its type, absent where none exists. */
    async row(mapping: TableMapping, target: ObjectReference): Promise<Row | undefined> {
        const [row] = await this.#rows(mapping, target.scope, [target.id]);

        return row;
    }

    /** Collect the tree of the owner role bindings on or above a row. */
    async ownership(row: Row, mapping: TableMapping): Promise<TupleTree> {
        await this.#prefetch(mapping, [row]);

        return TupleTree.any(
            await this.#bound(null, mapping, row, { arrows: [], path: ["ownership"] }),
        );
    }

    /** Collect the tuple tree of a permission on one row, of the permission's type by default. */
    async tree(
        permission: PermissionReference,
        row: Row,
        mapping = this.#authorizer.requireMapping(permission),
    ): Promise<TupleTree> {
        const [tree] = await this.trees(permission, [row], mapping);
        if (tree === undefined) {
            throw new TypeError(`permission ${permission.name} has no tuple tree for the row`);
        }

        return tree;
    }

    /** Collect the tuple tree of a permission on each row. */
    async trees(
        permission: PermissionReference,
        rows: readonly Row[],
        mapping = this.#authorizer.requireMapping(permission),
    ): Promise<TupleTree[]> {
        // read the rows' relationships and ancestry in bulk
        await this.#prefetch(mapping, rows);

        // collect each row's tree, through the permission's expression on rows of its own type
        const isSameType = mapping.policy === this.#authorizer.policy(permission);
        const trail = { arrows: [], path: [] };

        return Promise.all(
            rows.map(async (row) =>
                isSameType
                    ? this.#permission(permission, mapping, row, trail)
                    : TupleTree.any(await this.#bound(permission, mapping, row, trail)),
            ),
        );
    }

    /** Read the relationships of rows, tree ancestors and scope chains, one query per chunk. */
    async #prefetch(mapping: TableMapping, rows: readonly Row[]): Promise<void> {
        // read the ancestry of each scope's rows in every tree of their type
        const definition = mapping.policy.definition;
        const typed = (id: string, within: string): ObjectReference => ({
            packageId: definition.packageId,
            type: definition.name,
            scope: within,
            id,
        });
        const reached: ObjectReference[] = [];
        for (const [scope, within] of Map.groupBy(rows, (row) =>
            TableMapping.scope(mapping, row),
        )) {
            const ids = within.map((row) => TableMapping.text(row, mapping.id));
            const ancestors = new Set<string>();
            for (const [relation, tree] of Object.entries(mapping.trees ?? {})) {
                const byDescendant = new Map<string, string[]>(ids.map((id) => [id, []]));
                for (const chunk of TupleReader.#chunks(ids)) {
                    for (const entry of await tree.ancestry(this.#snapshot, scope, chunk)) {
                        found(byDescendant, entry.descendant).push(entry.ancestor);
                        ancestors.add(entry.ancestor);
                    }
                }
                for (const [id, above] of byDescendant) {
                    this.#ancestry.set(
                        TupleReader.#ancestryKey(relation, scope, id),
                        Promise.resolve(above),
                    );
                }
            }
            reached.push(
                ...[...new Set([...ids, ...ancestors])].map((id) => typed(id, scope)),
                ...this.#chain(scope),
            );
        }

        // read in one query the relationships of the rows, their ancestors, and their scope chains
        const objects = [
            ...new Map(reached.map((object) => [ObjectReference.key(object), object])).values(),
        ].filter((object) => !this.#read.has(ObjectReference.key(object)));
        const byObject = new Map<string, RelationshipRow[]>(
            objects.map((object) => [ObjectReference.key(object), []]),
        );
        for (const chunk of TupleReader.#chunks(objects)) {
            for (const entry of await Relationship.readByObject(this.#snapshot, chunk)) {
                found(byObject, ObjectReference.key(TupleReader.#relatedObject(entry))).push(entry);
            }
        }
        for (const [key, entries] of byObject) {
            this.#read.set(key, Promise.resolve(entries));
        }
    }

    /** Collect a permission's tree: its expression's, or its bound roles. */
    async #permission(
        permission: PermissionReference,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<TupleTree> {
        const expression = this.#authorizer.expression(permission);
        const step = {
            ...trail,
            path: [...trail.path, `${permission.type} ${permission.name}`],
        };

        return TupleTree.some([
            await this.#expression(expression, mapping, row, step),
            TupleTree.any(await this.#bound(permission, mapping, row, step)),
        ]);
    }

    /** Collect the tree of one expression, combining its branches like the expression. */
    async #expression(
        expression: AccessExpression,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<TupleTree> {
        switch (expression.kind) {
            case "none":
                return TupleTree.some([]);
            case "resource": {
                // grant nothing by attributes when the database has only the object's scope row
                const predicate = mapping.policy.resolve(expression);
                if (!TableMapping.decides(mapping, predicate)) {
                    return TupleTree.some([]);
                }

                // read the attributes from their columns
                const columns = Predicate.rename(predicate, (name) =>
                    TableMapping.attribute(mapping, name),
                );

                return { kind: "resource", match: Predicate.compile(columns, mapping.table), row };
            }
            case "context":
                return {
                    kind: "context",
                    match: Predicate.compile(mapping.policy.resolve(expression)),
                };
            case "union":
            case "intersection": {
                const branches = await Promise.all(
                    expression.expressions.map((child) =>
                        this.#expression(child, mapping, row, trail),
                    ),
                );

                return expression.kind === "union"
                    ? TupleTree.some(branches)
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
                return TupleTree.any(await this.#relation(expression.name, mapping, row, trail));
            case "through":
                return this.#through(expression, mapping, row, trail);
            case "granters":
                return this.#granters(expression.reference, mapping, row, trail);
            case "readers":
                return this.#readers(expression, mapping, row, trail);
            case "contained":
                return this.#contained(expression, mapping, row, trail);
        }
    }

    /** Grant a row to principals inside its scope, or inside the row as a scope, of some types. */
    #contained(
        expression: Extract<AccessExpression, { kind: "contained" }>,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): TupleTree {
        // find the scope containing the principals: the row itself for a scope, else the scope it lives in
        const definition = mapping.policy.definition;
        const scope = TableMapping.scope(mapping, row);
        const [nearest] = this.#chain(scope);
        const object =
            definition.scope === true
                ? {
                      packageId: definition.packageId,
                      type: definition.name,
                      scope,
                      id: TableMapping.text(row, mapping.id),
                  }
                : nearest?.id === scope
                  ? nearest
                  : undefined;
        if (object === undefined) {
            return TupleTree.some([]);
        }

        // grant the scope's containment set, only to principals of the named types when given
        return TupleReader.#containment(object, expression.principals, {
            ...trail,
            path: [...trail.path, CONTAINED],
        });
    }

    /** Grant an object's containment set, only to principals of the named types when given. */
    static #containment(
        object: ObjectReference,
        principals: readonly ObjectTypeReference[] | undefined,
        trail: Trail,
    ): TupleTree {
        const grant = (subject: Subject): Tuple => ({
            subject,
            arrows: trail.arrows,
            object,
            path: trail.path,
        });
        const inside = TupleTree.any([grant({ ...object, relation: CONTAINED })]);

        return principals === undefined
            ? inside
            : {
                  kind: "all",
                  trees: [
                      inside,
                      TupleTree.any(
                          principals.map((type) => grant({ ...type, scope: "*", id: "*" })),
                      ),
                  ],
              };
    }

    /** Collect the tuple tree of a row's referenced object, when local. */
    async #granters(
        reference: string,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<TupleTree> {
        // read the referenced object where its type grants and lives in this database
        const columns = TableMapping.reference(mapping, reference);
        const referenced = {
            packageId: PackageId.parse(row[columns.packageId]),
            type: schema.string().parse(row[columns.type]),
        };
        const target = this.#authorizer.mapping(referenced);
        const grantedBy = target?.policy.definition.grantedBy;
        if (target === undefined || grantedBy === undefined) {
            return TupleTree.some([]);
        }
        const [related] = await this.#rows(target, schema.string().parse(row[columns.scope]), [
            schema.string().parse(row[columns.id]),
        ]);

        return related === undefined
            ? TupleTree.some([])
            : this.#permission(target.policy.permission(grantedBy), target, related, {
                  ...trail,
                  path: [
                      ...trail.path,
                      `grants on ${referenced.type} ${TableMapping.text(row, columns.id)}`,
                  ],
              });
    }

    /** Collect the tree of who sees the relationships of a row's referenced object. */
    async #readers(
        expression: Extract<AccessExpression, { kind: "readers" }>,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<TupleTree> {
        // read the referenced object when its type is local and differs from the row's type
        const columns = TableMapping.reference(mapping, expression.reference);
        const referenced = {
            packageId: PackageId.parse(row[columns.packageId]),
            type: schema.string().parse(row[columns.type]),
        };
        const mapped = this.#authorizer.mapping(referenced);
        const target = mapped?.policy === mapping.policy ? undefined : mapped;
        const [related] =
            target === undefined
                ? []
                : await this.#rows(target, schema.string().parse(row[columns.scope]), [
                      schema.string().parse(row[columns.id]),
                  ]);
        if (target === undefined || related === undefined) {
            return TupleTree.some([]);
        }

        // see a concealed relation through its grant permission, any other through the relationship read or grant permission
        const relation = row[expression.relation];
        const definition = target.policy.definition;
        const declared =
            typeof relation === "string" && Object.hasOwn(definition.relations, relation)
                ? definition.relations[relation]
                : undefined;
        const concealed = declared?.concealed === true ? declared : undefined;
        const names =
            concealed === undefined
                ? [target.policy.visibility, definition.grantedBy]
                : [concealed.grantedBy];
        const step = {
            ...trail,
            path: [
                ...trail.path,
                `shares of ${referenced.type} ${TableMapping.text(row, columns.id)}`,
            ],
        };

        return TupleTree.some(
            await Promise.all(
                names
                    .filter((name): name is string => name !== undefined)
                    .map((name) =>
                        this.#permission(target.policy.permission(name), target, related, step),
                    ),
            ),
        );
    }

    /** Collect the subject in a field, or the subjects of the relation's relationships. */
    async #relation(name: string, mapping: TableMapping, row: Row, trail: Trail): Promise<Tuple[]> {
        // grant to the subject in a field
        const relation = this.#authorizer.relation(mapping.policy, name);
        const field = mapping.relations[name];
        if (field) {
            const value = row[field.column];
            if (value === null || value === undefined) {
                return [];
            }
            const subject = this.#fieldSubject(name, mapping, field, row);

            return [
                {
                    subject,
                    object: TupleReader.#object(mapping, row),
                    path: [...trail.path, `field ${field.column} holding relation ${name}`],
                    arrows: trail.arrows,
                },
            ];
        }

        // grant to the accepted subjects of the relation's relationships
        const relationships = await this.#rowRelationships(mapping, row);

        return relationships
            .filter(
                (entry) =>
                    entry.relation === name && accepts(relation, TupleReader.#subject(entry)),
            )
            .map((entry) => ({
                subject: TupleReader.#subject(entry),
                condition: new TupleCondition(entry),
                object: TupleReader.#object(mapping, row),
                path: [...trail.path, `relation ${name}`],
                arrows: trail.arrows,
            }));
    }

    /** Read a field's subject: a subject key, an identifier of the one type, or typed columns. */
    #fieldSubject(
        name: string,
        mapping: TableMapping,
        field: TableMapping["relations"][string],
        row: Row,
    ): Subject {
        const columns = field.subject;

        // read a subject key
        if (field.isKey === true) {
            return Subject.read(TableMapping.text(row, field.column));
        }
        // type the identifier by the one type the relation accepts
        else if (columns === undefined) {
            const type = this.#authorizer.onlySubject(mapping.policy, name);

            return {
                packageId: type.packageId,
                type: type.type,
                scope: field.scope ?? TableMapping.scope(mapping, row),
                id: TableMapping.text(row, field.column),
                ...(type.relation === undefined ? {} : { relation: type.relation }),
            };
        }
        // type the identifier by the row's columns
        else {
            const relation = columns.relation;
            const isSet =
                relation !== undefined && row[relation] !== null && row[relation] !== undefined;

            return {
                packageId: PackageId.parse(row[columns.packageId]),
                type: TableMapping.text(row, columns.type),
                scope: TableMapping.text(row, columns.scope),
                id: TableMapping.text(row, field.column),
                ...(isSet ? { relation: TableMapping.text(row, relation) } : {}),
            };
        }
    }

    /** Collect the trees of a related object's permission by field, ancestors or relationships. */
    async #through(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<TupleTree> {
        // decide a permission of the enclosing scope by the relationships on the scope chain's objects
        const scope = TableMapping.scope(mapping, row);
        if (mapping.policy.definition.relations[expression.relation]?.isScope === true) {
            return this.#enclosing(expression, mapping, scope, trail);
        }

        // follow relationships to plain objects in the same scope
        const field = mapping.relations[expression.relation];
        if (!field) {
            return this.#throughRelationships(expression, mapping, row, scope, trail);
        }

        // follow the parent a field refers to
        return this.#throughField(expression, mapping, field, row, scope, trail);
    }

    /** Follow a row's relationships to plain objects in its scope, adding arrow conditions. */
    async #throughRelationships(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        row: Row,
        scope: string,
        trail: Trail,
    ): Promise<TupleTree> {
        // keep the relationships to plain objects in the row's scope
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);
        const relationships = (await this.#rowRelationships(mapping, row)).filter(
            (entry) =>
                entry.relation === expression.relation &&
                entry.subjectScope === scope &&
                entry.subjectRelation === null,
        );

        // collect the trees of each related object in turn
        const trees: TupleTree[] = [];
        for (const entry of relationships) {
            trees.push(
                ...(await this.#throughRelationship(expression, relation, entry, scope, trail)),
            );
        }

        return TupleTree.some(trees);
    }

    /** Collect the trees of one relationship's object, none for a type the relation refuses. */
    async #throughRelationship(
        expression: Extract<AccessExpression, { kind: "through" }>,
        relation: RelationDescription,
        entry: RelationshipRow,
        scope: string,
        trail: Trail,
    ): Promise<TupleTree[]> {
        // find the accepted subject type of the relationship
        const subject = relation.subjects.find(
            (type) => type.packageId === entry.subjectPackageId && type.type === entry.subjectType,
        );
        if (subject === undefined) {
            return [];
        }

        // decide the permission on the related object under the arrow's conditions
        const target = this.#authorizer.requireMapping(subject);
        const trees: TupleTree[] = [];
        for (const related of await this.#rows(target, scope, [entry.subjectId])) {
            trees.push(
                await this.#permission(
                    target.policy.permission(expression.permission),
                    target,
                    related,
                    {
                        arrows: [...trail.arrows, new TupleCondition(entry)],
                        path: [
                            ...trail.path,
                            `through relation ${expression.relation} to ${target.policy.definition.name} ${entry.subjectId}`,
                        ],
                    },
                ),
            );
        }

        return trees;
    }

    /** Follow a field's parent or its ancestors, in the row's scope or the field's scope. */
    async #throughField(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        field: TableMapping["relations"][string],
        row: Row,
        scope: string,
        trail: Trail,
    ): Promise<TupleTree> {
        // find the related type: the relation's one type, or the type in the row's columns
        const relation = this.#authorizer.relation(mapping.policy, expression.relation);
        const typed = field.subject;
        const subject =
            typed === undefined
                ? relation.subjects[0]
                : relation.subjects.find(
                      (type) =>
                          type.packageId === row[typed.packageId] && type.type === row[typed.type],
                  );
        if (subject === undefined) {
            return TupleTree.some([]);
        }

        // read the parent or the ancestors the field refers to
        const target = this.#authorizer.requireMapping(subject);
        const isUnset = row[field.column] === null || row[field.column] === undefined;
        const direct = isUnset ? [] : [TableMapping.text(row, field.column)];
        const ids = expression.transitive
            ? await this.#ancestors(mapping, expression.relation, row)
            : direct;

        // decide the permission on each related object, a global type's at the universe it lives in
        const within = target.scope === undefined ? Scope.universe.id : (field.scope ?? scope);
        const trees: TupleTree[] = [];
        for (const related of await this.#rows(target, within, ids)) {
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

        return TupleTree.some(trees);
    }

    /** Collect the chain objects' relationships and containment sets granting the permission. */
    async #enclosing(
        expression: Extract<AccessExpression, { kind: "through" }>,
        mapping: TableMapping,
        scope: string,
        trail: Trail,
    ): Promise<TupleTree> {
        // find the objects of the chain deciding the enclosing scope's permission
        const steps = this.#authorizer.chainSteps(mapping.policy, expression, this.#chain(scope));

        // grant to the containment sets of the chain scopes admitting their principals
        const contained = steps.flatMap(({ object, contained: admitted }) =>
            admitted === undefined
                ? []
                : [
                      TupleReader.#containment(object, admitted.principals, {
                          ...trail,
                          path: [
                              ...trail.path,
                              `${CONTAINED} in ${object.type} ${object.id} enclosing it`,
                          ],
                      }),
                  ],
        );

        // grant to the subjects of the relationships
        const tuples = await Promise.all(
            steps
                .filter(({ relations }) => relations.length > 0)
                .map(async ({ object, relations }) =>
                    (await this.#relationships(object)).flatMap((entry) =>
                        entry.relation !== null && relations.includes(entry.relation)
                            ? [
                                  {
                                      subject: TupleReader.#subject(entry),
                                      condition: new TupleCondition(entry),
                                      object,
                                      path: [
                                          ...trail.path,
                                          `relation ${entry.relation} on ${object.type} ${object.id} enclosing it`,
                                      ],
                                      arrows: trail.arrows,
                                  },
                              ]
                            : [],
                    ),
                ),
        );

        return TupleTree.some([TupleTree.any(tuples.flat()), ...contained]);
    }

    /** Collect the role bindings granting a permission on the row, ancestors, owners or chain. */
    async #bound(
        permission: PermissionReference | null,
        mapping: TableMapping,
        row: Row,
        trail: Trail,
    ): Promise<Tuple[]> {
        const bindings = [
            ...(await this.#covering(mapping, row)),
            ...(await this.#scopeBindings(TableMapping.scope(mapping, row))),
        ];

        return bindings.map((entry) => ({
            subject: TupleReader.#subject(entry),
            condition: new TupleCondition(entry),
            role: { id: entry.roleId, binding: entry.id, permission },
            object: TupleReader.#relatedObject(entry),
            path: [...trail.path, `role ${entry.roleId} bound on ${entry.type} ${entry.objectId}`],
            arrows: trail.arrows,
        }));
    }

    /** Read role bindings on the row, its ancestors or owning parent's chain, and its owner. */
    async #covering(mapping: TableMapping, row: Row): Promise<RoleBinding[]> {
        const [placed, owned] = await Promise.all([
            this.#placed(mapping, row),
            mapping.owner === undefined ? [] : this.#owned(mapping, mapping.owner, row),
        ]);

        return [...placed, ...owned];
    }

    /** Read the role bindings covering a row relation's owner, and on its enclosing scopes. */
    async #owned(mapping: TableMapping, relation: string, row: Row): Promise<RoleBinding[]> {
        // find the owner the relation's field refers to, none for an unset field
        const subject = this.#authorizer.onlySubject(mapping.policy, relation);
        const target = this.#authorizer.requireMapping(subject);
        const field = TableMapping.field(mapping, relation);
        const value = row[field.column];
        if (value === null || value === undefined) {
            return [];
        }
        const referred = schema.string().parse(value);

        // read the bindings of a scope owner and its enclosing scopes by its chain only
        if (target.policy.definition.scope === true) {
            return await this.#chainBindings(referred);
        }

        // read any other owner's coverage, and the bindings of its scope's chain
        const owners = await this.#rows(target, field.scope ?? TableMapping.scope(mapping, row), [
            referred,
        ]);
        const bindings = await Promise.all(
            owners.map(async (owner) => [
                ...(await this.#covering(target, owner)),
                ...(await this.#chainBindings(TableMapping.scope(target, owner))),
            ]),
        );

        return bindings.flat();
    }

    /** Read the role bindings on a scope's object and the scopes enclosing it, once per scope. */
    #chainBindings(scope: string): Promise<RoleBinding[]> {
        // reuse the bindings of a chain read before
        const known = this.#chainBound.get(scope);
        if (known !== undefined) {
            return known;
        }

        // read the chain's objects and their bindings
        const read = Scope.chain(this.#snapshot, scope).then(async (links) => {
            const bindings = await Promise.all(
                links.map((link) => this.#relationships(link.object)),
            );

            return bindings.flat().filter((relationship) => Relationship.isBinding(relationship));
        });
        this.#chainBound.set(scope, read);

        return read;
    }

    /** Read the role bindings on the row, its tree ancestors, or its owning parent's chain. */
    async #placed(mapping: TableMapping, row: Row): Promise<RoleBinding[]> {
        // read the bindings on the row
        const definition = mapping.policy.definition;
        const direct = (await this.#rowRelationships(mapping, row)).filter((relationship) =>
            Relationship.isBinding(relationship),
        );
        const parent = mapping.parent;
        const tree = parent === undefined ? undefined : mapping.trees?.[parent];

        // read the bindings on tree ancestors
        if (parent !== undefined && tree !== undefined) {
            const ancestors = await this.#ancestors(mapping, parent, row);
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

            return [
                ...direct,
                ...above.flat().filter((relationship) => Relationship.isBinding(relationship)),
            ];
        }
        // read the bindings covering the owning parent
        else if (parent !== undefined) {
            const subject = this.#authorizer.onlySubject(mapping.policy, parent);
            const target = this.#authorizer.requireMapping(subject);
            const parentId = row[TableMapping.field(mapping, parent).column];
            const parents =
                parentId === null || parentId === undefined
                    ? []
                    : await this.#rows(target, TableMapping.scope(mapping, row), [
                          schema.string().parse(parentId),
                      ]);
            const covered = await Promise.all(
                parents.map((owner) => this.#covering(target, owner)),
            );

            return [...direct, ...covered.flat()];
        }
        // read the bindings on the row only
        else {
            return direct;
        }
    }

    /** Read the role bindings on the objects of a scope's chain. */
    async #scopeBindings(scope: string): Promise<RoleBinding[]> {
        const bindings = await Promise.all(
            this.#chain(scope).map((object) => this.#relationships(object)),
        );

        return bindings.flat().filter((relationship) => Relationship.isBinding(relationship));
    }

    /** Read the chain deciding a scope's rows: its own when covered, the reader's otherwise. */
    #chain(scope: string): readonly ObjectReference[] {
        return this.#chains.get(scope) ?? this.#scopes;
    }

    /** Read the relationships on a mapped row's object. */
    #rowRelationships(mapping: TableMapping, row: Row): Promise<RelationshipRow[]> {
        return this.#relationships(TupleReader.#object(mapping, row));
    }

    /** Read every relationship on one object once. */
    #relationships(object: ObjectReference): Promise<RelationshipRow[]> {
        // reuse the relationships read before
        const key = ObjectReference.key(object);
        const known = this.#read.get(key);
        if (known) {
            return known;
        }

        // read them all and leave their conditions to each caller
        const read = Relationship.readByObject(this.#snapshot, [object]);
        this.#read.set(key, read);

        return read;
    }

    /** Read rows of a mapped type in a scope by identifier, in the order of the identifiers. */
    async #rows(mapping: TableMapping, scope: string, ids: readonly string[]): Promise<Row[]> {
        // read nothing for no identifiers
        if (ids.length === 0) {
            return [];
        }

        // read the unread rows in one query, and note the missing ones
        const { packageId, name } = mapping.policy.definition;
        const key = (id: string) => JSON.stringify([packageId, name, scope, id]);
        const missing = ids.filter((id) => !this.#loaded.has(key(id)));
        if (missing.length > 0) {
            const read = TableMapping.read(this.#snapshot, mapping, scope, missing).then(
                (rows) => new Map(rows.map((row) => [TableMapping.text(row, mapping.id), row])),
            );
            for (const id of missing) {
                this.#loaded.set(
                    key(id),
                    read.then((byId) => byId.get(id)),
                );
            }
        }

        // keep the order of the identifiers, nearest ancestors first
        const rows = await Promise.all(ids.map((id) => found(this.#loaded, key(id))));

        return rows.flatMap((row) => (row === undefined ? [] : [row]));
    }

    /** Read the identifiers of a row's proper ancestors through a tree. */
    async #ancestors(mapping: TableMapping, relation: string, row: Row): Promise<string[]> {
        // reuse the ancestry read before
        const tree = TableMapping.tree(mapping, relation);
        const key = TupleReader.#ancestryKey(
            relation,
            TableMapping.scope(mapping, row),
            TableMapping.text(row, mapping.id),
        );
        const known = this.#ancestry.get(key);
        if (known) {
            return known;
        }

        // read the node's proper ancestors
        const read = (async () =>
            (
                await tree.ancestry(this.#snapshot, TableMapping.scope(mapping, row), [
                    TableMapping.text(row, mapping.id),
                ])
            ).map((entry) => entry.ancestor))();
        this.#ancestry.set(key, read);

        return read;
    }

    /** Reference a mapped row's object. */
    static #object(mapping: TableMapping, row: Row): ObjectReference {
        const definition = mapping.policy.definition;

        return {
            packageId: definition.packageId,
            type: definition.name,
            scope: TableMapping.scope(mapping, row),
            id: schema.string().parse(row[mapping.id]),
        };
    }

    /** Reference the object a relationship relates. */
    static #relatedObject(entry: RelationshipRow): ObjectReference {
        return {
            packageId: entry.packageId,
            type: entry.type,
            scope: entry.objectScope,
            id: entry.objectId,
        };
    }

    /** Read a relationship's subject. */
    static #subject(entry: RelationshipRow): Subject {
        return {
            packageId: entry.subjectPackageId,
            type: entry.subjectType,
            scope: entry.subjectScope,
            id: entry.subjectId,
            ...(entry.subjectRelation === null ? {} : { relation: entry.subjectRelation }),
        };
    }

    /** Key a node's ancestry within one tree and scope. */
    static #ancestryKey(relation: string, scope: string, id: string): string {
        return JSON.stringify([relation, scope, id]);
    }

    /** Split values into chunks within the parameter budget. */
    static #chunks<Value>(values: readonly Value[]): Value[][] {
        const result: Value[][] = [];
        for (let start = 0; start < values.length; start += PREFETCH_CHUNK) {
            result.push(values.slice(start, start + PREFETCH_CHUNK));
        }

        return result;
    }
}

/** Read and decide tuple trees. */
export const TupleTree = {
    /** List every grant in a tree, for indexing and expiry. */
    flatten(tree: TupleTree): Tuple[] {
        switch (tree.kind) {
            case "any":
                return [...tree.tuples];
            case "some":
            case "all":
                return tree.trees.flatMap((branch) => TupleTree.flatten(branch));
            case "except":
                return [...TupleTree.flatten(tree.include), ...TupleTree.flatten(tree.exclude)];
            case "resource":
            case "context":
                return [];
        }
    },

    /** Decide whether a tree admits a caller's authority, as the compiled SQL predicate does. */
    permits(tree: TupleTree, access: Access): boolean {
        switch (tree.kind) {
            case "any":
                return tree.tuples.some(
                    (grant) => access.authority.failure(grant, access) === undefined,
                );
            case "some":
                return tree.trees.some((branch) => TupleTree.permits(branch, access));
            case "all":
                return tree.trees.every((branch) => TupleTree.permits(branch, access));
            case "except":
                return (
                    TupleTree.permits(tree.include, access) &&
                    !TupleTree.permits(tree.exclude, access)
                );
            case "resource":
            case "context": {
                // read the row's attributes, or the request's
                const row = tree.kind === "resource" ? tree.row : undefined;
                const request = (name: string) => AccessContext.attribute(access.context, name);

                return (
                    tree.match({
                        field: (name) => (row === undefined ? request(name) : row[name]),
                        placeholder: request,
                        exists: (via) => {
                            throw new AccessError(
                                "INVALID_DECLARATION",
                                `policy conditions cannot follow relations: ${via}`,
                            );
                        },
                        search: () => {
                            throw new AccessError(
                                "INVALID_DECLARATION",
                                "policy conditions cannot search",
                            );
                        },
                    }) === true
                );
            }
        }
    },

    /** Combine trees into one that admits when any of them admits. */
    some(trees: readonly TupleTree[]): TupleTree {
        return { kind: "some", trees };
    },

    /** Admit through any of some tuples. */
    any(tuples: readonly Tuple[]): TupleTree {
        return { kind: "any", tuples };
    },
};
