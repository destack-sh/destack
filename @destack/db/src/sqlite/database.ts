import type { ConnectionClient } from "./client.ts";
import { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { EmptyRelations } from "drizzle-orm/relations";
import { ConnectionSession } from "./session.ts";
import { ConnectionState, DatabaseConnection, type Locality } from "../database/connection.ts";
import type { CommitNotifier } from "../log/notifier.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { SqliteSchemaCompiler } from "./compiler.ts";
import { expandTrees } from "../tree/tree.ts";
import * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import type { SQL } from "drizzle-orm";

/** Portable queries with an owned SQLite connection: embedded or hosted Turso, WebAssembly or shared. */
export class SqliteDatabase<
    Client extends ConnectionClient<unknown> = ConnectionClient<unknown>,
> extends DatabaseConnection<"sqlite"> {
    /** The underlying SQLite connection client. */
    readonly $client: Client;
    /** The explicit native SQL API. */
    readonly native: SqliteNative<Client>;

    /** Bind logical tables to a SQLite connection running somewhere, exchanging commit notifications with other writers. */
    constructor(
        client: Client,
        tables: declaration.Database | readonly Table[],
        locality: Locality,
        notifier: CommitNotifier,
        options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
    ) {
        // compile a database's tables, or the given tables with their trees' tables
        const compiler = new SqliteSchemaCompiler(
            tables instanceof declaration.Database ? tables.tables : expandTrees(tables),
        );
        const native = new SqliteNative(client, options);
        super(
            new DatabaseDriver(
                { dialect: "sqlite", database: native },
                new ConnectionState(locality, notifier),
            ),
            compiler,
        );
        this.$client = client;
        this.native = native;
    }

    /** Execute an explicit SQL statement. */
    async run(statement: SQL): Promise<RunResult<Client>> {
        return await this.driver.write(() => this.native.run(this.compiler.expression(statement)));
    }

    /** Close the physical database. */
    close(): Promise<void> {
        return this.state.close(() => this.$client.close());
    }
}

/** Native Drizzle queries over a SQLite connection client. */
export class SqliteNative<
    Client extends ConnectionClient<unknown> = ConnectionClient<unknown>,
> extends SQLiteAsyncDatabase<"async", RunResult<Client>, EmptyRelations> {
    /** The underlying SQLite connection client. */
    readonly $client: Client;

    /** Connect native query builders to the physical database. */
    constructor(
        client: Client,
        options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
    ) {
        // open a session over the client without relational queries
        const relations = {} as EmptyRelations;
        const session = new ConnectionSession<RunResult<Client>, EmptyRelations>(
            client as ConnectionClient<RunResult<Client>>,
            relations,
            options,
        );
        super("async", session.dialect, session, relations);
        this.$client = client;
    }
}

/** The result of executing a statement on a client. */
type RunResult<Client extends ConnectionClient<unknown>> = Awaited<ReturnType<Client["run"]>>;
