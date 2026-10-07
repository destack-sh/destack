import { type Condition, type QueryOptions, type Relation, Relations } from "@destack/db";
import { Scope, type Query } from "@destack/sync";
import { ServiceError } from "@destack/service/error";
import { present } from "@destack/schema";
import type { ObjectQuery, OpenQueryOptions } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import { CHUNKS } from "../text/chunk.ts";

/** The relations of each served list of object types, derived once. */
const SCHEMAS = new WeakMap<readonly ObjectType[], Relations>();

/** Compile object queries of a scope chain, nearest first, into queries of their tables. */
export function compileQueries(
    objects: readonly ObjectType[],
    queries: Readonly<Record<string, ObjectQuery>> | undefined,
    chain: readonly string[],
): Record<string, Query> {
    // default to every listed object type of the scope's level
    const scope = present(chain[0], "the scope chain's first scope");
    const isGlobal = scope === Scope.universe.id;
    const named = queries ?? defaultQueries(objects, isGlobal);

    // compile each query
    return Object.fromEntries(
        Object.entries(named).map(([name, query]) => {
            // find the object type and require its scope level
            const { object: objectName, ...options } = query;
            const object = objects.find((entry) => entry.name === objectName);
            if (!object) {
                throw new ServiceError("BAD_REQUEST", { message: `no object ${objectName}` });
            } else if ((object.scope === Scope.universe.id) !== isGlobal) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `object ${objectName} is not in scope ${scope}`,
                });
            }

            return [name, { ...compile(object, options, objects), scopes: object.scopesOf(chain) }];
        }),
    );
}

/** Query every listed object type of a scope level by its name, a recoverable type's trash included. */
function defaultQueries(
    objects: readonly ObjectType[],
    isGlobal: boolean,
): Record<string, ObjectQuery> {
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
}

/** Compile one object query: its table, the served types' relations and what it selects. */
export function compile(
    object: ObjectType,
    options: OpenQueryOptions,
    objects: readonly ObjectType[],
): Omit<Query, "scopes"> {
    return {
        table: object.table,
        relations: relationsOf(objects),
        ...optionsOf(object, options, objects, true),
    };
}

/** Compile what a query or an included relation selects, requiring a listed object type. */
function optionsOf(
    object: ObjectType,
    options: OpenQueryOptions,
    objects: readonly ObjectType[],
    isRoot: boolean,
): QueryOptions {
    // require a listed object type, and trash selection at the root only
    if (object.listing === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `object ${object.name} is not listed` });
    } else if (options.deleted !== undefined && object.lifecycle.recoverable === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `object ${object.name} has no trash` });
    } else if (options.deleted !== undefined && !isRoot) {
        throw new ServiceError("BAD_REQUEST", {
            message: `included ${object.name} objects leave their trash out`,
        });
    }

    // include the named relations, a text holder's chunks among them
    const includes =
        object.text.length === 0 || options.aggregate !== undefined
            ? options.with
            : { ...options.with, [CHUNKS]: {} };
    const included = Object.fromEntries(
        Object.entries(includes ?? {}).map(([name, selected]): [string, QueryOptions] => [
            name,
            optionsOf(
                related(object, name, objects).object,
                selected === true ? {} : selected,
                objects,
                false,
            ),
        ]),
    );

    // filter a root's trashed rows, which relations leave out for included ones
    const trash = isRoot ? trashCondition(object, options) : undefined;
    const where = joined([
        ...(options.where === undefined ? [] : [options.where]),
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
}

/** Select a root query's trashed or kept rows of a recoverable type, absent for every row. */
function trashCondition(object: ObjectType, options: OpenQueryOptions): Condition | undefined {
    const kept = { deletionRequestedAt: { isNull: true } };
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
}

/** Find the object type a relation of a type leads to, with the relation. */
export function related(
    object: ObjectType,
    name: string,
    objects: readonly ObjectType[],
): { readonly object: ObjectType; readonly relation: Relation } {
    // read the relation, refusing a name the type does not relate by
    const relation = relationsOf(objects).of(object.table)[name];
    const target =
        relation === undefined
            ? undefined
            : objects.find((entry) => entry.table === relation.table);
    if (relation === undefined || target === undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: `object ${object.name} includes nothing named ${name}`,
        });
    }

    return { object: target, relation };
}

