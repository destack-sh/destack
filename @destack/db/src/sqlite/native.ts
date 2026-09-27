import { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { ConnectionClient } from "./client.ts";
import { ConnectionSession } from "./session.ts";

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
export type RunResult<Client extends ConnectionClient<unknown>> = Awaited<
    ReturnType<Client["run"]>
>;
