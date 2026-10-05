import { ObjectReference } from "@destack/sync";
import type * as sync from "@destack/sync";
import { validated } from "../field/field.ts";
import { defineSchema, type Identifier, present, schema } from "@destack/schema";
import { type ColumnBuilder, and, eq, identifier, index, TABLE, text } from "@destack/db";
import type { PackageId } from "@destack/package";
import { ServiceError } from "@destack/service/error";
import type { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import { defineMethod, type Method } from "../method/method.ts";
import {
    type DestinationShape,
    type Procedure,
    RevisionShape,
    type ReplayShape,
    type RevisionField,
    type RowSchema,
    type TargetShape,
} from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import { keyColumn } from "../field/field.ts";
import type { FieldColumn } from "../field/field.ts";
import type { ObjectTable } from "../object/table.ts";
import type { Trait } from "./trait.ts";

/** The parent a tree node names, null for a root. */
const PARENT = schema.string().nullable();

/** A parent of any type, in the scope of the object naming it. */
export const ParentReference = defineSchema(ObjectReference.omit({ scope: true }));
/** A parent of any type, in the scope of the object naming it. */
export type ParentReference = schema.Infer<typeof ParentReference>;

/** The options of nesting under a parent. */
export type NestedDefinition = {
    /** The parent type, "self" for a tree, or "any" for attachments. */
    readonly in: ObjectType | "self" | "any";
    /** Whether an object may have no parent. */
    readonly optional?: boolean;
    /** Delete children with their parent, or refuse deleting a parent with children. */
    readonly delete?: "cascade" | "restrict";
    /** The parent permission for creating or moving an object under it. */
    readonly receive: string;
    /** The permission moving an object to another parent. */
    readonly move?: string;
};

/** The columns referencing the parent. */
export type ParentBuilderMap<Name extends string, Parent> = Parent extends {
    readonly in: string;
    readonly optional?: boolean;
}
    ? Parent["in"] extends "any"
        ? {
              parentPackageId: ColumnBuilder<FieldColumn<PackageId, IsRequired<Parent>, false>>;
              parentType: ColumnBuilder<FieldColumn<string, IsRequired<Parent>, false>>;
              parentId: ColumnBuilder<FieldColumn<string, IsRequired<Parent>, false>>;
          }
        : {
              parentId: ColumnBuilder<
                  FieldColumn<
                      Parent["in"] extends "self" ? Identifier<Name> : Identifier<Parent["in"]>,
                      IsRequired<Parent>,
                      false
                  >
              >;
          }
    : {};

/** Whether every object needs a parent. */
type IsRequired<Parent extends { readonly optional?: boolean }> = Parent["optional"] extends true
    ? false
    : true;

/** The method moving objects between parents. */
export type NestedMethodMap<Parent> = Parent extends {
    readonly move: infer Permission extends string;
}
    ? { readonly move: Method<{ kind: "move"; permission: Permission; mutates: true }> }
    : {};

/** Objects nested under a parent, or under an object of their type in a tree. */
export const nested: Trait<NestedDefinition> = {
    key: "nested",
    options: (definition) => definition.nested,
    columns: (options, object): Record<string, ColumnBuilder> => {
        // refer to a parent of any type by package, type and identifier
        if (options.in === "any") {
            const columns = {
                parentPackageId: identifier("parent_package_id", "package"),
                parentType: text("parent_type"),
                parentId: text("parent_id"),
            };

            return options.optional === true
                ? columns
                : {
                      parentPackageId: columns.parentPackageId.notNull(),
                      parentType: columns.parentType.notNull(),
                      parentId: columns.parentId.notNull(),
                  };
        }

        // reference the owning object, or the table itself for trees
        const owner = options.in === "self" ? undefined : options.in;
        const keyed = owner?.keyed ?? (owner === undefined ? object.key : undefined);
        const column = (
            keyed === undefined
                ? identifier("parent_id", owner?.identity ?? object.identity)
                : validated("parent_id", keyed)
        ).references(() => keyColumn(owner?.table ?? object.table()), {
            onDelete: options.delete ?? "cascade",
        });

        return { parentId: options.optional === true ? column : column.notNull() };
    },
    constraints: (options, table, columns) =>
        options.in === "any"
            ? [
                  index(`${table}_parent`).on(
                      present(columns["parentPackageId"], "the parent package column"),
                      present(columns["parentType"], "the parent type column"),
                      present(columns["parentId"], "the parent column"),
                  ),
              ]
            : [],
    // index the ancestry of trees
    table: (options) =>
        options.in === "self" ? { tree: { id: "id", scope: "scope", parent: "parentId" } } : {},
    policy: (options, object) => {
        // relate objects to a parent of any type through an open relation
        if (options.in === "any") {
            return { relations: { parent: { grantedBy: null, subjects: [], open: true } } };
        }

        // relate objects to a parent of one type
        const subject =
            options.in === "self"
                ? { packageId: object.packageId, type: object.name }
                : options.in.policy;

        return { relations: { parent: { grantedBy: null, subjects: [subject] } } };
    },
    mapping: (options, object) => {
        // find a parent of any type by its package, type and identifier
        if (options.in === "any") {
            return {
                relations: {
                    parent: {
                        column: "parentId",
                        subject: {
                            packageId: "parentPackageId",
                            type: "parentType",
                            scope: "scope",
                        },
                    },
                },
            };
        }

        // find a parent of one type and a tree's ancestors
        const tree = object.table()[TABLE].tree;

        return {
            relations: { parent: { column: "parentId" } },
            parent: "parent",
            ...(tree === undefined ? {} : { trees: { parent: tree } }),
        };
    },
    methods: (options): Record<string, Method> =>
        options.move === undefined ? {} : { move: move(options.move, options) },
    validate: (options, object) => {
        // require ephemeral objects to attach to any type
        if (object.storage === "ephemeral" && options.in !== "any") {
            throw new TypeError(
                `ephemeral object ${object.name} attaches to its parent as an attachment of any type`,
            );
        }

        // require trees to have roots
        if (options.in === "self" && options.optional !== true) {
            throw new TypeError(`${object.name} is a tree, so its parent must be optional`);
        }

        // require the receive permission on the parent type
        const parent = options.in === "self" ? object : options.in;
        if (parent !== "any" && !parent.permissions.includes(options.receive)) {
            throw new TypeError(
                `object ${object.name} has parents lacking permission ${options.receive}`,
            );
        }
    },
};

/** Move objects to another parent and keep trees acyclic. */
function move<const Permission extends string>(
    permission: Permission,
    nesting: NestedDefinition,
): Method<{ kind: "move"; permission: Permission; mutates: true }> {
    return defineMethod<{ kind: "move"; permission: Permission; mutates: true }>({
        kind: "move",
        permission,
        mutates: true,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/move" },
            input: shapes.target.extend({
                ...shapes.replay,
                ...RevisionShape,
                ...shapes.destination,
            }),
            output: shapes.row,
        }),
        handler: (call: Call<ObjectTable>) =>
            call.update(call.object.table[TABLE].values(call.parentColumns())),
        inverse: (step) => moveBack(step, nesting),
        async execute(call) {
            // require a valid destination before moving
            await requireDestination(call, nesting);

            return this.handler(call);
        },
    });
}

