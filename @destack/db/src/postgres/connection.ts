import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { type PostgresConnectOptions, PostgresDatabase } from "./database.ts";
import { PostgresPool } from "./pool.ts";
import type { Model } from "../query/model.ts";

/** Connect to a PostgreSQL URL with a pool of its own, or to a pool its opener closes. */
export async function connectPostgres<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    connection: string | PostgresPool,
    tables: declaration.Database<Models> | readonly Table[],
    options: PostgresConnectOptions,
): Promise<PostgresDatabase<Models>> {
    // bind the tables to a caller's pool, which the caller closes
    if (typeof connection !== "string") {
        return new PostgresDatabase(connection, tables, { ...options, poolCloser: "caller" });
    }

    // open a pool of the database's own for a URL, closing it should binding fail
    const pool = new PostgresPool(connection, options.environment);
    try {
        return new PostgresDatabase(pool, tables, { ...options, poolCloser: "database" });
    } catch (error) {
        try {
            await pool.end();
        } catch (cleanup) {
            throw new AggregateError([error, cleanup], "database binding and closure failed", {
                cause: cleanup,
            });
        }

        throw error;
    }
}
