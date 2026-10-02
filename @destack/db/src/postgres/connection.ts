import postgres from "postgres";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { PostgresDatabase } from "./database.ts";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The database log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** Connect to a PostgreSQL pool or URL. */
export async function connect(
    connection: string | postgres.Sql,
    tables: declaration.Database | readonly Table[] = [],
): Promise<PostgresDatabase> {
    const client =
        typeof connection === "string"
            ? postgres(connection, { onnotice: reportNotice })
            : connection;

    try {
        return new PostgresDatabase(client, tables);
    } catch (error) {
        // release only pools this call created
        if (typeof connection === "string") {
            try {
                await client.end();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "database binding and closure failed", {
                    cause: cleanup,
                });
            }
        }

        throw error;
    }
}

/** Report a server warning, dropping informational notices such as skipped drops. */
export function reportNotice(notice: postgres.Notice): void {
    if (notice["severity"] === "WARNING") {
        log.warn("postgresql.warning", { message: String(notice["message"]) });
    }
}
