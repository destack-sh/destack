import { channelHub } from "@destack/db";
import { ObjectError } from "../error/error.ts";
import {
    DATABASE_PATH,
    DURABLE_OBJECT_PROVIDER,
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
    durableObjectConnector,
} from "@destack/db/cloudflare";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/worker";
import type {} from "@destack/package/import-meta";
import type { Alarm } from "@destack/db";

import { refusal } from "../error/report.ts";
import {
    type RunnerOptions,
    SOCKET_PATH,
    WorkloadRunner,
    WorkloadStart,
} from "../workload/index.ts";

/** The instruments capturing the failures of the workloads instances run. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The export of a workerd output's workload entry with the class of its instances' objects. */
export const WORKLOAD_EXPORT = "Workload";

/** The path a partition's object posts its workload's start to. */
export const START_PATH = "/.destack/start";

/** The path where a partition's object asks its workload to drain and forget its start. */
export const STOP_PATH = "/.destack/stop";

/** The path where a partition's object rings an instance's alarm. */
export const ALARM_PATH = "/.destack/alarm";

/** The path below which an instance reads its build's files through its partition's object. */
export const BUILD_PATH = "/.destack/build";

/** The key of the stored start an instance resumes after an eviction. */
const START_KEY = "start";

/** The namespace of the database an instance has only while it runs. */
const EPHEMERAL_NAMESPACE = "ephemeral";

/** The start a partition's object hands its workload. */
export const DurableObjectStart = schema.object({
    /** The machine's start of the workload. */
    start: WorkloadStart,
    /** The manifest of the build the workload runs. */
    manifest: PackageManifest,
});
/** The start a partition's object hands its workload. */
export type DurableObjectStart = schema.Infer<typeof DurableObjectStart>;

/** A ring of an instance's alarm, or a move of its wake-up. */
export const DurableObjectAlarm = schema.object({
    /** When the ring's run ends or the wake-up rings next, in UTC epoch milliseconds. */
    at: schema.number().int().nullable(),
});
/** A ring of an instance's alarm, or a move of its wake-up. */
export type DurableObjectAlarm = schema.Infer<typeof DurableObjectAlarm>;

/** The storage of an instance's object: its databases and the start it stores. */
export interface DurableObjectRunnerStorage extends DurableObjectStorage {
    /** Read a stored value. */
    readonly get: (key: string) => Promise<unknown>;
    /** Store a value. */
    readonly put: (key: string, value: unknown) => Promise<void>;
    /** Forget a value. */
    readonly delete: (key: string) => Promise<boolean>;
}

/** The state of an instance's object. */
export interface DurableObjectRunnerState {
    /** The object's storage. */
    readonly storage: DurableObjectRunnerStorage;
    /** Block the object's other events until a closure settles. */
    readonly blockConcurrencyWhile: <Value>(closure: () => Promise<Value>) => Promise<Value>;
}

/** The bindings an instance's facet reads. */
export interface DurableObjectRunnerBindings {
    /** The partition's object. */
    readonly PARTITION: { readonly fetch: (request: Request) => Promise<Response> };
}

/** A workload instance in its partition's facet, beside its databases. */
export class DurableObjectRunner {
    /** The object's storage. */
    readonly #storage: DurableObjectRunnerStorage;
    /** The partition's object. */
    readonly #partition: DurableObjectRunnerBindings["PARTITION"];
    /** What the workload runs with. */
    readonly #runner: Omit<RunnerOptions, "ephemeral">;
    /** The object's databases. */
    readonly #databases: DurableObjectDatabaseHost;
    /** The channel the object's instances replicate their ephemeral writes over, one per object. */
    readonly #channels = channelHub<unknown>();
    /** The running workload, absent before a start and after a stop. */
    #running: Promise<WorkloadRunner | undefined>;

    /** Resume the stored start before the object takes any event. */
    constructor(
        state: DurableObjectRunnerState,
        bindings: DurableObjectRunnerBindings,
        runner: Omit<RunnerOptions, "ephemeral">,
    ) {
        // set the storage, the partition's object and the databases
        this.#storage = state.storage;
        this.#partition = bindings.PARTITION;
        this.#runner = runner;
        this.#databases = new DurableObjectDatabaseHost(state.storage);
        this.#running = state.blockConcurrencyWhile(() => this.#resume());
    }

