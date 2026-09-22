import * as turso from "@tursodatabase/serverless";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { TableRelations } from "../../schema/relation.ts";
import type { DatabaseSchema } from "../../schema/schema.ts";
import type { Table } from "../../table/table.ts";
import { Database } from "../database.ts";

/** Open a hosted Turso database and bind its declared tables. */
export async function connect<Relations extends Record<string, TableRelations> = {}>(
    connection: turso.Config | turso.Connection,
    schema: DatabaseSchema<Record<string, Table>, Relations> | readonly Table[] = [],
    options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
): Promise<Database<Relations, turso.Connection>> {
    const client = connection instanceof turso.Connection ? connection : turso.connect(connection);

    try {
        return new Database(client, schema, options);
    } catch (error) {
        // release only connections allocated by this call
        if (!(connection instanceof turso.Connection)) {
            try {
                await client.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed");
            }
        }

        throw error;
    }
}
