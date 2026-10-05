import { DATABASE_PATH } from "@destack/db/cloudflare";
import { defineSchema, schema } from "@destack/schema";
import {
    ALARM_PATH,
    BUILD_PATH,
    DurableObjectAlarm,
    DurableObjectStart,
    START_PATH,
    STOP_PATH,
    WORKLOAD_EXPORT,
} from "@destack/service/cloudflare";
import { ServiceError } from "@destack/service/error";
import { refusal } from "@destack/service/server";

/** How long an alarm runs the workload's controllers at most: 10 minutes, below the 15 minutes a Durable Object's alarm may run. */
const ALARM_MILLISECONDS = 10 * 60_000;

/** The workerd release the workloads' code runs on, as the cell's Workers pin it. */
const COMPATIBILITY_DATE = "2026-07-30";

/** The facet running the instance's workload and keeping its databases. */
const FACET = "workload";

/** The key of the start the object keeps, which it resumes after an eviction. */
const START_KEY = "instance";

/** The class of the static object answering database operations before a workload first starts, as the Worker exports it. */
const DATABASE_EXPORT = "DurableObjectDatabase";

/** A start of an instance's workload: its start, the build's modules it loads, and the zone it calls out through. */
export const DurableObjectInstanceStart = defineSchema(
    schema.object({
        /** The workload's start and its build's manifest. */
        workload: DurableObjectStart,
        /** The zone whose object serves the instance's egress and build files. */
        zone: schema.string().min(1),
        /** The output's directory within the build, which module names are relative to. */
        directory: schema.string().min(1),
        /** The module exporting the workload's class, relative to the output's directory. */
        main: schema.string().min(1),
        /** The output's modules, relative to its directory. */
        modules: schema.array(schema.string().min(1)).min(1),
    }),
);
/** A start of an instance's workload. */
export type DurableObjectInstanceStart = schema.Infer<typeof DurableObjectInstanceStart>;

/** A Durable Object namespace, as a Worker reaches its objects. */
export interface DurableObjectNamespace {
    /** Derive the identifier of the object of a name. */
    idFromName(name: string): unknown;
    /** Reach an object. */
    get(id: unknown): { fetch(request: Request): Promise<Response> };
    /** Narrow the namespace to the objects kept in a jurisdiction. */
    jurisdiction(name: "eu"): DurableObjectNamespace;
}

/** The loader of a workload's code into an isolate of its own. */
export interface WorkerLoader {
    /** Load code under a name once, keeping its isolate while it lives. */
    get(
        name: string,
        code: () => Promise<{
            readonly compatibilityDate: string;
            readonly compatibilityFlags: readonly string[];
            readonly mainModule: string;
            readonly modules: Readonly<Record<string, string>>;
            readonly env: Readonly<Record<string, unknown>>;
            readonly globalOutbound: { fetch(request: Request): Promise<Response> };
        }>,
    ): { getDurableObjectClass(name: string): unknown };
}

/** The state of an instance's object. */
export interface DurableObjectInstanceState {
    /** The object's identifier, which reaches the object itself. */
    readonly id: unknown;
    /** The object's storage: its kept start and its wake-up. */
    readonly storage: {
        /** Read a kept value. */
        get(key: string): Promise<unknown>;
        /** Keep a value. */
        put(key: string, value: unknown): Promise<void>;
        /** Forget a value. */
        delete(key: string): Promise<boolean>;
        /** Wake the object at a time. */
        setAlarm(at: number): Promise<void>;
        /** Wake the object at no time. */
        deleteAlarm(): Promise<void>;
    };
    /** The object's facets, each an object of its own beside it with storage of its own. */
    readonly facets: {
        /** Reach a facet, starting it with a class when it does not run. */
        get(
            name: string,
            options: () => { readonly class: unknown } | Promise<{ readonly class: unknown }>,
        ): { fetch(request: Request): Promise<Response> };
        /** Stop a facet, keeping its storage. */
        abort(name: string, reason: unknown): void;
    };
    /** The Worker's own exports, such as the static object answering database operations. */
    readonly exports: Readonly<Record<string, unknown>>;
}

/** The bindings an instance's object reads. */
export interface DurableObjectInstanceEnvironment {
    /** The loader of the workloads' code. */
    readonly LOADER: WorkerLoader;
    /** The instances' objects, reaching this object itself. */
    readonly INSTANCE: DurableObjectNamespace;
    /** The zones' objects, serving the instances' egress and build files. */
    readonly ZONE: DurableObjectNamespace;
    /** The jurisdiction keeping the zones' objects, empty outside one. */
    readonly DESTACK_JURISDICTION: "" | "eu";
}

/** The Durable Object of an installation in a space: its workload's code loaded into a facet keeping the installation's databases, its alarm, egress and build through its zone. */
export class DurableObjectInstance {
    /** The object's state. */
    readonly #state: DurableObjectInstanceState;
    /** The Worker's bindings. */
    readonly #environment: DurableObjectInstanceEnvironment;

    /** Keep the object's state and bindings. */
    constructor(state: DurableObjectInstanceState, environment: DurableObjectInstanceEnvironment) {
        this.#state = state;
        this.#environment = environment;
    }

