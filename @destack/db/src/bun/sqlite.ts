import { fileURLToPath } from "node:url";
import { type ResourceBinding, type ResourceRecord } from "@destack/resource";
import type { Database, DatabaseConnector } from "../declare/database.ts";
import type { Model } from "../query/model.ts";
import { DatabaseError } from "../error/error.ts";
import type { Table } from "../table/table.ts";
import { socketChannel } from "../channel/socket.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import { connect } from "./connection.ts";
import type { SqliteDatabase } from "../sqlite/database.ts";
import type { BunClient } from "./client.ts";

/** Open SQLite database files inside a workload, refusing one lacking its declaration's tables. */
export const sqliteConnector: DatabaseConnector = {
    code: "sqlite",
    async connect(binding, declaration) {
        // refuse a database lacking its tables
        const connection = await open(binding, declaration);
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
};

/** Require a provisioned resource reference. */
export function requireReference(reference: string | null): string {
    if (reference === null) {
        throw new TypeError("resource is not provisioned");
    }

    return reference;
}

/** Open a provisioned database file, announcing commits to the file's other writers on its channel. */
export function open<Models extends Readonly<Record<string, Model>>>(
    resource: Pick<ResourceRecord | ResourceBinding, "reference">,
    tables: Database<Models> | readonly Table[],
): Promise<SqliteDatabase<BunClient, Models>> {
    const path = fileURLToPath(requireReference(resource.reference));

    return connect(path, tables, { openChannel: (topic) => socketChannel(channelOf(path, topic)) });
}

/** List the channels a database file's connections share with its other writers: its log's commits. */
export function sqliteChannels(path: string): readonly string[] {
    return [channelOf(path, LOG_TOPIC)];
}

/** Name the channel a database file's connections share for one topic. */
function channelOf(path: string, topic: string): string {
    return `${path}#${topic}`;
}
