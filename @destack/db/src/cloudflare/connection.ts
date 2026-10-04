import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { SqliteDatabase, type ConnectOptions } from "../sqlite/database.ts";
import { DurableObjectClient, type DurableObjectStorage } from "./client.ts";
import type { Model } from "../query/model.ts";

/** Open a Durable Object's SQLite storage, the object being its sole writer. */
export function connect<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    storage: DurableObjectStorage,
    tables: declaration.Database<Models> | readonly Table[] = [],
    options: ConnectOptions = {},
): SqliteDatabase<DurableObjectClient, Models> {
    const { openChannel } = options;

    return new SqliteDatabase(new DurableObjectClient(storage), tables, "embedded", openChannel);
}
