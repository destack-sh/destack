import type { Environment } from "@destack/environment";
import type { ResourceBinding } from "@destack/resource";
import type { Database, DatabaseConnector } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import type { Model } from "../query/model.ts";
import { connectPostgres } from "./connection.ts";
import type { PostgresDatabase } from "./database.ts";

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

    /** Open the schema a binding refers to, refusing one lacking its declaration's tables. */
    async connect<Models extends Readonly<Record<string, Model>>>(
        binding: Pick<ResourceBinding, "reference">,
        declaration: Database<Models>,
        environment: Environment,
    ): Promise<PostgresDatabase<Models>> {
        // open the binding's schema on a pool of its own, as the sole writer in the environment
        const connection = await connectPostgres(this.cluster.href, declaration, {
            writers: "sole",
            environment,
            schema: binding.reference,
        });

        // refuse a database lacking its tables
        const unapplied = await declaration.check(connection);
        if (unapplied.length > 0) {
            await connection.close();
            throw DatabaseError.of(
                "MIGRATION_PENDING",
                `database ${declaration.name} has not applied ${unapplied.join(", ")}`,
            );
        }

        return connection;
    }
}
