import postgres from "postgres";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { PostgresDatabase, type PostgresWriteMode } from "./database.ts";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import type { Model } from "../query/model.ts";

/** The database log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** The connections one pool opens: five, within the six a Worker keeps open at once as Hyperdrive's guide sets it, so a development universe holds 24 of a server's default 100. */
const POOL_CONNECTIONS = 5;

/** Connect to a PostgreSQL pool or URL, as one of several writers or the sole one. */
export async function connectPostgres<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    connection: string | postgres.Sql,
    tables: declaration.Database<Models> | readonly Table[] = [],
    writers: PostgresWriteMode = "shared",
): Promise<PostgresDatabase<Models>> {
    const client =
        typeof connection === "string"
            ? postgres(connection, { max: POOL_CONNECTIONS, onnotice: reportNotice })
            : connection;

    try {
        return new PostgresDatabase(client, tables, writers);
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
