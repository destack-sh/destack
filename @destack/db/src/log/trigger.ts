import type { Triggers } from "../migration/trigger.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { LogDialect } from "./schema.ts";
import { sqliteLog } from "../sqlite/log.ts";
import { postgresLog } from "../postgres/log.ts";

/** The log of each dialect. */
const LOGS: Readonly<Record<Dialect, LogDialect>> = { sqlite: sqliteLog, postgresql: postgresLog };

/** Create the log once at a first epoch within its namespace, scoping unscoped tables' rows. */
export function createLog(
    dialect: Dialect,
    epoch: string,
    scope?: string,
    namespace?: string,
): readonly string[] {
    return LOGS[dialect].create(epoch, scope, namespace);
}

/** The change triggers of a logged table on its relation, recording changes under its SQL name. */
export const logTriggers: Triggers = {
    install: (state, dialect, namespace) =>
        state.log ? LOGS[dialect].install(state.table.name, state.log, namespace) : [],
    remove: (state, dialect) => (state.log ? LOGS[dialect].remove(state.table.name) : []),
};
