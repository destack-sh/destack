import type * as sync from "@destack/sync";
import { and, eq, TABLE } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import type { Call } from "./call.ts";
import { Step } from "./step.ts";
import { defineMethod, type Method } from "./method.ts";
import { RevisionShape } from "./procedure.ts";
import type { ObjectTable } from "../object/table.ts";

import { type NestedDefinition, PARENT, ParentReference } from "../trait/nested.ts";
/** Move objects to another parent and keep trees acyclic. */
export function move<const Permission extends string>(
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
