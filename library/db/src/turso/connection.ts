import * as turso from "@tursodatabase/database";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { TableRelations } from "../schema/relation.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import type { Table } from "../table/table.ts";
import { Database } from "./database.ts";

/** Open an embedded Turso database and bind its declared tables. */
export async function connect<Relations extends Record<string, TableRelations> = {}>(
    connection: string | turso.Database,
    schema: DatabaseSchema<Record<string, Table>, Relations> | readonly Table[] = [],
    options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
): Promise<Database<Relations, turso.Database>> {
    // enable generated columns for connections created by Destack
    const client =
        typeof connection === "string"
            ? await turso.connect(connection, { experimental: ["generated_columns"] })
            : connection;

    try {
        // enable durable commits and foreign keys on connections opened here
        if (typeof connection === "string") {
            await client.exec("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;");
        }

        return new Database(client, schema, options);
    } catch (error) {
        // release only connections allocated by this call
        if (typeof connection === "string") {
            try {
                await client.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed");
            }
        }

        throw error;
    }
}
