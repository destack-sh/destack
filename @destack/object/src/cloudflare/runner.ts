import { LineExporter, LOG_FORMAT_VARIABLE, LOG_LEVEL_VARIABLE } from "@destack/environment";
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
import { TelemetryScope } from "@destack/telemetry";
import { startTelemetry } from "@destack/telemetry/worker";
import type {} from "@destack/package/import-meta";
import type { Alarm } from "@destack/db";

import { refusal } from "../error/report.ts";
import {
    type WorkloadRunnerOptions,
    SOCKET_PATH,
    WorkloadRunner,
    WorkloadStart,
} from "../workload/index.ts";

/** The path the hosting object posts its workload's start to. */
export const START_PATH = "/.destack/start";

/** The path where the hosting object asks its workload to drain and forget its start. */
export const STOP_PATH = "/.destack/stop";

/** The path where the hosting object rings an instance's alarm. */
export const ALARM_PATH = "/.destack/alarm";

/** The path below which an instance reads its build's files through its hosting object. */
export const BUILD_PATH = "/.destack/build";

/** The key of the stored start an instance resumes after an eviction. */
const START_KEY = "start";

/** The namespace of the database an instance has only while it runs. */
const EPHEMERAL_NAMESPACE = "ephemeral";

/** The start the hosting object hands its workload. */
export const DurableObjectStart = schema.object({
    /** The machine's start of the workload. */
    start: WorkloadStart,
    /** The manifest of the build the workload runs. */
    manifest: PackageManifest,
});
/** The start the hosting object hands its workload. */
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
    /** The object hosting the facet: its space's or its partition's. */
    readonly PARTITION: { readonly fetch: (request: Request) => Promise<Response> };
    /** The lowest level the workload prints, as its host Worker's, info when absent. */
    readonly DESTACK_LOG_LEVEL?: string;
    /** The format the workload prints in, as its host Worker's, line when absent. */
    readonly DESTACK_LOG_FORMAT?: string;
}

/** A workload instance in its installation's facet, beside its databases. */
export class DurableObjectRunner {
    /** The object's storage. */
    readonly #storage: DurableObjectRunnerStorage;
    /** The object hosting the facet: its space's or its partition's. */
    readonly #object: DurableObjectRunnerBindings["PARTITION"];
    /** What the workload runs with. */
    readonly #runner: Omit<WorkloadRunnerOptions, "ephemeral">;
    /** The object's databases. */
    readonly #databases: DurableObjectDatabaseHost;
    /** The running workload, absent before a start and after a stop. */
    #running: Promise<WorkloadRunner | undefined>;

    /** Resume the stored start before the object takes any event. */
    constructor(
        state: DurableObjectRunnerState,
        bindings: DurableObjectRunnerBindings,
        runner: Omit<WorkloadRunnerOptions, "ephemeral" | "log">,
    ) {
        // set the storage, the hosting object and the databases
        this.#storage = state.storage;
        this.#object = bindings.PARTITION;

        // print the workload's records to the isolate's console, as its host Worker's variables ask
        const log = new LineExporter(
            (line, severity) => LineExporter.console(line, severity),
            LineExporter.options({
                [LOG_LEVEL_VARIABLE]: bindings.DESTACK_LOG_LEVEL,
                [LOG_FORMAT_VARIABLE]: bindings.DESTACK_LOG_FORMAT,
            }),
        );
        this.#runner = { ...runner, log };
        this.#databases = new DurableObjectDatabaseHost(state.storage, runner.environment);

        // resume the stored start
        this.#running = state.blockConcurrencyWhile(() => this.#resume());
    }

