import type { DatabaseConnector } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { DURABLE_OBJECT_PROVIDER, DurableObjectDatabaseHost } from "./host.ts";

/** Open a Durable Object's databases in its workload, refusing ones lacking declared tables. */
export function durableObjectConnector(host: DurableObjectDatabaseHost): DatabaseConnector {
    return {
        code: DURABLE_OBJECT_PROVIDER,
        async connect(binding, declaration) {
            // open the database's namespace in the object's storage, on the host's clock
            const namespace = DurableObjectDatabaseHost.namespace(binding.reference);
            const database = host.connect(declaration, { namespace });

            // refuse a database lacking its tables
            const unapplied = await declaration.check(database);
            if (unapplied.length > 0) {
                await database.close();
                throw DatabaseError.of(
                    "MIGRATION_PENDING",
                    `database ${declaration.name} has not applied ${unapplied.join(", ")}`,
                );
            }

            return database;
        },
    };
}
