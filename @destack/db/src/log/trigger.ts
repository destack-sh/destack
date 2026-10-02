import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { LogDialect } from "./schema.ts";
import { sqliteLog } from "./sqlite.ts";
import { postgresLog } from "./postgres.ts";

/** The log of each dialect. */
const LOGS: Readonly<Record<Dialect, LogDialect>> = { sqlite: sqliteLog, postgresql: postgresLog };

/** Create the log once per database at a first epoch, with the scope of the rows in tables without a scope column. */
export function createLog(dialect: Dialect, epoch: string, scope?: string): readonly string[] {
    return LOGS[dialect].create(epoch, scope);
}

/** The change triggers of a logged table. */
export const logTriggers: Triggers = {
    install: (state, dialect) => (state.log ? LOGS[dialect].install(state.log) : []),
    remove: (state, dialect) => (state.log ? LOGS[dialect].remove(state.log) : []),
};
