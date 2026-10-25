import type { DatabaseEnvironment } from "../database/connection.ts";
import type { SqliteConnectionClient } from "./client.ts";
import {
    ConnectionState,
    DatabaseConnection,
    requireDistinct,
    type Locality,
} from "../database/connection.ts";
import type { Channel } from "../channel/channel.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { SqliteSession } from "../database/session.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { Relations } from "../query/relation.ts";
import type { Model } from "../query/model.ts";

/** A SQLite database with a separate connection. */
export class SqliteDatabase<
    Client extends SqliteConnectionClient = SqliteConnectionClient,
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends DatabaseConnection<Models> {
    /** The SQLite connection client. */
    readonly $client: Client;

    /** Bind tables to a SQLite connection, within a shared store's namespace when given. */
    constructor(
        client: Client,
        tables: declaration.Database<Models> | readonly Table[],
        locality: Locality,
        options: SqliteConnectOptions & { readonly namespace?: string },
    ) {
        // declare the tables with their tree tables
        const declared = "tables" in tables ? tables.tables : expandTrees(tables);
        requireDistinct(declared, "sqlite");
        super(
            new DatabaseDriver(
                new SqliteSession(client, options.environment.clock),
                new ConnectionState({
                    locality,
                    openChannel: options.openChannel,
                    environment: options.environment,
                    announcer: "connection",
                    tables: declared,
                    dialect: "sqlite",
                    replicated: "tables" in tables ? tables.replicated : new Set(),
                    ...(options.namespace === undefined ? {} : { namespace: options.namespace }),
                }),
            ),
            declared,
            "tables" in tables ? tables.relations : new Relations<Models>(),
        );
        this.$client = client;
    }

    /** Close the database when its owner's scope ends. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Close the physical database. */
    close(): Promise<void> {
        return this.state.close(() => this.$client.close());
    }
}

/** The options of a SQLite connection. */
export interface SqliteConnectOptions {
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel?: (name: string) => Channel<unknown>;
    /** The clock, randomness, crypto and telemetry the connection runs on. */
    readonly environment: DatabaseEnvironment;
}
