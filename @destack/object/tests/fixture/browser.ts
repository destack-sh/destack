import type { DatabaseConnection } from "@destack/db";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import type { SimulatedBrowserEnvironment } from "@destack/environment/browser/simulation";
import type { Subject } from "@destack/sync";
import { BrowserDevice, type BrowserTabWorker } from "../../src/browser/index.ts";
import { ObjectClient, ProcedureRemote } from "../../src/client/index.ts";
import type { ClientOptions } from "../../src/client/package.ts";
import type { ObjectType } from "../../src/object/object.ts";
import { unmoved } from "../../src/test/space.ts";
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
    readonly endpoint: ClientOptions;
    readonly environment: SimulatedBrowserEnvironment;
    /** The device's own scope, the space by default. */
    readonly scope?: string;
    /** Take each device the worker serves, such as to read its holds. */
    readonly served?: (device: BrowserDevice) => void;
}): BrowserTabWorker {
    return async () => {
        // serve the device on the tabs' channel in its scope, following each scope through the fixture's machine
        const { environment, scope: own = spaceId } = options;
        const remotes = new Map<string, ProcedureRemote>();
        const publisher = (scope: string) => {
            const remote =
                remotes.get(scope) ??
                ProcedureRemote.at({
                    scope,
                    endpoint: options.endpoint,
                    reconnect: unmoved,
                    environment,
                });
            remotes.set(scope, remote);

            return remote;
        };
        const device = await BrowserDevice.serve({
            ...options,
            channel: broadcastChannel("destack:notes", environment),
            scope: own,
            remote: publisher(own),
            publisher,
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