    /** Answer the hosting object's database operations, starts, stops and rings. */
    async fetch(request: Request): Promise<Response> {
        try {
            const { pathname } = new URL(request.url);

            // run a database operation, reading content through the hosting object
            if (pathname === DATABASE_PATH) {
                return await this.#databases.answer(request, () => this.#object);
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
                await (await this.#running)?.alarm(at ?? this.#runner.environment.clock.now());

                return Response.json({});
            }

            // serve the workload
            const running = await this.#running;
            if (running === undefined) {
                throw ObjectError.of(
                    "WORKLOAD_UNAVAILABLE",
                    "workload is unavailable: none runs here",
                );
            }

            return await running.fetch(request);
        } catch (error) {
            return refusal(error, this.#runner.environment.telemetry);
        }
    }

    /** Run the stored start, forgetting one that no longer runs, such as one its host replaced. */
    async #resume(): Promise<WorkloadRunner | undefined> {
        // run nothing without a stored start
        const stored = await this.#storage.get(START_KEY);
        if (stored === undefined) {
            return undefined;
        }

        // run it, else forget it and report why, as its host hands each start a new secret
        try {
            return await this.#start(DurableObjectStart.parse(stored));
        } catch (error) {
            await this.#storage.delete(START_KEY);
            const { telemetry } = this.#runner.environment;
            TelemetryScope.of(import.meta.destack.package, telemetry).captureException(error, {
                isEscaped: true,
            });

            return undefined;
        }
    }

    /** Start the workload over the object's databases. */
    #start(started: DurableObjectStart): Promise<WorkloadRunner> {
        // read the build's files through the object hosting the facet
        const object = this.#object;
        const build = new BuildReader(
            started.manifest,
            async (path) => {
                const response = await object.fetch(
                    new Request(`https://partition${BUILD_PATH}/${path}`, {
                        headers: { authorization: `Bearer ${started.start.secret}` },
                    }),
                );
                if (!response.ok) {
                    throw ObjectError.of(
                        "BUILD_FILE_UNAVAILABLE",
                        `build file '${path}' is unavailable: the hosting object answered status ${response.status}`,
                    );
                }

                return present(response.body, `the hosting object's body of build file ${path}`);
            },
            this.#runner.environment.crypto,
        );
        // address the installation's own facet for its alarm and sockets, proving it with the start's secret
        const facet = `https://partition/${started.start.installation}`;
        const alarm: Alarm = {
            setAlarm: (at) => this.#move(facet, started.start.secret, at),
            deleteAlarm: () => this.#move(facet, started.start.secret, null),
        };

        // open the bound databases in the object's storage
        return WorkloadRunner.start(
            {
                ...this.#runner,
                alarm,
                socketHost: {
                    url: `${facet}${SOCKET_PATH}`,
                    fetch: (request) => this.#object.fetch(request),
                },
                ephemeral: {
                    open: async (tables) => {
                        // start the ephemeral objects' database empty in the object's storage
                        await this.#databases.drop(EPHEMERAL_NAMESPACE);

                        return this.#databases.connect(tables, { namespace: EPHEMERAL_NAMESPACE });
                    },
                },
                connectors: { [DURABLE_OBJECT_PROVIDER]: durableObjectConnector(this.#databases) },
            },
            started.start,
            build,
            startTelemetry,
            (error) =>
                TelemetryScope.of(
                    import.meta.destack.package,
                    this.#runner.environment.telemetry,
                ).captureException(error, { isEscaped: true }),
        );
    }

    /** Move the wake-up of the installation's facet, or clear it. */
    async #move(facet: string, secret: string, at: number | null): Promise<void> {
        const response = await this.#object.fetch(
            new Request(`${facet}${ALARM_PATH}`, {
                method: "POST",
                headers: { authorization: `Bearer ${secret}` },
                body: JSON.stringify({ at } satisfies DurableObjectAlarm),
            }),
        );
        await response.arrayBuffer();
        if (!response.ok) {
            throw ObjectError.of(
                "WAKE_UP_UNAVAILABLE",
                `wake-up is unavailable: the partition did not store it, status ${response.status}`,
            );
        }
    }

    /** Drain and close the running workload. */
    async #stop(): Promise<void> {
        const running = await this.#running;
        this.#running = Promise.resolve(undefined);
        await running?.close();
    }
}
