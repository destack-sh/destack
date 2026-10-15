import type { InstanceSpec, Runtime } from "../runtime/index.ts";
import type { ServerRuntime } from "@destack/package/runtime";

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

    /** Run instances as a server runtime, bun by default. */
    constructor(name: ServerRuntime = "bun", earlier: readonly string[] = []) {
        // keep the runtime's name and the instances left running
        this.name = name;
        this.#earlier = new Set(earlier);
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
        throw new TypeError(`the test runtime serves no requests of ${instanceId}`);
    }

    /** Deliver no requests. */
    async deliver(instanceId: string): Promise<Response> {
        throw new TypeError(`the test runtime delivers nothing to ${instanceId}`);
    }
}
