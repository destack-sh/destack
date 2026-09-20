import type { DatabaseConnection } from "../database/connection.ts";
import { type DatabaseSchema, orderSchemas } from "../schema/schema.ts";
import { migrate } from "./migrate.ts";

/** Prepare the declared schemas and close the connection if preparation fails. */
export async function prepare<
    Database extends DatabaseConnection & {
        /** Release the physical connection. */
        close(): Promise<void>;
    },
>(database: Database, schemas: readonly DatabaseSchema[]): Promise<Database> {
    // apply histories in dependency order before returning the connection
    try {
        for (const schema of orderSchemas(schemas)) await migrate(database, schema);

        return database;
    } catch (error) {
        // preserve both failures when releasing an unusable connection fails
        try {
            await database.close();
        } catch (cleanup) {
            throw new AggregateError([error, cleanup], "Database preparation and closure failed.");
        }

        throw error;
    }
}
