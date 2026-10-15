import { type Permission } from "@destack/access";
import { Duration, present, type schema } from "@destack/schema";
import { LogPosition, TABLE, type Row, Table } from "@destack/db";
import type { ActivityTable } from "../object/history.ts";
import { defineMethod, type Method } from "../method/method.ts";
import type { Call } from "../method/call.ts";
import { Step } from "../method/step.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { Procedure, ReplayShape, RowSchema, TargetShape } from "../method/procedure.ts";
import type { Gated, Trait } from "./trait.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

/** The options of history tracking. */
export interface TrackedDefinition<Permissions extends string = string> extends Gated<Permissions> {
    /** The activity type among the object's attachments. */
    readonly activity: ObjectOf<{ table: ActivityTable }>;
    /** How long one activity continues after its last change, 10 minutes by default. */
    readonly session?: Duration;
}

/** The methods history derives. */
export type TrackedMethodMap<History> = History extends TrackedDefinition
    ? {
          readonly revert: Method<{ kind: "revert"; permission: History["by"]; mutates: true }>;
      }
    : {};

/** The procedures history derives. */
export type TrackedProcedures<Object extends ObjectType> = {
    revert: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & PositionField>,
        RowSchema<Object>
    >;
};

/** The input field with the log position an object is read or reverted at. */
type PositionField = { readonly at: typeof LogPosition };

/** Objects keeping their history, grouped into activities. */
export const tracked: Trait<TrackedDefinition> & {
    /** Revert an object's written fields to a position. */
    readonly revert: (permission: string) => Method;
    /** Read the call's object at the input's position, readable by the caller then and now. */
    readonly earlier: (
        call: Call,
        execution: StoreExecution,
        permission: Permission,
    ) => Promise<Row>;
} = {
    key: "tracked",
    storages: ["durable"],
    options: (definition) => definition.tracked,
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ({ revert: tracked.revert(options.by) }),
    validate: (options, object) => {
        // require the activity attachment and a valid session
        if (!object.attachments.some((attachment) => attachment.object === options.activity)) {
            throw new TypeError(
                `object ${object.name} keeps history but has no ${options.activity.plural} attachment`,
            );
        }
        if (options.session !== undefined) {
            Duration.require(options.session, `history session of ${object.name}`);
        }
    },
    revert(permission: string): Method {
        return defineMethod<{ kind: "revert" }>({
            kind: "revert",
            permission,
            mutates: true,
            isPredicted: false,
            target: "object",
            result: "object",
            procedure: (_name, shapes) => ({
                route: { method: "POST", path: "/{id}/revert" },
                input: shapes.target.extend({ ...shapes.replay, at: LogPosition }),
                output: shapes.row,
            }),
            handler: async (given, execution) => {
                // write back the changed fields the history keeps, sensitive ones being unlogged
                const call = given;
                const row = await tracked.earlier(
                    call,
                    Execution.store(execution, call),
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
                    : call.update(Table.values(call.object.table, changes));
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
    },
    earlier(call: Call, execution: StoreExecution, permission: Permission): Promise<Row> {
        const { database } = execution.transaction;
        const snapshot = database.log.at(LogPosition.parse(call.input["at"]));

        return execution.authorization.readIn(call, call.requireId(), snapshot, permission);
    },
};
