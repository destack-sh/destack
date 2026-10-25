import init from "@sqlite.org/sqlite-wasm";
import type { DatabaseEnvironment } from "../database/connection.ts";
import type { Channel } from "../channel/channel.ts";
import { serveDatabase } from "./shared.ts";
import { WasmClient } from "./client.ts";

/** Open an OPFS database file and serve it to a channel's parties until stopped. */
export async function serveBrowserDatabase(
    name: string,
    channel: Channel<unknown>,
    environment: DatabaseEnvironment,
): Promise<() => Promise<void>> {
    // open the file through synchronous access handles
    const sqlite = await init();
    const pool = await sqlite.installOpfsSAHPoolVfs({ name: "destack" });
    const database = new pool.OpfsSAHPoolDb(`/${name}`);
    database.exec("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;");

    // serve every party
    const client = new WasmClient(database, environment.clock);
    const stop = serveDatabase(client, channel, environment);

    return async () => {
        stop();
        await client.close();
    };
}