/** Move an object back under its former parent. */
function moveBack(step: Step, nesting: NestedDefinition): readonly sync.Call[] | undefined {
    // name the former parent, of one type or any
    const before = step.before;
    if (before === undefined) {
        return undefined;
    }
    const destination =
        nesting.in !== "any"
            ? { parentId: before["parentId"] ?? null }
            : {
                  parent:
                      before["parentId"] === null || before["parentId"] === undefined
                          ? null
                          : ParentReference.parse({
                                packageId: before["parentPackageId"],
                                type: before["parentType"],
                                id: before["parentId"],
                            }),
              };

    return [
        Step.record(step, step.name, {
            ...Step.target(step),
            ...destination,
        }),
    ];
}

/** Require a valid parent to move an object under. */
async function requireDestination(call: Call, nesting: NestedDefinition): Promise<void> {
    // require a parent unless optional
    const { object } = call;
    const parent = call.parent();
    if (parent === undefined && nesting.optional !== true) {
        throw new ServiceError("BAD_REQUEST", { message: `${object.name} needs a parent` });
    }

    // require a receiving parent outside the subtree
    if (parent !== undefined) {
        if (!call.isPredicted) {
            await call.requireReceiving(parent);
        }
        await requireOutside(call, parent.id);
    }
}

/** Refuse moving a node of a tree into its own subtree. */
async function requireOutside(call: Call, parentId: string): Promise<void> {
    // walk only trees of the object's own type
    if (!call.object.tree) {
        return;
    }

    // check the ancestor index on the server
    const moved = call.requireId();
    if (!call.isPredicted) {
        const ancestors = call.object.tree.ancestors;
        const [below] = await call.database
            .select({ depth: ancestors.depth })
            .from(ancestors)
            .where(
                and(
                    eq(ancestors.scope, call.scope),
                    eq(ancestors.ancestor, moved),
                    eq(ancestors.descendant, parentId),
                ),
            );
        if (below !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `${call.object.name} cannot move into its own subtree`,
            });
        }

        return;
    }

    // climb from the new parent on a client
    const table = call.object.table;
    const visited = new Set<string>();
    for (let node: string | null = parentId; node !== null && !visited.has(node);) {
        if (node === moved) {
            throw new ServiceError("CONFLICT", {
                message: `${call.object.name} cannot move into its own subtree`,
            });
        }
        visited.add(node);
        const [row] = await call.database
            .select({ parentId: table[TABLE].column("parentId") })
            .from(table)
            .where(eq(table[TABLE].column("id"), node));
        node = PARENT.parse(row?.parentId ?? null);
    }
}

/** The procedure moves derive. */
export type NestedProcedures<Object extends ObjectType> = {
    move: Procedure<
        schema.Object<
            TargetShape<Object> & ReplayShape<Object> & RevisionField & DestinationShape<Object>
        >,
        RowSchema<Object>
    >;
};
