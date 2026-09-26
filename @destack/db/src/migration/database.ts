import type { DatabaseConnection } from "../database/connection.ts";
import type { Table } from "../table/table.ts";
import {
    DatabaseState,
    declareState,
    readState,
    readTables,
    type DeclareOptions,
    type TableState,
} from "./state.ts";
import type { ResourceState } from "@destack/package/declare";
import { type TablePlan, planTables } from "./plan.ts";
import { applyPlan } from "./apply.ts";
import { type MergedState, mergeStates } from "./merge.ts";

/** Plan the steps from a connected database's applied state to declared table states. */
export async function planMigration(
    connection: DatabaseConnection,
    declared: readonly TableState[],
    conflicts: MergedState["conflicts"] = [],
): Promise<TablePlan> {
    return planTables({
        applied: await readState(connection),
        existing: await readTables(connection),
        declared,
        conflicts,
        dialect: connection.dialect,
    });
}

/** Plan the union of the desired states of every declaration bound to a connected database. */
export async function planStates(
    connection: DatabaseConnection,
    desired: readonly ResourceState[],
): Promise<TablePlan> {
    // merge each declaration's tables in the connection's dialect
    const dialect = connection.dialect;
    const merged = mergeStates(desired.map((state) => DatabaseState.parse(state).tables[dialect]));

    return await planMigration(connection, merged.declared, merged.conflicts);
}

/** Plan and apply the given tables at once, for replicas and tests that own their database. */
export async function migrate(
    connection: DatabaseConnection,
    tables: readonly Table[],
    options: DeclareOptions = {},
): Promise<TablePlan> {
    // plan and apply the tables in the connection's dialect
    const declared = declareState(tables, connection.dialect, options);
    const plan = await planMigration(connection, declared);
    await applyPlan(connection, plan);

    return plan;
}
