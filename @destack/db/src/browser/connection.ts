import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import type { Model } from "../query/model.ts";
import { SqliteDatabase, type SqliteConnectOptions } from "../sqlite/database.ts";
import type { WasmClient } from "./client.ts";

/** Connect to a SQLite WebAssembly database, in memory or an OPFS file, as its only writer. */
export function connectWasmSqlite<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(
    client: WasmClient,
    tables: declaration.DatabaseSchema<Models> | readonly Table[],
    options: SqliteConnectOptions & { readonly name: string },
): SqliteDatabase<WasmClient, Models> {
    return new SqliteDatabase(client, tables, "embedded", "writer", options);
}
