import { Table } from "@destack/db";
import { t } from "@destack/locale";
import { type JsonObject, schema } from "@destack/schema";
import type * as sync from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import type { MethodContext } from "../method/context.ts";
import type { Execution } from "../method/execution.ts";
import { defineMethod, Method } from "../method/method.ts";
import {
    type DestinationFields,
    type PlacementFields,
    type Procedure,
    type ReplayFields,
    RevisionFields,
    type RowSchema,
    type TargetFields,
    type WrittenFields,
} from "../method/procedure.ts";
import { Step } from "../method/step.ts";
import type { Field } from "../field/field.ts";
import { FractionalIndex } from "../field/fractional-index.ts";
import type { ObjectDefinition, ObjectType, UpdatedField } from "../object/object.ts";
import { type NestedDefinition, PARENT, ParentReference } from "./nested.ts";
import { Orderable, type Ordering } from "./orderable.ts";
import type { TraitBehavior } from "./trait.ts";

/** The sibling a moved object goes after, null for the first place. */
const ANCHOR = schema.string().nullable();

/** What a type's objects move between: parents, values of a field, and places among siblings. */
export interface Movement {
    /** The permission moving an object. */
    readonly permission: string;
    /** The nesting whose parents objects move between, absent for objects keeping theirs. */
    readonly nesting: NestedDefinition | undefined;
    /** The ordering objects move within, absent for a type ordering none. */
    readonly ordering: Ordering | undefined;
    /** The fields of the ordering a move also writes, such as a board's column. */
    readonly fields: readonly string[];
}

/** The permission a type's nesting or ordering moves its objects by. */
type MovePermission<Nested, Ordered> = Nested extends { readonly move: infer Permission }
    ? Permission
    : Ordered extends { readonly move: infer Permission }
      ? Permission
      : undefined;

/** Whether a move takes a parent: always, only to change it, or never. */
type MoveParentOf<Nested, Ordered> = Nested extends { readonly move: string }
    ? Ordered extends undefined
        ? "required"
        : "optional"
    : "none";

/** The field of an ordering that callers write. */
type MoveField<Fields extends Readonly<Record<string, Field>>, Ordered> = Ordered extends {
    readonly within: infer Within extends string;
}
    ? Extract<Within | `${Within}Id`, UpdatedField<Fields>>
    : never;

/** The method moving objects between parents and among siblings. */
export type MoveMethodMap<Fields extends Readonly<Record<string, Field>>, Nested, Ordered> =
    MovePermission<Nested, Ordered> extends infer Permission extends string
        ? {
              readonly move: Method<{
                  kind: "move";
                  permission: Permission;
                  mutates: true;
                  fields: MoveField<Fields, Ordered>;
                  parent: MoveParentOf<Nested, Ordered>;
              }>;
          }
        : {};

/** The procedure moves derive. */
export type MoveProcedures<Object extends ObjectType, Declaration extends Method = Method> = {
    move: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> &
                typeof RevisionFields &
                WrittenFields<Object, Declaration, true> &
                DestinationFields<Object, NonNullable<Declaration["parent"]>> &
                PlacementFields<Object, NonNullable<Declaration["parent"]>>
        >,
        RowSchema<Object>
    >;
};

/** Objects moving between parents and among siblings, by the permission of one of the two. */
export const movable: TraitBehavior<Movement> = {
    options: (definition) => Move.of(definition),
    columns: () => ({}),
    constraints: () => [],
    methods: (movement) => ({ move: Move.method(movement) }),
};

