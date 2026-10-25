import type * as sync from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { Table } from "@destack/db";

import type { MethodContext } from "./context.ts";
import { Step } from "./step.ts";
import { defineMethod, Method } from "./method.ts";
import { RevisionFields } from "./procedure.ts";
import type { Execution } from "./execution.ts";
import { RecordMethod } from "./record.ts";

import { type NestedDefinition, PARENT, ParentReference } from "../trait/nested.ts";

import type {} from "@destack/package/import-meta";

/** The nested methods' handlers: moving an object between parents. */
export const NestedMethod = {
    /** Move objects to another parent refusing cycles. */
    move<const Permission extends string>(
        permission: Permission,
        nesting: NestedDefinition,
    ): Method<{ kind: "move"; permission: Permission; mutates: true }> {
        return defineMethod<{ kind: "move"; permission: Permission; mutates: true }>({
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
            inverse: (step) => NestedMethod.moveBack(step, nesting),
            async execute(context, execution) {
                // require a valid destination before moving
                await NestedMethod.requireDestination(context, execution, nesting);

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
            throw ObjectError.of("PARENT_MISSING", `${object.name} needs a parent`);
        }

        // require a receiving parent outside the subtree
        if (parent !== undefined) {
            if (execution.kind === "store") {
                await RecordMethod.requireReceiving(context, execution, parent);
            }
            await NestedMethod.requireOutside(context, parent.id);
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
                    "MOVE_INTO_SUBTREE",
                    `${context.object.name} cannot move into its subtree`,
                );
            }
            visited.add(node);
            const row = await reader.findFirst({ where: { id: node } });
            node = PARENT.parse(row?.["parentId"] ?? null);
        }
    },
};
