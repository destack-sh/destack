import { fileURLToPath } from "node:url";
import {
    ResourceDeclaration,
    type Connector,
    type ResourceBinding,
    type ResourceRecord,
} from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Database } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import type { Table } from "../table/table.ts";
import { socketChannel } from "../channel/socket.ts";
import { connect } from "./bun/connection.ts";
import type { SqliteDatabase } from "./database.ts";

/** Open SQLite database files inside a workload, refusing one lacking its declaration's tables. */
export const sqliteConnector: Connector<DatabaseConnection> = {
    code: "sqlite",
    connect: async (binding, declaration) => {
        // refuse a database lacking its tables
        const database = requireDatabase(declaration);
        const connection = await open(binding, database.tables);
        const unapplied = await database.check(connection);
        if (unapplied.length > 0) {
            await connection.close();
            throw new DatabaseError(
                "NOT_APPLIED",
                `database ${database.name} has not applied ${unapplied.join(", ")}`,
            );
        }

        return connection;
    },
};

/** Require a provisioned resource reference. */
export function requireReference(reference: string | null): string {
    if (reference === null) {
        throw new TypeError("resource is not provisioned");
    }

    return reference;
}

/** Require a database declaration. */
function requireDatabase(declaration: unknown): Database {
    if (!(declaration instanceof ResourceDeclaration) || declaration.kind !== "database") {
        throw new TypeError("not a database declaration");
    }

    return declaration as Database;
}

/** Open a provisioned database file, announcing commits to the file's other writers on its channel. */
export function open(
    resource: Pick<ResourceRecord | ResourceBinding, "reference">,
    tables: readonly Table[],
): Promise<SqliteDatabase> {
    const path = fileURLToPath(requireReference(resource.reference));

    return connect(path, tables, { openChannel: (name) => socketChannel(`${path}#${name}`) });
}
