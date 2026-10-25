import type { Environment } from "@destack/environment";
import { fileURLToPath } from "node:url";
import { type ResourceBinding, ResourceRecord } from "@destack/resource";
import type { Database, DatabaseConnector } from "../declare/database.ts";
import type { Model } from "../query/model.ts";
import { DatabaseError } from "../error/error.ts";
import type { Table } from "../table/table.ts";
import { socketChannel } from "../channel/socket.ts";
import { LOG_TOPIC } from "../log/schema.ts";
import { connectBunSqlite } from "./connection.ts";
import type { SqliteDatabase } from "../sqlite/database.ts";
import type { BunClient } from "./client.ts";

/** Open SQLite database files inside a workload, refusing one lacking its declaration's tables. */
export const bunSqliteConnector: DatabaseConnector = {
    code: "sqlite",
    async connect(binding, declaration, environment) {
        // require the files and server the channel needs
        if (!hasChannelEffects(environment)) {
            throw new TypeError("a sqlite database file needs its environment's files and server");
        }

        // open the file, refusing a database lacking its tables
        const connection = await openBunSqlite(binding, declaration, environment);
        const unapplied = await declaration.check(connection);
        if (unapplied.length > 0) {
            await connection.close();
            throw DatabaseError.of(
                "MIGRATION_PENDING",
                `database ${declaration.name} has not applied ${unapplied.join(", ")}`,
            );
        }

        return connection;
    },
};

/** Open a provisioned database file, announcing commits to its other writers on its channel. */
export function openBunSqlite<Models extends Readonly<Record<string, Model>>>(
    resource: Pick<ResourceRecord | ResourceBinding, "reference">,
    tables: Database<Models> | readonly Table[],
    environment: Environment<"files" | "server">,
): Promise<SqliteDatabase<BunClient, Models>> {
    const path = fileURLToPath(ResourceRecord.reference(resource));

    return connectBunSqlite(path, tables, {
        openChannel: (topic) => socketChannel(channelOf(path, topic), environment),
        environment,
    });
}

/** List the channels a database file shares with its other writers: its log's commits. */
export function bunSqliteChannels(path: string): readonly string[] {
    return [channelOf(path, LOG_TOPIC)];
}

/** Write the channel a database file's connections share for one topic. */
function channelOf(path: string, topic: string): string {
    return `${path}#${topic}`;
}

/** Report whether an environment has files and a server, as a database file's channel needs. */
function hasChannelEffects(
    environment: Environment,
): environment is Environment<"files" | "server"> {
    return "files" in environment && "server" in environment;
}
