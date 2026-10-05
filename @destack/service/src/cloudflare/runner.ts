import {
    connect,
    DATABASE_PATH,
    DURABLE_OBJECT_PROVIDER,
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
    durableObjectConnector,
} from "@destack/db/cloudflare";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { defineSchema, schema } from "@destack/schema";
import { telemetry } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/worker";
import type {} from "@destack/package/import-meta";
import type { Alarm } from "../control/index.ts";
import { ServiceError } from "../error/index.ts";
import { refusal } from "../server/error.ts";
import { type RunnerOptions, WorkloadRunner, WorkloadStart } from "../workload/index.ts";

/** The instruments capturing the failures of the workloads instances run. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The export of a workerd output's workload entry naming the class of its instances' objects. */
export const WORKLOAD_EXPORT = "Workload";

/** The path an instance's host object posts its workload's start to. */
export const START_PATH = "/.destack/start";

/** The path an instance's host object asks its workload to drain and forget its start at. */
export const STOP_PATH = "/.destack/stop";

/** The path a host object rings an instance's alarm at, and the instance moves its wake-up at. */
export const ALARM_PATH = "/.destack/alarm";

/** The path below which an instance reads its build's files through its host. */
export const BUILD_PATH = "/.destack/build";

/** The key of the start an instance keeps, which it resumes after an eviction. */
const START_KEY = "start";

/** The namespace of the database an instance keeps only while it runs, such as for its ephemeral objects. */
const EPHEMERAL_NAMESPACE = "ephemeral";

/** The start an instance's host object hands its workload: the host's start and the build's manifest. */
export const DurableObjectStart = defineSchema(
    schema.object({
        /** The host's start of the workload. */
        start: WorkloadStart,
        /** The manifest of the build the workload runs. */
        manifest: PackageManifest,
    }),
);
/** The start an instance's host object hands its workload. */
export type DurableObjectStart = schema.Infer<typeof DurableObjectStart>;

/** A ring of an instance's alarm, or a move of its wake-up. */
export const DurableObjectAlarm = defineSchema(
    schema.object({
        /** When the ring's run ends, or when the wake-up rings next, absent for none, in UTC epoch milliseconds. */
        at: schema.number().int().nullable(),
    }),
);
/** A ring of an instance's alarm, or a move of its wake-up. */
export type DurableObjectAlarm = schema.Infer<typeof DurableObjectAlarm>;

/** The storage of an instance's object: its databases and the start it keeps. */
export interface DurableObjectRunnerStorage extends DurableObjectStorage {
    /** Read a kept value. */
    get(key: string): Promise<unknown>;
    /** Keep a value. */
    put(key: string, value: unknown): Promise<void>;
    /** Forget a value. */
    delete(key: string): Promise<boolean>;
}

/** The state of an instance's object. */
export interface DurableObjectRunnerState {
    /** The object's storage. */
    readonly storage: DurableObjectRunnerStorage;
    /** Hold the object's other events until a closure settles. */
    blockConcurrencyWhile<Value>(closure: () => Promise<Value>): Promise<Value>;
}

/** The bindings an instance's object reads: the host object starting it, which keeps its alarm and serves its build. */
export interface DurableObjectRunnerEnvironment {
    /** The host object. */
    readonly HOST: { fetch(request: Request): Promise<Response> };
}

/** A workload instance in its own Durable Object, beside the databases it keeps, started, woken and served by its host's object. */
export class DurableObjectRunner {
    /** The object's storage. */
    readonly #storage: DurableObjectRunnerStorage;
    /** The host object. */
    readonly #host: DurableObjectRunnerEnvironment["HOST"];
    /** What the workload runs with. */
    readonly #runner: RunnerOptions;
    /** The databases the object keeps. */
    readonly #databases: DurableObjectDatabaseHost;
    /** The running workload, absent before a start and after a stop. */
    #running: Promise<WorkloadRunner | undefined>;

    /** Resume the kept start before the object takes any event. */
    constructor(
        state: DurableObjectRunnerState,
        environment: DurableObjectRunnerEnvironment,
        runner: RunnerOptions,
    ) {
        // keep the storage, the host object and the databases, then resume the kept start
        this.#storage = state.storage;
        this.#host = environment.HOST;
        this.#runner = runner;
        this.#databases = new DurableObjectDatabaseHost(state.storage);
        this.#running = state.blockConcurrencyWhile(() => this.#resume());
    }

    /** Answer the host object's database operations, starts, stops and rings, and serve the workload. */
    async fetch(request: Request): Promise<Response> {
        try {
            const { pathname } = new URL(request.url);

            // run a database operation
            if (pathname === DATABASE_PATH) {
                return await this.#databases.answer(request);
            }
            // keep a new start and run it
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
                throw new ServiceError("SERVICE_UNAVAILABLE", { message: "no workload runs here" });
            }

            return await running.fetch(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Start the kept start, absent before a start. */
    async #resume(): Promise<WorkloadRunner | undefined> {
        const kept = await this.#storage.get(START_KEY);

        return kept === undefined ? undefined : this.#start(DurableObjectStart.parse(kept));
    }

    /** Start the workload over the object's databases, its build and alarm kept by the host object. */
    #start(started: DurableObjectStart): Promise<WorkloadRunner> {
        // read the build's files through the host and keep the wake-up on the host's alarm
        const host = this.#host;
        const build = new BuildReader(started.manifest, async (path) => {
            const response = await host.fetch(
                new Request(`https://host${BUILD_PATH}/${path}`, {
                    headers: { authorization: `Bearer ${started.start.secret}` },
                }),
            );
            if (!response.ok) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: `the host serves no build file ${path}: ${response.status}`,
                });
            }

            return new Uint8Array(await response.arrayBuffer());
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
                ephemeral: async (tables) => {
                    // start the ephemeral objects' database empty in the object's storage
                    await this.#databases.drop(EPHEMERAL_NAMESPACE);

                    return connect(this.#storage, tables, { namespace: EPHEMERAL_NAMESPACE });
                },
                connectors: { [DURABLE_OBJECT_PROVIDER]: durableObjectConnector(this.#storage) },
            },
            started.start,
            build,
            startTelemetry,
            (error) => instruments.captureException(error, { isEscaped: true }),
        );
    }

    /** Move the host object's wake-up, or clear it. */
    async #move(at: number | null): Promise<void> {
        const response = await this.#host.fetch(
            new Request(`https://host${ALARM_PATH}`, {
                method: "POST",
                body: JSON.stringify({ at } satisfies DurableObjectAlarm),
            }),
        );
        await response.arrayBuffer();
        if (!response.ok) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `the host kept no wake-up: ${response.status}`,
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