    /** Take a start, a stop or a database operation from the zone, a wake-up or build read from the workload, or a request for the workload. */
    async fetch(request: Request): Promise<Response> {
        try {
            const { pathname } = new URL(request.url);

            // keep a start and load its workload's code
            if (pathname === START_PATH) {
                const started = DurableObjectInstanceStart.parse(await request.json());
                await this.#stop();
                await this.#state.storage.put(START_KEY, started);

                return await this.#facet(started).fetch(
                    new Request(`https://workload${START_PATH}`, {
                        method: "POST",
                        body: JSON.stringify(started.workload),
                    }),
                );
            }
            // drain the workload and forget its start
            else if (pathname === STOP_PATH) {
                await this.#stop();

                return Response.json({});
            }
            // move the wake-up the workload asks for
            else if (pathname === ALARM_PATH) {
                const { at } = DurableObjectAlarm.parse(await request.json());
                await (at === null
                    ? this.#state.storage.deleteAlarm()
                    : this.#state.storage.setAlarm(at));

                return Response.json({});
            }
            // read a build file through the zone serving it
            else if (pathname.startsWith(`${BUILD_PATH}/`)) {
                const started = await this.#started();
                if (started === undefined) {
                    throw new ServiceError("NOT_FOUND", { message: "no workload runs here" });
                }

                return await this.#zone(started).fetch(request);
            }

            // hand a database operation or a request to the facet
            const started = await this.#started();
            if (started === undefined && pathname !== DATABASE_PATH) {
                throw new ServiceError("SERVICE_UNAVAILABLE", { message: "no workload runs here" });
            }

            return await this.#facet(started).fetch(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Run the workload's due controllers for at most ten minutes, keeping the alarm due while keys still run. */
    async alarm(): Promise<void> {
        // ring nothing before a start
        const started = await this.#started();
        if (started === undefined) {
            return;
        }

        // run the workload's controllers until the ring's deadline
        const ring: DurableObjectAlarm = { at: Date.now() + ALARM_MILLISECONDS };
        const response = await this.#facet(started).fetch(
            new Request(`https://workload${ALARM_PATH}`, {
                method: "POST",
                body: JSON.stringify(ring),
            }),
        );
        await response.arrayBuffer();
    }

    /** Read the kept start, absent before a start and after a stop. */
    async #started(): Promise<DurableObjectInstanceStart | undefined> {
        const kept = await this.#state.storage.get(START_KEY);

        return kept === undefined ? undefined : DurableObjectInstanceStart.parse(kept);
    }

    /** Drain the running workload and stop its facet, keeping its databases. */
    async #stop(): Promise<void> {
        // drain the workload of the kept start
        const started = await this.#started();
        if (started !== undefined) {
            const response = await this.#facet(started).fetch(
                new Request(`https://workload${STOP_PATH}`, { method: "POST" }),
            );
            await response.arrayBuffer();
        }

        // forget the start and stop the facet
        await this.#state.storage.delete(START_KEY);
        this.#state.facets.abort(FACET, new Error("the instance's start changed"));
    }

    /** Reach the facet: the workload's code of the kept start, or the static object answering database operations without one. */
    #facet(started: DurableObjectInstanceStart | undefined): {
        fetch(request: Request): Promise<Response>;
    } {
        return this.#state.facets.get(FACET, () => ({
            class:
                started === undefined
                    ? this.#state.exports[DATABASE_EXPORT]
                    : this.#load(started).getDurableObjectClass(WORKLOAD_EXPORT),
        }));
    }

    /** Load a start's workload code into an isolate of its own, reaching this object and calling out only through the zone. */
    #load(started: DurableObjectInstanceStart): ReturnType<WorkerLoader["get"]> {
        // name the isolate by the instance, whose code and object it keeps
        const environment = this.#environment;
        const self = environment.INSTANCE.get(this.#state.id);
        const zone = this.#zone(started);
        const { secret, instance } = started.workload.start;

        return environment.LOADER.get(instance, async () => {
            // read each module through the zone serving the build
            const modules: Record<string, string> = {};
            for (const module of started.modules) {
                const response = await zone.fetch(
                    new Request(`https://zone${BUILD_PATH}/${started.directory}/${module}`, {
                        headers: { authorization: `Bearer ${secret}` },
                    }),
                );
                if (!response.ok) {
                    throw new ServiceError("SERVICE_UNAVAILABLE", {
                        message: `the zone serves no module ${module}: ${response.status}`,
                    });
                }
                modules[module] = await response.text();
            }

            return {
                compatibilityDate: COMPATIBILITY_DATE,
                compatibilityFlags: ["nodejs_compat"],
                mainModule: started.main,
                modules,
                env: { HOST: self },
                globalOutbound: zone,
            };
        });
    }

    /** Reach the zone's object, in its jurisdiction. */
    #zone(started: DurableObjectInstanceStart): { fetch(request: Request): Promise<Response> } {
        const environment = this.#environment;
        const namespace =
            environment.DESTACK_JURISDICTION === ""
                ? environment.ZONE
                : environment.ZONE.jurisdiction(environment.DESTACK_JURISDICTION);

        return namespace.get(namespace.idFromName(started.zone));
    }
}
