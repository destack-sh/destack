import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type * as declaration from "../../declare/database.ts";
import type { Table } from "../../table/table.ts";
import type { OpenChannel } from "../../channel/channel.ts";
import { SqliteDatabase } from "../database.ts";
import { DurableObjectClient, type DurableObjectStorage } from "./client.ts";

/** Open a Durable Object's SQLite storage, the object being its sole writer. */
export function connect(
    storage: DurableObjectStorage,
    tables: declaration.Database | readonly Table[] = [],
    options: ConnectOptions = {},
): SqliteDatabase<DurableObjectClient> {
    const { openChannel, ...drizzle } = options;

    return new SqliteDatabase(
        new DurableObjectClient(storage),
        tables,
        "embedded",
        openChannel,
        drizzle,
    );
}

/** The options of a Durable Object connection. */
export type ConnectOptions = Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> & {
    /** Open a channel of a name to the database's other connections, absent for a sole writer. */
    readonly openChannel?: OpenChannel;
};
