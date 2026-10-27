import type { Identifier } from "@destack/schema";
import { StackError } from "../error/index.ts";
import { PROCESS_RUNTIME } from "../object/index.ts";
import type { InstanceSpec, Runtime } from "./runtime.ts";

/** The machine's own process, which serves the deployments of the servers it ships in its store. */
export class ProcessRuntime implements Runtime {
    /** The runtime of deployments served in process. */
    readonly name = PROCESS_RUNTIME;
    /** The instances served in process, by instance. */
    readonly #running = new Map<Identifier<"instance">, InstanceSpec>();

    /** Record an instance served, as the machine's store serves its shipped server already. */
    async start(spec: InstanceSpec): Promise<void> {
        this.#running.set(spec.instanceId, spec);
    }

    /** Record an instance stopped, as the store stops serving its revision. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        this.#running.delete(instanceId);
    }

    /** Report whether an instance serves in process now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#running.has(instanceId);
    }

    /** Identify no instance, since in-process servers have no instance secret. */
    identify(): undefined {
        return undefined;
    }

    /** Refuse forwarding a request, which the machine's store serves without a runtime. */
    async fetch(instanceId: Identifier<"instance">): Promise<Response> {
        throw StackError.of(
            "SERVED_IN_PROCESS",
            `instance ${instanceId} serves in the machine's store, which answers its requests`,
        );
    }

    /** Refuse a delivery, which the machine's store hands its in-process server itself. */
    async deliver(instanceId: Identifier<"instance">): Promise<Response> {
        throw StackError.of(
            "SERVED_IN_PROCESS",
            `instance ${instanceId} serves in the machine's store, which takes its deliveries`,
        );
    }
}
