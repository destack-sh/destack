import type { ConnectionClient } from "./client.ts";
import { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { EmptyRelations } from "drizzle-orm/relations";
import { SqliteNative, type RunResult } from "./native.ts";
import { ConnectionState, DatabaseConnection, type Locality } from "../database/connection.ts";
import type { OpenChannel } from "../channel/channel.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { SqliteSchemaCompiler } from "./compiler.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import type { SQL } from "drizzle-orm";

/** A SQLite database with its own connection. */
export class SqliteDatabase<
    Client extends ConnectionClient<unknown> = ConnectionClient<unknown>,
> extends DatabaseConnection<"sqlite"> {
    /** The SQLite connection client. */
    readonly $client: Client;
    /** The native SQL API. */
    readonly native: SqliteNative<Client>;

    /** Bind tables to a SQLite connection. */
    constructor(
        client: Client,
        tables: declaration.Database | readonly Table[],
        locality: Locality,
        openChannel: OpenChannel | undefined,
        options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
    ) {
        // compile the tables with their tree tables
        const compiler = new SqliteSchemaCompiler(
            "tables" in tables ? tables.tables : expandTrees(tables),
        );
        const native = new SqliteNative(client, options);
        super(
            new DatabaseDriver(
                { dialect: "sqlite", database: native },
                new ConnectionState(
                    locality,
                    openChannel,
                    "connection",
                    "tables" in tables ? tables.spec.tier : undefined,
                ),
            ),
            compiler,
        );
        this.$client = client;
        this.native = native;
    }

    /** Execute an SQL statement. */
    async run(statement: SQL): Promise<RunResult<Client>> {
        return (await this.driver.write((native) =>
            (native.database as SQLiteAsyncDatabase<"async", unknown>).run(
                this.compiler.expression(statement),
            ),
        )) as RunResult<Client>;
    }

    /** Close the physical database. */
    close(): Promise<void> {
        return this.state.close(() => this.$client.close());
    }
}
