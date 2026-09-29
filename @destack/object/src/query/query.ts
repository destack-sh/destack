import { type Table } from "@destack/db";
import { Scope, type Include, type Path, type Query, type Relation } from "@destack/sync";
import { Condition, Expression } from "@destack/db/query";
import { ServiceError } from "@destack/service/error";
import type { ObjectInclude, ObjectQuery } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import { CHUNKS } from "../text/chunk.ts";

/** Compile object queries of a scope chain, nearest first, into queries of their tables. */
export function compileQueries(
    objects: readonly ObjectType[],
    queries: Readonly<Record<string, ObjectQuery>> | undefined,
    chain: readonly string[],
): Record<string, Query> {
    // default to every listed object type of the scope's level
    const scope = chain[0]!;
    const isGlobal = scope === Scope.universe.id;
    const named =
        queries ??
        Object.fromEntries(
            objects
                .filter(
                    (object) =>
                        object.listing !== undefined &&
                        (object.scope === Scope.universe.id) === isGlobal,
                )
                .map((object) => [
                    object.name,
                    {
                        object: object.name,
                        ...(object.recoverable === undefined
                            ? {}
                            : { deleted: "include" as const }),
                    },
                ]),
        );

    // compile each query
    return Object.fromEntries(
        Object.entries(named).map(([name, query]) => {
            // find the object type and require its scope level
            const { object: objectName, ...shape } = query;
            const object = objects.find((entry) => entry.name === objectName);
            if (!object) {
                throw new ServiceError("BAD_REQUEST", { message: `no object ${objectName}` });
            } else if ((object.scope === Scope.universe.id) !== isGlobal) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `object ${objectName} is not in scope ${scope}`,
                });
            }

            return [name, { ...compile(object, shape, objects), scopes: object.scopesOf(chain) }];
        }),
    );
}

/** Compile one object query or include, requiring a listed object type. */
export function compile(
    object: ObjectType,
    shape: ObjectInclude,
    objects: readonly ObjectType[],
): Omit<Query, "scopes"> {
    // require a listed object type
    if (object.listing === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `object ${object.name} is not listed` });
    }

    // compile the includes, a text holder's chunks among them
    const includes =
        object.text.length === 0 || shape.aggregate !== undefined
            ? shape.include
            : { ...shape.include, [CHUNKS]: {} };
    const include = Object.fromEntries(
        Object.entries(includes ?? {}).map(([name, nested]): [string, Include] => {
            const { object: target, on, where } = join(object, nested.via ?? name, objects);
            const compiled = compile(target, nested, objects);

            return [
                name,
                {
                    ...compiled,
                    on,
                    ...(where === undefined
                        ? {}
                        : {
                              where:
                                  compiled.where === undefined
                                      ? where
                                      : Condition.all(compiled.where, where),
                          }),
                },
            ];
        }),
    );

    // filter trashed rows
    if (shape.deleted !== undefined && object.recoverable === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `object ${object.name} has no trash` });
    }
    const kept = Condition.missing("deletionRequestedAt");
    const deleted = object.recoverable === undefined ? "include" : (shape.deleted ?? "exclude");
    const where = [
        ...(shape.where === undefined ? [] : [shape.where]),
        ...(deleted === "exclude" ? [kept] : deleted === "only" ? [Condition.not(kept)] : []),
    ];

    // resolve the relations
    const relations = relationsOf(object, shape.where, Object.values(shape.compute ?? {}), objects);

    return {
        table: object.table as Table,
        ...(Object.keys(relations).length === 0 ? {} : { relations }),
        ...(shape.compute === undefined ? {} : { compute: shape.compute }),
        ...(where.length === 0
            ? {}
            : { where: where.length === 1 ? where[0]! : Condition.all(...where) }),
        ...(shape.order === undefined ? {} : { order: shape.order }),
        ...(shape.limit === undefined ? {} : { limit: shape.limit }),
        ...(shape.aggregate === undefined ? {} : { aggregate: shape.aggregate }),
        ...(Object.keys(include).length === 0 ? {} : { include }),
    };
}

