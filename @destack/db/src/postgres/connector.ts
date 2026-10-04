import postgres from "postgres";
import type { ResourceBinding } from "@destack/resource";
import type { Database, DatabaseConnector } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import type { Model } from "../query/model.ts";
import { connect, reportNotice } from "./connection.ts";
import type { PostgresDatabase } from "./database.ts";

/** The connections one pool opens, within the six a Worker keeps open at once, as Hyperdrive's guide sets it. */
const POOL_CONNECTIONS = 5;

/** Open a PostgreSQL database by URL as its instance's sole writer, refusing one lacking its declaration's tables. */
export const postgresConnector = {
    code: "postgresql",
    async connect<Models extends Readonly<Record<string, Model>>>(
        binding: Pick<ResourceBinding, "reference">,
        declaration: Database<Models>,
    ): Promise<PostgresDatabase<Models>> {
        // open the binding's pool as the sole writer
        const client = postgres(binding.reference, {
            max: POOL_CONNECTIONS,
            onnotice: reportNotice,
        });
        const connection = await connect(client, declaration, "sole");

        // refuse a database lacking its tables
        const unapplied = await declaration.check(connection);
        if (unapplied.length > 0) {
            await connection.close();
            throw new DatabaseError(
                "NOT_APPLIED",
                `database ${declaration.name} has not applied ${unapplied.join(", ")}`,
            );
        }

        return connection;
    },
} satisfies DatabaseConnector;
