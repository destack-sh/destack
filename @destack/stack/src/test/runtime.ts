import type { Scheduler } from "@destack/environment";
import type { BuildReader } from "@destack/package/manifest";
import type { ServerRuntime } from "@destack/package/runtime";
import { JsonValue, present } from "@destack/schema";
import type { InstanceSpec, Runtime } from "../runtime/index.ts";

/** A runtime recording the instances it starts and stops. */
export class TestRuntime implements Runtime {
    /** The server runtime. */
    readonly name: ServerRuntime;
    /** The runs started, in order. */
    readonly started: InstanceSpec[] = [];
    /** The instances stopped, in order. */
    readonly stopped: string[] = [];
    /** The instances an earlier process of its machine left running. */
    readonly #earlier: ReadonlySet<string>;
    /** The scheduler a call's module import runs on. */
    readonly #scheduler: Scheduler;

    /** Run instances as a server runtime, bun by default, importing on a world's scheduler. */
    constructor(options: {
        /** The scheduler a call's module import runs on. */
        readonly scheduler: Scheduler;
        /** The server runtime, bun by default. */
        readonly name?: ServerRuntime;
        /** The instances an earlier process of its machine left running, none by default. */
        readonly earlier?: readonly string[];
    }) {
        // store the runtime's name, the instances left running and the scheduler
        this.name = options.name ?? "bun";
        this.#earlier = new Set(options.earlier ?? []);
        this.#scheduler = options.scheduler;
    }

    /** Record a run started. */
    async start(run: InstanceSpec): Promise<void> {
        this.started.push(run);
    }

    /** Record an instance stopped. */
    async stop(instanceId: string): Promise<void> {
        this.stopped.push(instanceId);
    }

    /** Report an instance running once started and until stopped. */
    isRunning(instanceId: string): boolean {
        return (
            (this.#earlier.has(instanceId) ||
                this.started.some((run) => run.instanceId === instanceId)) &&
            !this.stopped.includes(instanceId)
        );
    }

    /** Identify no instance. */
    identify(): undefined {
        return undefined;
    }

    /** Refuse requests. */
    async fetch(instanceId: string): Promise<Response> {
        throw new TypeError(`the test runtime serves no requests of '${instanceId}'`);
    }

    /** Deliver no requests. */
    async deliver(instanceId: string): Promise<Response> {
        throw new TypeError(`the test runtime delivers nothing to '${instanceId}'`);
    }

    /** Call an export of a build's module in this process, as the workspace installs it. */
    call(
        build: BuildReader,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        return callInProcess(build, path, name, input, this.#scheduler);
    }
}

/** Call an export of a build's module by importing its package's export in this process. */
export async function callInProcess(
    build: BuildReader,
    path: string,
    name: string,
    input: readonly JsonValue[],
    scheduler: Scheduler,
): Promise<JsonValue> {
    // find the export the module path belongs to, and import it by the package's name
    const subpath = present(
        Object.values(build.manifest.outputs)
            .flatMap((output) => Object.entries(output.exports))
            .find(([, exported]) => exported.path === path)?.[0],
        `the export of ${path}`,
    );
    const specifier = `${build.manifest.package.name}${subpath.slice(1)}`;
    const module: unknown = await scheduler.run(() => import(specifier));
    const called: unknown =
        typeof module === "object" && module !== null ? Reflect.get(module, name) : undefined;
    if (typeof called !== "function") {
        throw new TypeError(`'${specifier}' exports no function '${name}'`);
    }

    return JsonValue.of(await Reflect.apply(called, undefined, [...input]));
}
