import postgres from "postgres";
import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import type { CommitNotifier } from "../log/notifier.ts";
import { LOG_CHANNEL } from "../log/trigger.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSchemaCompiler } from "./compiler.ts";
import { expandTrees } from "../tree/tree.ts";
import * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";

/** Portable queries with an owned PostgreSQL connection pool. */
export class PostgresDatabase extends DatabaseConnection<"postgresql"> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;
    /** The explicit native SQL API. */
    readonly native: PostgresJsDatabase;

    /** Bind logical tables to a PostgreSQL connection pool. */
    constructor(
        client: postgres.Sql,
        tables: declaration.Database | readonly Table[],
        options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
    ) {
        // compile a database's tables, or the given tables with their trees' tables
        const compiler = new PostgresSchemaCompiler(
            tables instanceof declaration.Database ? tables.tables : expandTrees(tables),
        );
        const native = drizzle({ ...options, client });
        super(
            new DatabaseDriver(
                { dialect: "postgresql", database: native },
                new ConnectionState("networked", postgresNotifier(client)),
            ),
            compiler,
        );
        this.$client = client;
        this.native = native;
    }

    /** Close the connection pool after pending queries complete. */
    async close(): Promise<void> {
        await this.state.close(() => this.$client.end());
    }
}

/** Listen for every commit that changed the log on its notification channel. */
function postgresNotifier(client: postgres.Sql): CommitNotifier {
    return {
        listen(commits) {
            // wake readers once listening, since commits before it went unannounced
            const listening = client.listen(
                LOG_CHANNEL,
                () => commits.wake(),
                () => commits.wake(),
            );

            // fail the readers when listening fails, since no commit would wake them
            listening.catch((error: unknown) => commits.fail(error));

            // stop listening, when listening started
            return async () => {
                const listener = await listening.catch(() => undefined);
                await listener?.unlisten();
            };
        },
        notify() {
            // leave the commit to the change trigger, which notifies every listener
        },
    };
}