    /** Answer the partition's object's database operations, starts, stops and rings. */
    async fetch(request: Request): Promise<Response> {
        try {
            const { pathname } = new URL(request.url);

            // run a database operation, reading content through the partition's object
            if (pathname === DATABASE_PATH) {
                return await this.#databases.answer(request, () => this.#partition);
            }
            // store a new start and run it
            else if (pathname === START_PATH) {
                const started = DurableObjectStart.parse(await request.json());
                await this.#stop();
                await this.#storage.put(START_KEY, started);
                this.#running = this.#start(started);
                await this.#running;

                return Response.json({});
            }
            // drain and forget the start
            else if (pathname === STOP_PATH) {
                await this.#stop();
                await this.#storage.delete(START_KEY);

                return Response.json({});
            }
            // run the due controllers until the ring's deadline
            else if (pathname === ALARM_PATH) {
                const { at } = DurableObjectAlarm.parse(await request.json());
                await (await this.#running)?.alarm(at ?? Date.now());

                return Response.json({});
            }

            // serve the workload
            const running = await this.#running;
            if (running === undefined) {
                throw new ObjectError("NO_WORKLOAD", {
                    message: "no workload runs here",
                });
            }

            return await running.fetch(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Start the stored start, absent before a start. */
    async #resume(): Promise<WorkloadRunner | undefined> {
        const stored = await this.#storage.get(START_KEY);

        return stored === undefined ? undefined : this.#start(DurableObjectStart.parse(stored));
    }

    /** Start the workload over the object's databases. */
    #start(started: DurableObjectStart): Promise<WorkloadRunner> {
        // read the build's files through the partition's object and set the wake-up on its alarm
        const partition = this.#partition;
        const build = new BuildReader(started.manifest, async (path) => {
            const response = await partition.fetch(
                new Request(`https://partition${BUILD_PATH}/${path}`, {
                    headers: { authorization: `Bearer ${started.start.secret}` },
                }),
            );
            if (!response.ok) {
                throw new ObjectError("BUILD_FILE_UNAVAILABLE", {
                    message: `the partition has no build file ${path}: ${response.status}`,
                });
            }

            return present(response.body, `the partition's body of build file ${path}`);
        });
        const alarm: Alarm = {
            setAlarm: (at) => this.#move(at),
            deleteAlarm: () => this.#move(null),
        };

        // open the bound databases in the object's storage
        return WorkloadRunner.start(
            {
                ...this.#runner,
                alarm,
                socketHost: {
                    url: `https://partition${SOCKET_PATH}`,
                    fetch: (request) => this.#partition.fetch(request),
                },
                ephemeral: {
                    open: async (tables) => {
                        // start the ephemeral objects' database empty in the object's storage
                        await this.#databases.drop(EPHEMERAL_NAMESPACE);

                        return this.#databases.connect(tables, { namespace: EPHEMERAL_NAMESPACE });
                    },
                    // replicate between the object's instances, such as the one a deploy replaces
                    channel: () => this.#channels(),
                },
                connectors: { [DURABLE_OBJECT_PROVIDER]: durableObjectConnector(this.#databases) },
            },
            started.start,
            build,
            startTelemetry,
            (error) => instruments.captureException(error, { isEscaped: true }),
        );
    }

    /** Move the partition's object's wake-up, or clear it. */
    async #move(at: number | null): Promise<void> {
        const response = await this.#partition.fetch(
            new Request(`https://partition${ALARM_PATH}`, {
                method: "POST",
                body: JSON.stringify({ at } satisfies DurableObjectAlarm),
            }),
        );
        await response.arrayBuffer();
        if (!response.ok) {
            throw new ObjectError("WAKE_UP_UNSTORED", {
                message: `the partition did not store the wake-up: ${response.status}`,
            });
        }
    }

    /** Drain and close the running workload. */
    async #stop(): Promise<void> {
        const running = await this.#running;
        this.#running = Promise.resolve(undefined);
        await running?.close();
    }
}
