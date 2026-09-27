import type { DatabaseConnection } from "../database/connection.ts";
import { DatabaseState, readState, readTables, type TableState } from "./state.ts";
import type { ResourceState } from "@destack/package/declare";
import { type TablePlan, planTables } from "./plan.ts";
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
