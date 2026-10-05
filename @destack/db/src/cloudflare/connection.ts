import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { SqliteDatabase, type ConnectOptions } from "../sqlite/database.ts";
import { DurableObjectClient, type DurableObjectStorage } from "./client.ts";
import type { Model } from "../query/model.ts";

/** The options of a connection to a database in a Durable Object's SQLite storage. */
export interface DurableObjectConnectOptions extends ConnectOptions {
    /** The database's namespace in the storage several databases share, such as its identifier; absent for the storage's only database. */
    readonly namespace?: string;
}

/** Open a database in a Durable Object's SQLite storage, the object being its sole writer. */
export function connect<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    storage: DurableObjectStorage,
    tables: declaration.Database<Models> | readonly Table[] = [],
    options: DurableObjectConnectOptions = {},
): SqliteDatabase<DurableObjectClient, Models> {
    const { openChannel, namespace } = options;

    return new SqliteDatabase(
        new DurableObjectClient(storage),
        tables,
        "embedded",
        openChannel,
        namespace,
    );
}