/** Derive the relations of served object types: references, parents, children, attachments and join types. */
export function relationsOf(objects: readonly ObjectType[]): Relations {
    // derive once per served list
    const known = SCHEMAS.get(objects);
    if (known !== undefined) {
        return known;
    }

    // relate each listed type to the listed types it references, nests in or holds
    const listed = objects.filter((object) => object.listing !== undefined);
    const tables = new Map<ObjectType["table"], Record<string, Relation>>();
    for (const object of listed) {
        tables.set(object.table, directRelations(object, listed));
    }

    // relate through join types: a child type's reference to a third type
    for (const named of tables.values()) {
        for (const [near, first] of Object.entries(named)) {
            for (const [far, relation] of junctions(first, tables)) {
                named[`${near}.${far}`] ??= relation;
            }
        }
    }

    // keep the relations of this list
    const relations = new Relations(tables);
    SCHEMAS.set(objects, relations);

    return relations;
}

/** Relate through a to-many relation's join type to each type it references, keyed by the reference name. */
function junctions(
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
}

/** Relate a type to the types it references, its parent, and its children, attachments and referrers. */
function directRelations(
    object: ObjectType,
    listed: readonly ObjectType[],
): Record<string, Relation> {
    // name each relation once, the first declaration winning
    const named: Record<string, Relation> = {};
    const relate = (name: string, relation: Relation) => {
        named[name] ??= relation;
    };

    // relate the type a reference names, and the parent of a nested type
    for (const [column, field] of Object.entries(object.fields)) {
        const served = listed.find((entry) => entry.same(field.target?.()));
        if (field.relation !== undefined && !field.qualified && served !== undefined) {
            relate(field.relation, one(served, { kind: "key", column: "id", parent: column }));
        }
    }
    const parent = listed.find((entry) => entry.same(object.parent?.object));
    if (parent !== undefined && !parent.same(object)) {
        relate("parent", one(parent, { kind: "key", column: "id", parent: "parentId" }));
    }

    // relate the children nested in, attached to or referencing the type
    for (const children of listed) {
        const isAttached = !children.same(object) && object.attaches(children);
        const column = childColumn(children, object, isAttached);
        if (column !== undefined) {
            const where = isAttached
                ? { parentPackageId: object.policy.definition.packageId, parentType: object.name }
                : undefined;
            relate(children.plural, many(children, { kind: "key", column, parent: "id" }, where));
        }
    }

    return named;
}

/** Name the column relating children to a type: the parent column, or their only reference to it. */
function childColumn(
    children: ObjectType,
    object: ObjectType,
    isAttached: boolean,
): string | undefined {
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
        return referenceTo(children, object);
    }
}

/** Relate one object of a type, leaving its trash out. */
function one(target: ObjectType, on: Relation["on"]): Relation {
    return keep(target, { table: target.table, cardinality: "one", on });
}

/** Relate many objects of a type, leaving its trash out. */
function many(target: ObjectType, on: Relation["on"], where: Condition | undefined): Relation {
    return keep(target, {
        table: target.table,
        cardinality: "many",
        on,
        ...(where === undefined ? {} : { where }),
    });
}

/** Leave a recoverable type's trashed objects out of a relation. */
function keep(target: ObjectType, relation: Relation): Relation {
    const where = joined([
        ...(relation.where === undefined ? [] : [relation.where]),
        ...(target.lifecycle.recoverable === undefined
            ? []
            : [{ deletionRequestedAt: { isNull: true } }]),
    ]);

    return where === undefined ? relation : { ...relation, where };
}

/** Name a type's only unqualified reference field to another type. */
function referenceTo(from: ObjectType, to: ObjectType): string | undefined {
    const fields = Object.entries(from.fields).filter(
        ([, field]) => field.type === "reference" && !field.qualified && to.same(field.target?.()),
    );
    const [only, ...others] = fields;

    return only !== undefined && others.length === 0 ? only[0] : undefined;
}

/** Join conditions into one, a single condition as itself, absent when there are none. */
function joined(conditions: readonly Condition[]): Condition | undefined {
    const [only, ...others] = conditions;

    return only === undefined || others.length === 0 ? only : { AND: conditions };
}
