import type { DatabaseConnection } from "@destack/db/database";
import type { Channel } from "@destack/db";
import type { Environment } from "@destack/environment";
import type {} from "@destack/environment/browser";
import type { Duration } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { ObjectClient } from "../client/client.ts";
import type { ObjectType } from "../object/object.ts";
import { ChannelRemote } from "../replica/channel.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import type { ObjectRemote } from "../replica/remote.ts";
import { BrowserTab } from "./tab.ts";

/** A person's objects of one app in one browser, serving the tabs of every scope. */
export class BrowserDevice<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The device's client, predicting and pushing the tabs' writes to their homes. */
    readonly client: ObjectClient<Objects>;
    /** Stop answering the tabs. */
    readonly #stop: () => void;

    /** Serve a client's store to the tabs on a channel. */
    private constructor(client: ObjectClient<Objects>, stop: () => void) {
        this.client = client;
        this.#stop = stop;
    }

    /** Open a person's objects over a database, and answer the tabs on a channel as that person. */
    static async serve<const Objects extends Readonly<Record<string, ObjectType>>>(options: {
        /** The device's database, an OPFS file in a browser. */
        readonly database: DatabaseConnection;
        /** The channel the person's tabs call the device on. */
        readonly channel: Channel<unknown>;
        /** The object types to replicate, by key. */
        readonly objects: Objects;
        /** The person's scope, whose own objects the device stores. */
        readonly scope: string;
        /** The person whose tabs share the device. */
        readonly caller: Subject;
        /** The browser's device storing device-scoped local objects, absent for a visitor. */
        readonly device?: Subject;
        /** The node the device follows and writes through: the machine serving its scope. */
        readonly remote: ObjectRemote;
        /** Open the node toward each followed scope's home, the remote by default. */
        readonly publisher?: (home: string) => ObjectRemote;
        /** The most object rows the device stores, absent for no limit. */
        readonly storage?: { readonly rows: number };
        /** How long the device's log retains changes, a minute by default. */
        readonly log?: { readonly retention: Duration };
        /** The environment the device runs in, with the browser's locks. */
        readonly environment: Environment<"locks"> & Environment;
    }): Promise<BrowserDevice<Objects>> {
        // open the client over the device's database, forgetting each tab once it lets go of its party lock
        const { locks } = options.environment;
        const client = await ObjectClient.open({
            ...options,
            closed: (origin, signal) =>
                locks.request(BrowserTab.lock(origin), { signal }, async () => {}),
        });

        // answer the tabs with the store as the person, whose tabs share it
        const stop = ChannelRemote.serve(
            options.channel,
            client.store.remote,
            options.caller,
            options.environment,
        );

        return new BrowserDevice(client, stop);
    }

    /** Open the device a starting tab asks for, tell the tab it serves or why not, and run it. */
    static answer(
        scope: DeviceWorkerScope,
        open: (message: unknown) => Promise<BrowserDevice>,
        message: unknown,
    ): void {
        open(message).then(
            (device) => {
                scope.postMessage({ kind: "serving" });
                void device.run(new AbortController().signal);
            },
            (error: unknown) => scope.postMessage(BrowserDevice.failed(error)),
        );
    }

    /** Write the message telling a starting tab why its device does not serve. */
    static failed(error: unknown): { readonly kind: "failed"; readonly message: string } {
        return {
            kind: "failed",
            message: error instanceof Error ? `${error.name}: ${error.message}` : String(error),
        };
    }

    /** Push the tabs' writes and follow their queries until aborted, capturing a final failure. */
    async run(signal: AbortSignal): Promise<void> {
        await this.client.run(signal).catch((error: unknown) => {
            TelemetryScope.of(
                import.meta.destack.package,
                this.client.environment.telemetry,
            ).captureException(error, { isEscaped: true });
        });
    }

    /** Stop answering the tabs and close the client. */
    async close(): Promise<void> {
        this.#stop();
        await this.client.close();
    }
}

/** The global scope of a device worker: the messages of the tab that started it. */
export interface DeviceWorkerScope {
    /** Listen to the starting tab's message asking for the device. */
    readonly addEventListener: (
        type: "message",
        listener: (event: MessageEvent<unknown>) => void,
        options: { readonly once: true },
    ) => void;
    /** Tell the starting tab the device serves, or why it cannot. */
    readonly postMessage: (message: unknown) => void;
}
