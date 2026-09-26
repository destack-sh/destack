import init from "@sqlite.org/sqlite-wasm";
import type { Channel } from "../../channel/channel.ts";
import { serveDatabase, type Message } from "../../shared/shared.ts";
import { WasmClient } from "./client.ts";

/** Open a database file in the origin's private file system, and serve it to every party of a channel until the returned stop runs. */
export async function serveBrowserDatabase(
    name: string,
    channel: Channel<Message>,
): Promise<() => Promise<void>> {
    // open the file through the origin private file system's synchronous access handles
    const sqlite = await init();
    const pool = await sqlite.installOpfsSAHPoolVfs({ name: "destack" });
    const database = new pool.OpfsSAHPoolDb(`/${name}`);
    database.exec("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;");

    // serve every party
    const client = new WasmClient(database);
    const stop = serveDatabase(client, channel);

    return async () => {
        stop();
        await client.close();
    };
}