/** The method moving objects between parents and among siblings. */
export const Move = {
    /** Read what a definition's objects move between, absent for objects that stay in place. */
    of(
        definition: Pick<ObjectDefinition, "name" | "nested" | "orderable" | "fields">,
    ): Movement | undefined {
        // take the permission from the nesting or from the ordering
        const { nested, orderable } = definition;
        const ordered = orderable === undefined || orderable === true ? undefined : orderable.move;
        if (nested?.move !== undefined && ordered !== undefined) {
            throw new TypeError(
                `object '${definition.name}' moves by its nesting, so its ordering cannot declare move`,
            );
        }
        const permission = nested?.move ?? ordered;
        if (permission === undefined) {
            return undefined;
        }

        // write the ordering's field where callers write it
        const ordering = Orderable.of(definition);
        const within = orderable === undefined || orderable === true ? undefined : orderable.within;
        const { fields: declared = {} } = definition;
        const field =
            within === undefined ? undefined : (declared[within] ?? declared[`${within}Id`]);
        const fields = field?.written === true ? (ordering?.within ?? []) : [];

        return {
            permission,
            nesting: nested?.move === undefined ? undefined : nested,
            ordering,
            fields,
        };
    },

    /** Declare the method moving an object, undone by moving it back. */
    method(movement: Movement): Method {
        const { nesting, ordering, fields } = movement;

        return defineMethod<{ kind: "move"; permission: string; mutates: true }>({
            title: t`Move`,
            kind: "move",
            permission: movement.permission,
            mutates: true,
            fields,
            parent:
                nesting === undefined ? "none" : ordering === undefined ? "required" : "optional",
            target: "object",
            result: "object",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/move" },
                input: schemas.target.extend({
                    ...schemas.replay,
                    ...RevisionFields,
                    ...schemas.written(fields, true),
                    ...(nesting === undefined ? {} : schemas.destination),
                    ...(ordering === undefined ? {} : schemas.placement),
                }),
                output: schemas.row,
            }),
            handler: async (context) => {
                // write the new parent and field, and the place among the siblings there
                const { table } = context.object;
                const input: JsonObject = context.input;
                const written = Move.written(input, movement);
                const changes = {
                    ...Table.decode(table, written),
                    ...(Move.takesParent(input) ? context.parentColumns() : {}),
                };
                const position =
                    ordering === undefined ? {} : { position: await Move.place(context, changes) };

                return context.update(Table.values(table, { ...changes, ...position }));
            },
            inverse: (step) => Move.back(step, movement),
            compose: (previous, next) => Move.compose(previous, next),
            async execute(context, execution) {
                // refuse a field the caller may not write, and a parent that cannot receive the object
                const input: JsonObject = context.input;
                const written = Object.keys(Move.written(input, movement));
                execution.store.record.requireUnmanagedFields(context, execution, written);
                if (!execution.isPredicted) {
                    await execution.authorization.requireWritable(context, context.id, execution);
                }
                if (nesting !== undefined && Move.takesParent(input)) {
                    await Move.requireDestination(context, execution, nesting);
                }

                return Method.handle(this, context, execution);
            },
        });
    },

    /** Read the values of the ordering's fields a move's input sets. */
    written(input: JsonObject, movement: Movement): JsonObject {
        return Object.fromEntries(
            movement.fields.flatMap((name) => {
                const value = input[name];

                return value === undefined ? [] : [[name, value]];
            }),
        );
    },

    /** Decide whether a move's input has a parent, none included. */
    takesParent(input: JsonObject): boolean {
        return input["parentId"] !== undefined || input["parent"] !== undefined;
    },

    /** Place a moved object where its input says: at a position, after a sibling, first or last. */
    async place(
        context: MethodContext<{ readonly table: Table }, unknown>,
        changes: Readonly<Record<string, unknown>>,
    ): Promise<string> {
        // take an exact position as given
        const { object } = context;
        const input: JsonObject = context.input;
        const { after, position } = input;
        if (after !== undefined && position !== undefined) {
            throw ObjectError.of(
                "MOVE_INVALID",
                `move of ${object.name} is invalid: it takes either after or position`,
            );
        } else if (position !== undefined) {
            return FractionalIndex.parse(position);
        }

        return await Orderable.place(
            object,
            { ...context.target, ...changes },
            after === undefined ? "last" : { after: ANCHOR.parse(after) },
            context,
        );
    },

    /** Move an object back to its former place, unless its parent or field changed since. */
    back(step: Step, movement: Movement): readonly sync.Call[] | undefined {
        // leave an object alone that was moved elsewhere since
        const { before, after, current } = step;
        const moved = [...movement.fields, ...(movement.nesting === undefined ? [] : ["parentId"])];
        const isChanged =
            current !== undefined &&
            !moved.every((name) => Step.same(current[name], after?.[name]));
        if (before === undefined || isChanged) {
            return undefined;
        }

        // find the former parent, of one type or any
        const destination =
            movement.nesting === undefined ? {} : Move.formerParent(before, movement.nesting);

        // find the former field values and place
        const former = Object.fromEntries(
            [...movement.fields, ...(movement.ordering === undefined ? [] : ["position"])].map(
                (name) => [name, before[name] ?? null],
            ),
        );

        return [Step.record(step, step.name, { ...Step.target(step), ...destination, ...former })];
    },

    /** Read the parent a row had, as a move takes it. */
    formerParent(before: JsonObject, nesting: NestedDefinition): JsonObject {
        // read the parent's identifier, or the reference to a parent of any type
        const parentId = before["parentId"] ?? null;
        if (nesting.in !== "any") {
            return { parentId };
        } else if (parentId === null) {
            return { parent: null };
        } else {
            return {
                parent: ParentReference.parse({
                    packageId: before["parentPackageId"],
                    type: before["parentType"],
                    id: parentId,
                }),
            };
        }
    },

    /** Compose two moves of one object into one, placing it where the later one does. */
    compose(previous: Step, next: Step): Step | undefined {
        const { after: _after, position: _position, ...input } = previous.input;

        return Step.compose({ ...previous, input }, next);
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
            if (!execution.isPredicted) {
                await execution.store.record.requireReceiving(context, execution, parent);
            }
            await Move.requireOutside(context, parent.id);
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