/** Resolve the relations a condition's terms, lookups and rollups follow. */
function relationsOf(
    object: ObjectType,
    condition: Condition | undefined,
    computed: readonly Expression[],
    objects: readonly ObjectType[],
): Record<string, Relation> {
    // gather the relation terms
    const terms = [
        ...Condition.relations(condition ?? Condition.all()),
        ...computed.flatMap((expression) => [
            ...Expression.lookups(expression).map(({ via }) => ({ via, where: undefined })),
            ...Expression.rollups(expression),
        ]),
    ];
    const relations: Record<string, Relation> = {};
    for (const via of new Set(terms.map((term) => term.via))) {
        // join a listed related type
        const { object: target, on, where } = join(object, via, objects);
        if (target.listing === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `object ${target.name} is not listed`,
            });
        }

        // relate untrashed objects with nested relations
        const wheres = terms.flatMap((term) =>
            term.via === via && term.where !== undefined ? [term.where] : [],
        );
        const nested = relationsOf(target, Condition.all(...wheres), [], objects);
        const kept = [
            ...(where === undefined ? [] : [where]),
            ...(target.recoverable === undefined ? [] : [Condition.missing("deletionRequestedAt")]),
        ];
        relations[via] = {
            table: target.table as Table,
            on,
            ...(kept.length === 0
                ? {}
                : { where: kept.length === 1 ? kept[0]! : Condition.all(...kept) }),
            ...(Object.keys(nested).length === 0 ? {} : { relations: nested }),
        };
    }

    return relations;
}

/** The object type an include joins and how. */
export interface Join {
    /** The joined object type. */
    readonly object: ObjectType;
    /** The path from the including rows to the joined ones. */
    readonly on: Path;
    /** The condition the joined rows meet, for joins through a join type. */
    readonly where?: Condition;
}

/** Resolve the name an include follows into its join. */
export function join(object: ObjectType, name: string, objects: readonly ObjectType[]): Join {
    // join through a join type
    const hops = name.split(".");
    if (hops.length === 2) {
        const first = join(object, hops[0]!, objects);
        const second = join(first.object, hops[1]!, objects);
        if (
            first.on.kind === "key" &&
            first.on.parent === "id" &&
            second.on.kind === "key" &&
            second.on.column === "id"
        ) {
            return {
                object: second.object,
                on: {
                    kind: "junction",
                    table: first.object.table as Table,
                    from: { column: first.on.column, key: "id" },
                    to: { column: second.on.parent, key: "id" },
                },
            };
        }
    }
    // join the rows below or above a row of a tree
    else if (
        (name === "descendants" || name === "ancestors") &&
        object.same(object.parent?.object)
    ) {
        return { object, on: { kind: name, column: "parentId" } };
    }
    // join the parent, a reference, or children
    else if (hops.length === 1) {
        const field = object.fields[name];
        const parent = object.parent?.object;
        const target =
            name === "parent" && parent !== undefined && parent !== "any"
                ? { object: parent, column: "parentId" }
                : field?.type === "reference" && field.target && !field.qualified
                  ? { object: field.target(), column: name }
                  : undefined;
        const served = target && objects.find((entry) => entry.same(target.object));
        if (target !== undefined && served !== undefined) {
            return {
                object: served,
                on: { kind: "key", column: "id", parent: target.column },
            };
        }
        const children = objects.find((entry) => entry.plural === name && !entry.same(object));
        const isAttached = children !== undefined && object.holds(children);
        const column =
            children === undefined
                ? undefined
                : object.same(children.parent?.object) || isAttached
                  ? "parentId"
                  : referenceTo(children, object);
        if (children !== undefined && column !== undefined) {
            return {
                object: children,
                on: { kind: "key", column, parent: "id" },
                ...(isAttached
                    ? {
                          where: Condition.all(
                              Condition.eq("parentPackageId", object.policy.definition.packageId),
                              Condition.eq("parentType", object.name),
                          ),
                      }
                    : {}),
            };
        }
    }

    // refuse any other name
    throw new ServiceError("BAD_REQUEST", {
        message: `object ${object.name} includes nothing named ${name}`,
    });
}

/** Name a type's only unqualified reference field to another type. */
function referenceTo(from: ObjectType, to: ObjectType): string | undefined {
    const fields = Object.entries(from.fields).filter(
        ([, field]) => field.type === "reference" && !field.qualified && to.same(field.target?.()),
    );

    return fields.length === 1 ? fields[0]![0] : undefined;
}
