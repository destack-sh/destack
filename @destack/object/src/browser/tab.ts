import type { SqliteDatabase } from "@destack/db/sqlite";
import { type Duration, type JsonObject, schema } from "@destack/schema";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import { type Environment, waitAborted } from "@destack/environment";
import type {} from "@destack/environment/browser";
import { RequestId, type Subject } from "@destack/sync";
import { ObjectError } from "../error/index.ts";
import { ObjectClient } from "../client/client.ts";
import type { ObjectType } from "../object/object.ts";
import { ChannelRemote } from "../replica/channel.ts";
import { Replay } from "../audit/replay.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** What the device worker tells the tab that started it: it serves, or it failed to open. */
const WorkerAnswer = schema.discriminatedUnion("kind", [
    schema.object({ kind: schema.literal("serving") }),
    schema.object({ kind: schema.literal("failed"), message: schema.string() }),
]);

/** The browser effects a tab takes: the origin's locks, broadcast channels and storage. */
export type BrowserTabEnvironment = Environment<"locks" | "broadcast" | "storage">;

/** Start the device worker, returning how to stop it. */
export type BrowserTabWorker = () => Promise<() => Promise<void>>;

/** A tab's objects in memory, following and writing through the person's device. */
export class BrowserTab<
    Objects extends Readonly<Record<string, ObjectType>> = Readonly<Record<string, ObjectType>>,
> {
    /** The tab's client. */
    readonly client: ObjectClient<Objects>;
    /** Settles once the tab has the caller's access rows, or refuses with why not. */
    readonly ready: Promise<void>;
    /** The device the tab follows and writes through. */
    readonly #remote: ChannelRemote;
    /** The tab's database in memory. */
    readonly #database: SqliteDatabase;
    /** Aborts the tab's loops on close. */
    readonly #stopping: AbortController;
    /** Settles once the tab's loops end. */
    readonly #running: Promise<void>;
    /** Settles once the tab no longer holds its party lock. */
    readonly #party: Promise<void>;

    /** Run a tab's client, leading the device once no other tab does. */
    private constructor(
        client: ObjectClient<Objects>,
        database: SqliteDatabase,
        remote: ChannelRemote,
        options: {
            readonly name: string;
            readonly election: DeviceElection;
            readonly environment: BrowserTabEnvironment & Environment;
            /** Aborts the tab's loops and its party lock on close. */
            readonly stopping: AbortController;
            /** Settles once the tab no longer holds its party lock. */
            readonly party: Promise<void>;
        },
    ) {
        // refuse readiness on the tab's first final failure, such as a refused follow or a failed worker
        this.client = client;
        this.#database = database;
        this.#remote = remote;
        this.#stopping = options.stopping;
        this.#party = options.party;
        const { signal } = this.#stopping;
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => undefined);
        const instruments = TelemetryScope.of(
            import.meta.destack.package,
            options.environment.telemetry,
        );
        const stopRefusals = client.store.listenRefusals((error) => {
            if (Replay.failure(error) !== undefined) {
                failed.reject(error);
            }
        });

        // fail readiness when the page's election of the device fails, unless closing
        options.election.failed.catch((error: unknown) => {
            if (!signal.aborted) {
                failed.reject(error);
            }
        });

        // run the client until closed, capturing a failure that ends it, ready once the tab has the caller's access rows
        this.#running = client
            .run(signal)
            .catch((error: unknown) => {
                failed.reject(error);
                instruments.captureException(error, { isEscaped: true });
            })
            .finally(stopRefusals);
        const synced = client.database.log
            .waitUntil(
                async () => (await client.access.position(client.database)) !== undefined,
                signal,
            )
            .then(() => {});
        this.ready = Promise.race([synced, failed.promise]);
    }

    /** Open a tab's objects in memory over the person's device on the origin's channel. */
    static async open<const Objects extends Readonly<Record<string, ObjectType>>>(options: {
        /** The device's name, shared by every tab that opens the same device. */
        readonly name: string;
        /** The object types to replicate, by key. */
        readonly objects: Objects;
        /** The scope whose objects to replicate. */
        readonly scope: string;
        /** The calling principal. */
        readonly caller: Subject;
        /** The browser's device storing device-scoped local objects, absent for a visitor. */
        readonly device?: Subject;
        /** How often the device's merged changes arrive, such as for a background tab. */
        readonly refresh?: { readonly every: Duration };
        /** The page's election of the device, which starts its worker once the page leads. */
        readonly election: DeviceElection;
        /** The environment the tab runs in, with the browser's locks, channels and storage. */
        readonly environment: Environment & BrowserTabEnvironment;
    }): Promise<BrowserTab<Objects>> {
        // store the tab's rows in memory
        const { environment, objects } = options;
        const memory = await WasmClient.memory(environment);
        const database = connectWasmSqlite(memory, ObjectClient.tables(objects), {
            name: "memory",
            environment,
        });

        // follow and write through the device on the origin's channel
        const channel = broadcastChannel(`destack:${options.name}`, environment);
        const remote = new ChannelRemote(channel, RequestId.create(environment), environment);
        const client = await ObjectClient.open({ ...options, database, remote, local: "followed" });

        // hold the tab's party lock until it closes, before it streams, telling the device once it is gone
        const stopping = new AbortController();
        const origin = (await database.log.position()).log;
        const held = Promise.withResolvers<void>();
        const party = environment.locks
            .request(BrowserTab.lock(origin), { signal: stopping.signal }, async () => {
                held.resolve();
                await waitAborted(stopping.signal);
            })
            .catch((error: unknown) => {
                if (!stopping.signal.aborted) {
                    held.reject(error);
                }
            });
        await held.promise;

        return new BrowserTab(client, database, remote, { ...options, stopping, party });
    }

    /** Start a module's device worker at once, asking it for the device once the page can. */
    static worker(
        module: URL,
        name: string,
        request: Promise<JsonObject>,
        environment: Environment<"storage" | "workers">,
    ): BrowserTabWorker {
        return async () => {
            // ask to persist the storage beside the start, as a browser may wait for the person's answer
            void environment.storage
                .persist()
                .catch((error: unknown) => globalThis.reportError(error));

            // start the device worker loading its modules, then ask for the device once the page knows its request
            const worker = environment.workers.start(
                (Worker) => new Worker(module, { type: "module", name: `destack:${name}` }),
            );
            worker.postMessage(await request);

            // wait until the worker serves the device, refusing one that cannot open its database or fails to load
            const message = await new Promise<unknown>((resolve, reject) => {
                worker.addEventListener("message", (event) => resolve(event.data));
                worker.addEventListener("error", (event) =>
                    reject(new Error(`the device worker failed: ${event.message}`)),
                );
            });
            const answer = WorkerAnswer.parse(message);
            if (answer.kind === "failed") {
                worker.terminate();
                throw ObjectError.of(
                    "DATABASE_UNAVAILABLE",
                    `database '${name}' is unavailable: this browser cannot open it: ${answer.message}`,
                );
            }

            return async () => worker.terminate();
        };
    }

    /** Name the lock a tab holds while open, by its log, which a device waits on to forget it. */
    static lock(origin: string): string {
        return `destack:party:${origin}`;
    }

    /** Close the tab, handing the device over to another tab when it leads. */
    async close(): Promise<void> {
        // stop the loops, then close the client, the channel and the database
        this.#stopping.abort();
        await this.#running;
        await this.#party;
        await this.client.close();
        await this.#remote.close();
        await this.#database.close();
    }
}

