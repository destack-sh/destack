import type { DatabaseConnection } from "@destack/db/database";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import type { SimulatedBrowserEnvironment } from "@destack/environment/browser/simulation";
import type { Subject } from "@destack/sync";
import { BrowserDevice, type BrowserTabWorker } from "../../src/browser/index.ts";
import { ObjectClient, type ObjectRemote } from "../../src/client/index.ts";
import type { ObjectType } from "../../src/object/object.ts";
import { spaceId } from "./device.ts";

/** Open a device's database in memory with a client's tables, standing in for its OPFS file. */
export async function deviceDatabase(
    objects: Readonly<Record<string, ObjectType>>,
    environment: SimulatedBrowserEnvironment,
): Promise<DatabaseConnection> {
    const memory = await WasmClient.memory(environment);

    return connectWasmSqlite(memory, ObjectClient.tables(objects), { name: "notes", environment });
}

/** Serve a person's device to a browser's tabs over one database, as its worker does. */
export function deviceWorker(options: {
    readonly database: DatabaseConnection;
    readonly objects: Readonly<Record<string, ObjectType>>;
    readonly caller: Subject;
    readonly device: Subject;
    /** Reach the machine for a scope, every scope over one connection. */
    readonly remote: (scope: string) => ObjectRemote;
    readonly environment: SimulatedBrowserEnvironment;
    /** The device's own scope, the space by default. */
    readonly scope?: string;
    /** Take each device the worker serves, such as to read its holds. */
    readonly served?: (device: BrowserDevice) => void;
}): BrowserTabWorker {
    return async () => {
        // serve the device on the tabs' channel in its scope, following each scope over the one connection
        const { environment, remote, scope: own = spaceId } = options;
        const device = await BrowserDevice.serve({
            ...options,
            channel: broadcastChannel("destack:notes", environment),
            scope: own,
            remote: remote(own),
            publisher: remote,
        });

        // run it until the leading tab stops it
        options.served?.(device);
        const stopping = new AbortController();
        const running = device.run(stopping.signal);

        return async () => {
            stopping.abort();
            await running;
            await device.close();
        };
    };
}

/** Hold some of a remote's calls until a promise settles, as a machine answering late. */
export function holdRemote(
    remote: ObjectRemote,
    until: Promise<void>,
    held: ReadonlySet<"stream" | "push">,
): ObjectRemote {
    return {
        stream: async function* (subscription, signal) {
            // follow once released
            if (held.has("stream")) {
                await until;
            }
            yield* remote.stream(subscription, signal);
        },
        request: (subscription) => remote.request(subscription),
        push: async (home, push, signal) => {
            // push once released
            if (held.has("push")) {
                await until;
            }

            return remote.push(home, push, signal);
        },
        call: (scope, call, signer) => remote.call(scope, call, signer),
        broadcast: (scope, object, event, signer) => remote.broadcast(scope, object, event, signer),
    };
}
