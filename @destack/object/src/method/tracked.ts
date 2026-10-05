import { type Permission } from "@destack/access";
import { present } from "@destack/schema";
import { TABLE, type Row, LogPosition } from "@destack/db";
import type { Call } from "./call.ts";
import type { ObjectTable } from "../object/table.ts";
import { defineMethod, type Method } from "./method.ts";
import { Step } from "./step.ts";

import type {} from "../trait/tracked.ts";
/** Revert an object's written fields to a position. */
export function revertMethod(permission: string): Method {
    return defineMethod<{ kind: "revert" }>({
        kind: "revert",
        permission,
        mutates: true,
        isPredicted: false,
        target: true,
        result: "object",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/revert" },
            input: shapes.target.extend({ ...shapes.replay, at: LogPosition }),
            output: shapes.row,
        }),
        handler: async (call: Call<ObjectTable>) => {
            // write back the changed fields the history keeps, sensitive ones being unlogged
            const row = await earlier(
                call,
                present(call.object.reading, `the read permission of ${call.object.name}`),
            );
            const target: Row = call.requireTarget();
            const kept = call.object.table[TABLE].encode(row);
            const current = call.object.table[TABLE].encode(target);
            const changes = Object.fromEntries(
                call.object.written.flatMap((name) => {
                    const value = row[name];

                    return value === undefined || Step.same(kept[name], current[name])
                        ? []
                        : [[name, value] as const];
                }),
            );

            return Object.keys(changes).length === 0
                ? target
                : call.update(call.object.table[TABLE].values(changes));
        },
        inverse: (step) => {
            // restore reverted fields unchanged since
            const current = step.current;
            if (current === undefined) {
                return undefined;
            }
            const restored = Object.fromEntries(
                step.object.written
                    .filter((name) => !Step.same(step.before?.[name], step.after?.[name]))
                    .filter((name) => Step.same(current[name], step.after?.[name]))
                    .map((name) => [name, step.before?.[name] ?? null]),
            );
            const update = Step.call(step, "update", {
                ...Step.target(step),
                ...restored,
            });

            return Object.keys(restored).length === 0 || update === undefined
                ? undefined
                : [update];
        },
    });
}

/** Read the call's object at the input's position, where the caller may read it then and now. */
function earlier(call: Call, permission: Permission): Promise<Row> {
    const snapshot = call.database.log.at(LogPosition.parse(call.input["at"]));

    return call.requireAuthorization().readIn(call, call.requireId(), snapshot, permission);
}