/** A page's election to lead its device, running its worker while the page leads. */
export class DeviceElection {
    /** Rejects once the worker fails to start, which fails the page's tabs. */
    readonly failed: Promise<never>;
    /** Aborts the election, stopping a started worker. */
    readonly #stopping: AbortController;
    /** Settles once the page no longer leads or waits to. */
    readonly #leading: Promise<void>;

    /** Stand with a stopping controller, the settled election and its failure. */
    private constructor(stopping: AbortController, leading: Promise<void>, failed: Promise<never>) {
        this.#stopping = stopping;
        this.#leading = leading;
        this.failed = failed;
    }

    /** Stand to lead a device, starting its worker once the page leads. */
    static start(
        name: string,
        worker: BrowserTabWorker,
        environment: Environment<"locks">,
    ): DeviceElection {
        // run the worker while holding the device's lock, failing the election on its failure unless closing
        const stopping = new AbortController();
        const { signal } = stopping;
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => undefined);
        const lead = async () => {
            if (!signal.aborted) {
                const stop = await worker();
                try {
                    await waitAborted(signal);
                } finally {
                    await stop();
                }
            }
        };
        const leading = environment.locks
            .request(`destack:${name}:device`, { signal }, lead)
            .catch((error: unknown) => {
                if (!signal.aborted) {
                    failed.reject(error);
                }
            });

        return new DeviceElection(stopping, leading, failed.promise);
    }

    /** Leave the election, stopping a started worker. */
    async close(): Promise<void> {
        this.#stopping.abort();
        await this.#leading;
    }
}
