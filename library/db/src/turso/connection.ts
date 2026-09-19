import * as turso from "@tursodatabase/database";
import { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { AnyRelations, EmptyRelations } from "drizzle-orm/relations";
import { ConnectionSession } from "./session/connection.ts";

export type { Transaction } from "./session/transaction.ts";

/** The result of executing a Turso statement. */
type RunResult = Awaited<ReturnType<turso.Database["run"]>>;

/** Drizzle queries backed by a Turso connection and scoped transactions. */
export class Database<Relations extends AnyRelations = EmptyRelations>
    extends SQLiteAsyncDatabase<"async", RunResult, Relations> {
    /** The underlying Turso connection. */
    readonly $client: turso.Database;

    /** Connect query builders to the physical database. */
    constructor(client: turso.Database, options: DrizzleSQLiteConfig<Relations> = {}) {
        const relations = options.relations ?? {} as Relations;
        const session = new ConnectionSession<RunResult, Relations>(client, relations, options);
        super("async", session.dialect, session, relations);
        this.$client = client;
    }
}

/** Open a Turso connection with Drizzle query and relation options. */
export function connect<Relations extends AnyRelations = EmptyRelations>(
    connection: string | turso.Database,
    options: DrizzleSQLiteConfig<Relations> = {},
): Database<Relations> {
    const client = typeof connection === "string" ? new turso.Database(connection) : connection;

    return new Database(client, options);
}
