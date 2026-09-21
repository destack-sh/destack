import postgres from "postgres";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import type { TableRelations } from "../schema/relation.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import type { Table } from "../table/table.ts";
import { Database } from "./database.ts";

/** Connect PostgreSQL queries to an existing pool or connection URL. */
export async function connect<Relations extends Record<string, TableRelations> = {}>(
    connection: string | postgres.Sql,
    schema: DatabaseSchema<Record<string, Table>, Relations> | readonly Table[] = [],
    options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
): Promise<Database<Relations>> {
    const client = typeof connection === "string" ? postgres(connection) : connection;

    try {
        return new Database(client, schema, options);
    } catch (error) {
        // release only pools allocated by this call
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
