import type { ResourceBinding } from "@destack/resource";
import type { Database, DatabaseConnector } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import type { Model } from "../query/model.ts";
import { connectPostgres } from "./connection.ts";
import type { PostgresDatabase } from "./database.ts";

/** The connection parameter that selects a database's schema. */
const SCHEMA_PARAMETER = "search_path";

/** Open the databases of one PostgreSQL cluster, each a binding's schema, as sole writer. */
export class PostgresConnector implements DatabaseConnector {
    /** The provider code of PostgreSQL databases. */
    readonly code = "postgresql";
    /** The URL of the cluster database, such as a Hyperdrive connection string. */
    readonly cluster: URL;

    /** Open schemas of the cluster database a PostgreSQL URL selects. */
    constructor(cluster: URL) {
        // require a PostgreSQL URL
        if (cluster.protocol !== "postgres:" && cluster.protocol !== "postgresql:") {
            throw new TypeError(`postgresql cluster must be a postgres URL: ${cluster.protocol}`);
        }
        this.cluster = cluster;
    }

    /** Build the URL of one schema of the cluster. */
    url(schema: string): string {
        const url = new URL(this.cluster);
        url.searchParams.set(SCHEMA_PARAMETER, schema);

        return url.href;
    }

    /** Open the schema a binding refers to, refusing one lacking its declaration's tables. */
    async connect<Models extends Readonly<Record<string, Model>>>(
        binding: Pick<ResourceBinding, "reference">,
        declaration: Database<Models>,
    ): Promise<PostgresDatabase<Models>> {
        // open the binding's pool as the sole writer
        const url = this.url(binding.reference);
        const connection = await connectPostgres(url, declaration, "sole");

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
    }
}
