import { type Condition, FindOptions, type Relation, Relations } from "@destack/db";
import { Scope, type Query } from "@destack/sync";
import { CallError } from "@destack/runtime/error";
import { present } from "@destack/schema";
import type { ObjectQuery } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import { CHUNKS } from "../text/chunk.ts";
import { ObjectSearch } from "../trait/search.ts";

/** Compile object queries of a scope chain, nearest first, into queries of their tables. */
export function compileQueries(
    objects: readonly ObjectType[],
    queries: Readonly<Record<string, ObjectQuery>> | undefined,
    chain: readonly string[],
): Record<string, Query> {
    // default to every listed object type of the scope's level
    const scope = present(chain[0], "the scope chain's first scope");
    const isGlobal = scope === Scope.universe.id;
    const named = queries ?? QueryDerivation.defaultQueries(objects, isGlobal);

    // compile each query
    return Object.fromEntries(
        Object.entries(named).map(([name, query]): [string, Query] => {
            // find the object type and require its scope level
            const { object: objectName, scopes, ...options } = query;
            const object = objects.find((entry) => entry.name === objectName);
            if (!object) {
                throw new CallError("BAD_REQUEST", { message: `no object ${objectName}` });
            }
            // read every scope's rows, each decided by its own access
            else if (scopes === "every") {
                return [name, { ...compile(object, options, objects), scopes }];
            }
            // refuse a type of another scope level
            else if ((object.scope === Scope.universe.id) !== isGlobal) {
                throw new CallError("BAD_REQUEST", {
                    message: `object ${objectName} is not in scope ${scope}`,
                });
            }

            return [
                name,
                { ...compile(object, options, objects), scopes: object.readScopes(chain) },
            ];
        }),
    );
}

/** Compile one object query: its table, the served types' relations and what it selects. */
export function compile(
    object: ObjectType,
    options: Omit<ObjectQuery, "object">,
    objects: readonly ObjectType[],
): Omit<Query, "scopes"> {
    return {
        table: object.table,
        relations: RelationGraph.of(objects),
        ...QueryDerivation.compileOptions(object, options, objects, true),
    };
}

/** Find the object type a relation of a type leads to, with the relation. */
export function related(
    object: ObjectType,
    name: string,
    objects: readonly ObjectType[],
): { readonly object: ObjectType; readonly relation: Relation } {
    // read the relation, refusing a name the type does not relate by
    const relation = RelationGraph.of(objects).of(object.table)[name];
    const target =
        relation === undefined
            ? undefined
            : objects.find((entry) => entry.table === relation.table);
    if (relation === undefined || target === undefined) {
        throw new CallError("BAD_REQUEST", {
            message: `object ${object.name} has no include named ${name}`,
        });
    }

    return { object: target, relation };
}

/** The relations of a list of served object types, derived once per list. */
export class RelationGraph {
    /** The graph of each list derived before. */
    static readonly #known = new WeakMap<readonly ObjectType[], RelationGraph>();
    /** The relations between the listed types. */
    readonly relations: Relations;

    /** Derive served types' relations: references, parents, children, attachments, join types. */
    private constructor(objects: readonly ObjectType[]) {
        // relate each listed type to the listed types it references, nests in or contains
        const listed = objects.filter((object) => object.listing !== undefined);
        const tables = new Map<ObjectType["table"], Record<string, Relation>>();
        for (const object of listed) {
            tables.set(object.table, QueryDerivation.directRelations(object, listed));
        }

        // relate through join types: a child type's reference to a third type
        for (const named of tables.values()) {
            for (const [near, first] of Object.entries(named)) {
                for (const [far, relation] of QueryDerivation.junctions(first, tables)) {
                    named[`${near}.${far}`] ??= relation;
                }
            }
        }
        this.relations = new Relations(tables);
    }

    /** Read the relations of a served list, deriving them once per list. */
    static of(objects: readonly ObjectType[]): Relations {
        const graph = RelationGraph.#known.get(objects) ?? new RelationGraph(objects);
        RelationGraph.#known.set(objects, graph);

        return graph.relations;
    }
}

