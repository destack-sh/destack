import postgres from "postgres";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { PostgresDatabase } from "./database.ts";

/** Connect to a PostgreSQL pool or URL. */
export async function connect(
    connection: string | postgres.Sql,
    tables: declaration.Database | readonly Table[] = [],
    options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
): Promise<PostgresDatabase> {
    const client = typeof connection === "string" ? postgres(connection) : connection;

    try {
        return new PostgresDatabase(client, tables, options);
    } catch (error) {
        // release only pools this call created
        if (typeof connection === "string") {
            try {
                await client.end();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed");
            }
        }

        throw error;
    }
}
