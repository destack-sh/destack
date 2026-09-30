import postgres from "postgres";
import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import type { CommitNotifier } from "../log/notifier.ts";
import { LOG_CHANNEL } from "../log/schema.ts";
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
                    postgresNotifier(client),
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

/** Listen for logged commits on the notification channel. */
function postgresNotifier(client: postgres.Sql): CommitNotifier {
    return {
        listen(commits) {
            // wake readers once listening
            const listening = client.listen(
                LOG_CHANNEL,
                () => commits.wake(),
                () => commits.wake(),
            );

            // fail the readers when listening fails
            listening.catch((error: unknown) => commits.fail(error));

            // stop listening
            return async () => {
                const listener = await listening.catch(() => undefined);
                await listener?.unlisten();
            };
        },
        notify() {
            // leave notification to the change trigger
        },
    };
}
