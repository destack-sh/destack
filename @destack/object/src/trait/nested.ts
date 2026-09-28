import { defineSchema, type Identifier, schema } from "@destack/schema";
import { ObjectReference } from "@destack/access";
import {
    type Column,
    type ColumnBuilder,
    and,
    eq,
    identifier,
    index,
    TABLE,
    type Table,
    text,
} from "@destack/db";
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
import type { IdentifierOf } from "../field/field.ts";
import type { Trait } from "./trait.ts";

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
export type ParentBuilderMap<Name extends string, Parent> = Parent extends NestedDefinition
    ? Parent["in"] extends "any"
        ? {
              parentPackageId: ColumnBuilder<PackageId, IsRequired<Parent>, false>;
              parentType: ColumnBuilder<string, IsRequired<Parent>, false>;
              parentId: ColumnBuilder<string, IsRequired<Parent>, false>;
          }
        : {
              parentId: ColumnBuilder<
                  Parent["in"] extends ObjectType ? IdentifierOf<Parent["in"]> : Identifier<Name>,
                  IsRequired<Parent>,
                  false
              >;
          }
    : {};

/** Whether every object needs a parent. */
type IsRequired<Parent extends NestedDefinition> = Parent["optional"] extends true ? false : true;

/** The method moving objects between parents. */
export type NestedMethodMap<Parent> = Parent extends {
    readonly move: infer Permission extends string;
}
    ? { readonly move: Method<"move", Permission, never, never, true> }
    : {};

/** Objects nested under a parent, or under an object of their type in a tree. */
export const nested: Trait<NestedDefinition> = {
    key: "nested",
    options: (definition) => definition.nested,
    columns: (options, object): Record<string, ColumnBuilder<any, boolean, boolean>> => {
        // name a parent of any type by package, type and identifier
        if (options.in === "any") {
            const columns = {
                parentPackageId: identifier("parent_package_id", "package"),
                parentType: text("parent_type"),
                parentId: text("parent_id"),
            };

            return options.optional
                ? columns
                : {
                      parentPackageId: columns.parentPackageId.notNull(),
                      parentType: columns.parentType.notNull(),
                      parentId: columns.parentId.notNull(),
                  };
        }

        // reference the owning object, or the table itself for trees
        const owner = options.in === "self" ? undefined : options.in;
        const column = identifier("parent_id", owner?.identity ?? object.identity).references(
            () =>
                ((owner?.table ?? object.table()) as Table)[TABLE].columns.id as Column<
                    Identifier<string>
                >,
            { onDelete: options.delete ?? "cascade" },
        );

        return { parentId: options.optional ? column : column.notNull() };
    },
    constraints: (options, table, columns) =>
        options.in === "any"
            ? [
                  index(`${table}_parent`).on(
                      columns.parentPackageId!,
                      columns.parentType!,
                      columns.parentId!,
                  ),
              ]
            : [],
    // index the ancestry of trees
    table: (options) =>
        options.in === "self"
            ? { tree: { id: "id" as never, scope: "scope" as never, parent: "parentId" as never } }
            : {},
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
        options.move === undefined ? {} : { move: move(options.move) },
    validate: (options, object) => {
        // require ephemeral objects to attach to any type
        if (object.storage === "ephemeral" && options.in !== "any") {
            throw new TypeError(
                `ephemeral object ${object.name} attaches to its parent as an attachment of any type`,
            );
        }

        // require trees to have roots
        if (options.in === "self" && !options.optional) {
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

/** Move objects to another parent, keeping trees acyclic. */
function move<const Permission extends string>(
    permission: Permission,
): Method<"move", Permission, never, never, true> {
    return defineMethod<Method<"move", Permission, never, never, true>>({
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
        effect: (call) => call.revise(call.parentColumns()),
        inverse: (step) => {
            // move back under the former parent
            const before = step.before;
            if (before === undefined) {
                return undefined;
            }
            const destination =
                step.object.parent!.object !== "any"
                    ? { parentId: before.parentId ?? null }
                    : {
                          parent:
                              before.parentId === null || before.parentId === undefined
                                  ? null
                                  : {
                                        packageId: before.parentPackageId,
                                        type: before.parentType,
                                        id: before.parentId,
                                    },
                      };

            return [
                Step.record(step, step.name, {
                    ...Step.target(step, step.input.id),
                    ...destination,
                }),
            ];
        },
        async execute(call) {
            // require a parent unless optional
            const { object } = call;
            const parent = call.parent();
            if (parent === undefined && !object.parent!.optional) {
                throw new ServiceError("BAD_REQUEST", { message: `${object.name} needs a parent` });
            }

            // require a receiving parent outside the subtree
            if (parent !== undefined) {
                if (!call.isPredicted) {
                    await call.requireReceiving(parent);
                }
                await requireOutside(call, parent.id);
            }

            return this.effect(call);
        },
    });
}

/** Refuse moving a node of a tree into its own subtree. */
async function requireOutside(call: Call, parentId: string): Promise<void> {
    // walk only trees of the object's own type
    if (!call.object.tree) {
        return;
    }

    // check the ancestor index on the server
    const moved = String(call.target!.id);
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
    const table = call.object.table as Table & Record<string, never>;
    const visited = new Set<string>();
    for (let node: string | null = parentId; node !== null && !visited.has(node);) {
        if (node === moved) {
            throw new ServiceError("CONFLICT", {
                message: `${call.object.name} cannot move into its own subtree`,
            });
        }
        visited.add(node);
        const [row] = (await call.database
            .select({ parentId: table.parentId })
            .from(table)
            .where(eq(table.id, node))) as { parentId: string | null }[];
        node = row?.parentId ?? null;
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
