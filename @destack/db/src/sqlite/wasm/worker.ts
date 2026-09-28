import init from "@sqlite.org/sqlite-wasm";
import type { Relay } from "../../relay/relay.ts";
import { serveDatabase, type Message } from "../shared/shared.ts";
import { WasmClient } from "./client.ts";

/** Open an OPFS database file and serve it to a relay's parties until stopped. */
export async function serveBrowserDatabase(
    name: string,
    relay: Relay<Message>,
): Promise<() => Promise<void>> {
    // open the file through synchronous access handles
    const sqlite = await init();
    const pool = await sqlite.installOpfsSAHPoolVfs({ name: "destack" });
    const database = new pool.OpfsSAHPoolDb(`/${name}`);
    database.exec("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;");

    // serve every party
    const client = new WasmClient(database);
    const stop = serveDatabase(client, relay);

    return async () => {
        stop();
        await client.close();
    };
}
