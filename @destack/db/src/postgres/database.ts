import postgres from "postgres";
import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import type { Channel } from "../channel/channel.ts";
import { CHANNEL_PREFIX } from "../log/schema.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSchemaCompiler } from "./compiler.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";

/** A PostgreSQL database with its own pool. */
export class PostgresDatabase extends DatabaseConnection<"postgresql"> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;
    /** The native SQL API. */
    readonly native: PostgresJsDatabase;

    /** Bind tables to a connection pool. */
    constructor(
        client: postgres.Sql,
        tables: declaration.Database | readonly Table[],
        options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
    ) {
        // compile the tables with their tree tables
        const compiler = new PostgresSchemaCompiler(
            "tables" in tables ? tables.tables : expandTrees(tables),
        );
        const native = drizzle({ ...options, client });
        super(
            new DatabaseDriver(
                { dialect: "postgresql", database: native },
                new ConnectionState(
                    "networked",
                    (name) => postgresChannel(client, `${CHANNEL_PREFIX}${name}`),
                    "database",
                    "tables" in tables ? tables.spec.tier : undefined,
                ),
            ),
            compiler,
        );
        this.$client = client;
        this.native = native;
    }

    /** Close the pool after pending queries. */
    async close(): Promise<void> {
        await this.state.close(() => this.$client.end());
    }
}

/** Reach every party of a PostgreSQL notification channel, carrying messages as JSON. */
export function postgresChannel<Message>(client: postgres.Sql, name: string): Channel<Message> {
    return {
        notify: (message) => void client.notify(name, JSON.stringify(message)),
        listen: (receive, resume, fail) => {
            // deliver each notification once listening, resuming on each reconnect
            const listening = client.listen(
                name,
                (payload) => receive(JSON.parse(payload) as Message),
                () => resume?.(),
            );
            listening.catch((error: unknown) => fail?.(error));

            // stop listening
            return () =>
                void listening.then(
                    (listener) => listener.unlisten(),
                    () => undefined,
                );
        },
    };
}