/** The derivation of object types' queries: defaults, relations, joins and trash conditions. */
const QueryDerivation = {
    /** Query every listed object type of a scope level by name, including recoverable trash. */
    defaultQueries(objects: readonly ObjectType[], isGlobal: boolean): Record<string, ObjectQuery> {
        const listed = objects.filter(
            (object) =>
                object.listing !== undefined && (object.scope === Scope.universe.id) === isGlobal,
        );

        return Object.fromEntries(
            listed.map((object) => [
                object.name,
                {
                    object: object.name,
                    ...(object.lifecycle.recoverable === undefined
                        ? {}
                        : { deleted: "include" as const }),
                },
            ]),
        );
    },

    /** Compile what a query or an included relation selects, requiring a listed object type. */
    compileOptions(
        object: ObjectType,
        options: Omit<ObjectQuery, "object">,
        objects: readonly ObjectType[],
        isRoot: boolean,
    ): FindOptions {
        // require a listed object type, and trash selection at the root only
        if (object.listing === undefined) {
            throw new CallError("BAD_REQUEST", { message: `object ${object.name} is not listed` });
        } else if (options.deleted !== undefined && object.lifecycle.recoverable === undefined) {
            throw new CallError("BAD_REQUEST", { message: `object ${object.name} has no trash` });
        } else if (options.deleted !== undefined && !isRoot) {
            throw new CallError("BAD_REQUEST", {
                message: `included ${object.name} objects cannot select their trash`,
            });
        }

        // include the requested relations, a text holder's chunks among them
        const includes =
            object.text.length === 0 || options.aggregate !== undefined
                ? options.with
                : { ...options.with, [CHUNKS]: {} };
        const included = Object.fromEntries(
            FindOptions.includes({ with: includes ?? {} }).map(
                ([name, selected]): [string, FindOptions] => [
                    name,
                    QueryDerivation.compileOptions(
                        related(object, name, objects).object,
                        selected,
                        objects,
                        false,
                    ),
                ],
            ),
        );

        // filter a root's trashed rows, which relations leave out for included ones
        const trash = isRoot ? QueryDerivation.trashCondition(object, options) : undefined;
        const where = QueryDerivation.joined([
            ...(options.where === undefined ? [] : [ObjectSearch.expand(object, options.where)]),
            ...(trash === undefined ? [] : [trash]),
        ]);

        return {
            ...(options.extras === undefined ? {} : { extras: options.extras }),
            ...(where === undefined ? {} : { where }),
            ...(options.orderBy !== undefined
                ? { orderBy: options.orderBy }
                : object.ordering === undefined
                  ? {}
                  : { orderBy: { position: "asc", id: "asc" } }),
            ...(options.limit === undefined ? {} : { limit: options.limit }),
            ...(options.aggregate === undefined ? {} : { aggregate: options.aggregate }),
            ...(Object.keys(included).length === 0 ? {} : { with: included }),
        };
    },

    /** Select a root query's trashed or kept rows of a recoverable type, absent for every row. */
    trashCondition(
        object: ObjectType,
        options: Omit<ObjectQuery, "object">,
    ): Condition | undefined {
        const kept = { deletedAt: { isNull: true } };
        const deleted = options.deleted ?? "exclude";

        // keep every row of a type without trash, or when asked to
        if (object.lifecycle.recoverable === undefined || deleted === "include") {
            return undefined;
        }
        // keep only the trashed rows
        else if (deleted === "only") {
            return { NOT: kept };
        }
        // leave the trashed rows out
        else {
            return kept;
        }
    },

    /** Relate through a to-many join type to each referenced type, keyed by reference name. */
    junctions(
        first: Relation,
        tables: ReadonlyMap<ObjectType["table"], Readonly<Record<string, Relation>>>,
    ): [string, Relation][] {
        // join through a keyed to-many relation only
        const joins = tables.get(first.table);
        if (first.cardinality !== "many" || first.on.kind !== "key" || joins === undefined) {
            return [];
        }

        // join each keyed reference of the join type other than its parent
        const { column } = first.on;

        return Object.entries(joins).flatMap(([far, second]): [string, Relation][] => {
            if (second.cardinality !== "one" || second.on.kind !== "key" || far === "parent") {
                return [];
            }
            const on = {
                kind: "junction" as const,
                table: first.table,
                from: { column, key: "id" },
                to: { column: second.on.parent, key: "id" },
            };

            return [[far, { table: second.table, cardinality: "many", on }]];
        });
    },

    /** Relate a type to its referenced types, parent, children, attachments and referrers. */
    directRelations(object: ObjectType, listed: readonly ObjectType[]): Record<string, Relation> {
        // key each relation once, the first declaration winning
        const named: Record<string, Relation> = {};
        const relate = (name: string, relation: Relation) => {
            named[name] ??= relation;
        };

        // relate the type a reference refers to, and the parent of a nested type
        for (const [column, field] of Object.entries(object.fields)) {
            const served = listed.find((entry) => entry.same(field.target?.()));
            if (field.relation !== undefined && served !== undefined) {
                relate(
                    field.relation,
                    QueryDerivation.one(served, { kind: "key", column: "id", parent: column }),
                );
            }
        }
        const parent = listed.find((entry) => entry.same(object.parent?.object));
        if (parent !== undefined && !parent.same(object)) {
            relate(
                "parent",
                QueryDerivation.one(parent, { kind: "key", column: "id", parent: "parentId" }),
            );
        }

        // relate the children nested in, attached to or referencing the type
        for (const children of listed) {
            const isAttached = !children.same(object) && object.attaches(children);
            const column = QueryDerivation.childColumn(children, object, isAttached);
            if (column !== undefined) {
                const where = isAttached
                    ? {
                          parentPackageId: object.policy.definition.packageId,
                          parentType: object.name,
                      }
                    : undefined;
                relate(
                    children.plural,
                    QueryDerivation.many(children, { kind: "key", column, parent: "id" }, where),
                );
            }
        }

        return named;
    },

    /** Find the column relating children to a type: the parent column, or their one reference. */
    childColumn(children: ObjectType, object: ObjectType, isAttached: boolean): string | undefined {
        // relate a type to itself through its parent relation only
        if (children.same(object)) {
            return undefined;
        }
        // relate nested and attached children by parent
        else if (object.same(children.parent?.object) || isAttached) {
            return "parentId";
        }
        // relate referrers by their only reference
        else {
            return QueryDerivation.referenceTo(children, object);
        }
    },

    /** Relate one object of a type, leaving its trash out. */
    one(target: ObjectType, on: Relation["on"]): Relation {
        return QueryDerivation.keep(target, { table: target.table, cardinality: "one", on });
    },

    /** Relate many objects of a type, leaving its trash out. */
    many(target: ObjectType, on: Relation["on"], where: Condition | undefined): Relation {
        return QueryDerivation.keep(target, {
            table: target.table,
            cardinality: "many",
            on,
            ...(where === undefined ? {} : { where }),
        });
    },

    /** Leave a recoverable type's trashed objects out of a relation. */
    keep(target: ObjectType, relation: Relation): Relation {
        const where = QueryDerivation.joined([
            ...(relation.where === undefined ? [] : [relation.where]),
            ...(target.lifecycle.recoverable === undefined
                ? []
                : [{ deletedAt: { isNull: true } }]),
        ]);

        return where === undefined ? relation : { ...relation, where };
    },

    /** Find a type's only unqualified reference field to another type. */
    referenceTo(from: ObjectType, to: ObjectType): string | undefined {
        const fields = Object.entries(from.fields).filter(
            ([, field]) => field.type === "reference" && to.same(field.target?.()),
        );
        const [only, ...others] = fields;

        return only !== undefined && others.length === 0 ? only[0] : undefined;
    },

    /** Join conditions into one, a single condition as itself, absent when there are none. */
    joined(conditions: readonly Condition[]): Condition | undefined {
        const [only, ...others] = conditions;

        return only === undefined || others.length === 0 ? only : { AND: conditions };
    },
};
