import postgres from "postgres";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { type PostgresConnectOptions, PostgresDatabase } from "./database.ts";
import { TelemetryScope } from "@destack/telemetry";
import type { Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import type { Model } from "../query/model.ts";

/** The connections of one pool: five, within the six a Worker opens per Hyperdrive. */
const POOL_CONNECTIONS = 5;

/** Connect to a PostgreSQL pool or URL, as one of several writers or the sole one. */
export async function connectPostgres<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    connection: string | postgres.Sql,
    tables: declaration.Database<Models> | readonly Table[],
    options: PostgresConnectOptions,
): Promise<PostgresDatabase<Models>> {
    const client =
        typeof connection === "string"
            ? postgres(connection, {
                  max: POOL_CONNECTIONS,
                  onnotice: reportNotices(options.environment.telemetry),
              })
            : connection;

    try {
        return new PostgresDatabase(client, tables, options);
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

/** Make the reporter of a server's warnings on a telemetry, dropping informational notices. */
export function reportNotices(
    telemetry: Environment["telemetry"],
): (notice: postgres.Notice) => void {
    const { log } = TelemetryScope.of(import.meta.destack.package, telemetry);

    return (notice) => {
        if (notice["severity"] === "WARNING") {
            log.warn("postgresql.warning", { message: String(notice["message"]) });
        }
    };
}
