import { ObjectReference } from "@destack/sync";
import type * as sync from "@destack/sync";
import { Duration, type Identifier, present, schema } from "@destack/schema";
import { boolean, type ColumnBuilder, identifier, index, Table, TABLE, text } from "@destack/db";
import type { PackageId } from "@destack/package";
import { t } from "@destack/locale";
import { ObjectError } from "../error/error.ts";
import type { MethodContext } from "../method/context.ts";
import type { Execution } from "../method/execution.ts";
import { defineMethod, Method } from "../method/method.ts";
import {
    type DestinationFields,
    type Procedure,
    type ReplayFields,
    RevisionFields,
    type RowSchema,
    type TargetFields,
} from "../method/procedure.ts";
import { ObjectRecord } from "../method/record.ts";
import { Step } from "../method/step.ts";
import type { ObjectType } from "../object/object.ts";
import { idColumn } from "../field/field.ts";
import type { FieldColumn } from "../field/field.ts";
import type { TraitBehavior } from "./trait.ts";
import { callableTrait } from "./trait.ts";

/** The parent a tree node refers to, null for a root. */
export const PARENT = schema.string().nullable();

/** A parent of any type, in the scope of the object referring to it. */
export const ParentReference = ObjectReference.omit({ scope: true });
/** A parent of any type, in the scope of the object referring to it. */
export type ParentReference = schema.Infer<typeof ParentReference>;

/** The options of nesting under a parent. */
export type NestedDefinition = {
    /** The parent type, "self" for a tree, or "any" for attachments. */
    readonly in: ObjectType | "self" | "any";
    /** Whether an object may have no parent. */
    readonly optional?: boolean;
    /** Delete children with their parent, trash them with it, or refuse deleting it. */
    readonly delete?: "cascade" | "trash" | "restrict";
    /** The parent permission that adds an object under it, by creating or moving it there. */
    readonly add: string;
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
                      Parent["in"] extends "self"
                          ? Identifier<Name>
                          : Parent extends { readonly id: infer Id extends string }
                            ? Id
                            : Identifier<Parent["in"]>,
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
const nestedTrait: TraitBehavior<NestedDefinition> = {
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

        // reference the owner object, or the table itself for trees
        const owner = options.in === "self" ? undefined : options.in;
        const column = identifier("parent_id", owner?.identity ?? object.identity).references(
            () => idColumn(owner?.table ?? object.table()),
            {
                // cascade a purge of the parent to its children, trashed with it or not, and restrict otherwise
                onDelete: options.delete === "restrict" ? "no action" : "cascade",
            },
        );

        // mark the children their parent's deletion moved to the trash, which its restore brings back
        const trashed =
            options.delete === "trash"
                ? {
                      isTrashedWithParent: boolean("is_trashed_with_parent")
                          .notNull()
                          .default(false),
                  }
                : {};

        return { parentId: options.optional === true ? column : column.notNull(), ...trashed };
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
        options.move === undefined ? {} : { move: Nested.move(options.move, options) },
    validate: (options, object) => {
        // require ephemeral objects to attach to any type
        if (object.storage === "ephemeral" && options.in !== "any") {
            throw new TypeError(
                `ephemeral object ${object.name} must attach to a parent of any type`,
            );
        }

        // require trees to have roots
        if (options.in === "self" && options.optional !== true) {
            throw new TypeError(`${object.name} is a tree, so its parent must be optional`);
        }

        // require a trash on both a child trashed with its parent and the parent
        const parent = options.in === "self" ? object : options.in;
        if (
            options.delete === "trash" &&
            (parent === "any" || !parent.defersDeletion() || !object.defersDeletion())
        ) {
            throw new TypeError(
                `object ${object.name} moves to the trash with its parent, so both must have a trash`,
            );
        }

        // require a child trashed with its parent to stay recoverable as long as the parent
        const child = object.lifecycle.recoverable;
        const held = parent === "any" ? undefined : parent.lifecycle.recoverable;
        if (options.delete === "trash" && child !== undefined && held !== undefined) {
            if (Duration.milliseconds(child.within) < Duration.milliseconds(held.within)) {
                throw new TypeError(
                    `object ${object.name} moves to the trash with its parent, so it must stay recoverable as long as its parent`,
                );
            }
        }

        // require the receive permission on the parent type
        if (parent !== "any" && !parent.permissions.includes(options.add)) {
            throw new TypeError(
                `object ${object.name} has parents without permission '${options.add}'`,
            );
        }
    },
};

/** Nest objects under a parent, or under an object of their type in a tree. */
export const nested = callableTrait("nested", nestedTrait);

/** The procedure moves derive. */
export type NestedProcedures<Object extends ObjectType> = {
    move: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> &
                typeof RevisionFields &
                DestinationFields<Object>
        >,
        RowSchema<Object>
    >;
};

/** The nested methods' handlers: moving an object between parents. */
export const Nested = {
    /** Move objects to another parent refusing cycles. */
    move<const Permission extends string>(
        permission: Permission,
        nesting: NestedDefinition,
    ): Method<{ kind: "move"; permission: Permission; mutates: true }> {
        return defineMethod<{ kind: "move"; permission: Permission; mutates: true }>({
            title: t`Move`,
            kind: "move",
            permission,
            mutates: true,
            target: "object",
            result: "object",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/move" },
                input: schemas.target.extend({
                    ...schemas.replay,
                    ...RevisionFields,
                    ...schemas.destination,
                }),
                output: schemas.row,
            }),
            handler: (context) => {
                // write the new parent columns over the record
                const recorded = context;

                return recorded.update(
                    Table.values(recorded.object.table, recorded.parentColumns()),
                );
            },
            inverse: (step) => Nested.moveBack(step, nesting),
            async execute(context, execution) {
                // require a valid destination before moving
                await Nested.requireDestination(context, execution, nesting);

                return Method.handle(this, context, execution);
            },
        });
    },

    /** Move an object back under its former parent. */
    moveBack(step: Step, nesting: NestedDefinition): readonly sync.Call[] | undefined {
        // find the former parent, of one type or any
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
    },

    /** Require a valid parent to move an object under. */
    async requireDestination(
        context: MethodContext,
        execution: Execution,
        nesting: NestedDefinition,
    ): Promise<void> {
        // require a parent unless optional
        const { object } = context;
        const parent = context.parent();
        if (parent === undefined && nesting.optional !== true) {
            throw ObjectError.of("PARENT_REQUIRED", `parent of ${object.name} is required`);
        }

        // require a receiving parent outside the subtree
        if (parent !== undefined) {
            if (execution.kind === "store") {
                await ObjectRecord.requireReceiving(context, execution, parent);
            }
            await Nested.requireOutside(context, parent.id);
        }
    },

    /** Refuse moving a node of a tree into its subtree. */
    async requireOutside(context: MethodContext, parentId: string): Promise<void> {
        // walk only trees of the object's type
        if (!context.object.tree) {
            return;
        }

        // climb from the new parent through the call's view, which has the call's moves
        const moved = context.id;
        const reader = context.read(context.object.table);
        const visited = new Set<string>();
        for (let node: string | null = parentId; node !== null && !visited.has(node);) {
            if (node === moved) {
                throw ObjectError.of(
                    "MOVE_INVALID",
                    `move of ${context.object.name} is invalid: it must stay outside its own subtree`,
                );
            }
            visited.add(node);
            const row = await reader.findFirst({ where: { id: node } });
            node = PARENT.parse(row?.["parentId"] ?? null);
        }
    },
};
