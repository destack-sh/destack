import type { Caller } from "@destack/access";
import type { Identifier } from "@destack/schema";
import { Egress } from "@destack/runtime";
import { CallError } from "@destack/runtime/error";
import type { Deriver } from "@destack/identity";
import { VERSION_HEADER } from "@destack/runtime/request";
import { type RunnerOptions, WorkloadRunner } from "@destack/runtime/workload";
import { bunEphemeral } from "../bun/runner.ts";
import { InstanceRegistry, InstanceSpec, type Runtime, WorkloadRequest } from "../runtime/index.ts";

/** The origin a runner in this process answers forwarded requests at (RFC 6761 6.2). */
const RUNNER_ORIGIN = "http://runner.test";

/** A workload running in this process, with its spec. */
interface Running {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** The runner serving it. */
    readonly runner: WorkloadRunner;
    /** The secret the machine and the runner prove each other's requests with. */
    readonly secret: string;
}

/** Run instances' workloads in the test's own process, with each spec's runner options. */
export class LocalRuntime implements Runtime {
    /** The server runtime the workloads' builds target. */
    readonly name = "bun";
    /** Report a failure of a workload's background work. */
    readonly #report: (error: unknown) => void;
    /** The machine's egress of a space's instances, below which they call addresses. */
    readonly #egress: (scope: Identifier<"space">) => string;
    /** Read the identity of a space, which its installations' roots derive from. */
    readonly #identity: (scope: Identifier<"space">) => Promise<Pick<Deriver, "derive">>;
    /** Select the runner options of an instance's workload. */
    readonly #runner: (spec: InstanceSpec) => Omit<RunnerOptions, "ephemeral">;
    /** The workloads starting and running. */
    readonly #registry = new InstanceRegistry<Running>();

    /** Run workloads calling addresses through a machine's egress. */
    constructor(options: {
        /** Report a failure of a workload's background work. */
        readonly report: (error: unknown) => void;
        /** The machine's egress of a space's instances, below which they call addresses. */
        readonly egress: (scope: Identifier<"space">) => string;
        /** Read the identity of a space, which its installations' roots derive from. */
        readonly identity: (scope: Identifier<"space">) => Promise<Pick<Deriver, "derive">>;
        /** Select the runner options of an instance's workload. */
        readonly runner: (spec: InstanceSpec) => Omit<RunnerOptions, "ephemeral">;
    }) {
        // keep the egress, the space identities and the workloads
        this.#egress = options.egress;
        this.#identity = options.identity;
        this.#runner = options.runner;
        this.#report = options.report;
    }

    /** Start an instance's workload in this process. */
    start(spec: InstanceSpec): Promise<void> {
        return this.#registry.start(spec, (identify) => this.#start(spec, identify));
    }

    /** Stop an instance after any start in progress, draining its workload. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        await (await this.#registry.stop(instanceId))?.runner.close();
    }

    /** Report whether an instance's workload runs. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#registry.isRunning(instanceId);
    }

    /** Find the spec of the instance whose workload has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#registry.identify(secret);
    }

    /** Stop every instance. */
    async close(): Promise<void> {
        for (const running of this.#registry.list()) {
            await this.stop(running.spec.instanceId);
        }
    }

    /** Forward a request below an instance's installation to its workload as a verified caller. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        caller: Caller,
    ): Promise<Response> {
        // forward at the path below the installation with the runner's secret and the verified caller
        const { runner, secret } = this.#registry.require(instanceId);

        return runner.fetch(WorkloadRequest.forward(RUNNER_ORIGIN, path, request, secret, caller));
    }

    /** Hand an instance's workload a request the machine makes at a path below its origin. */
    async deliver(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // hand it over at its path with the runner's secret alone
        const { runner, secret } = this.#registry.require(instanceId);

        return runner.fetch(WorkloadRequest.forward(RUNNER_ORIGIN, path, request, secret));
    }

    /** Call an address through the machine's egress as an installation's running instance. */
    async call(
        installationId: Identifier<"installation">,
        address: string,
        path: string,
        init: RequestInit = {},
    ): Promise<Response> {
        // find the installation's running workload
        const running = this.#registry
            .list()
            .find((entry) => entry.spec.installationId === installationId);
        if (running === undefined) {
            throw new CallError("SERVICE_UNAVAILABLE", {
                message: `installation ${installationId} runs no instance here`,
            });
        }

        // call with its secret
        const headers = new Headers(init.headers);
        headers.set("authorization", `Bearer ${running.secret}`);
        headers.set(VERSION_HEADER, running.spec.build.manifest.package.version);

        // call through the egress of the instance's space
        const egress = this.#egress(running.spec.scope);

        return fetch(`${Egress.url(egress, address)}${path}`, { ...init, headers });
    }

    /** Start an instance's workload from its start message, identifying its secret. */
    async #start(spec: InstanceSpec, identify: (secret: string) => void): Promise<Running> {
        // describe the start with a fresh secret
        const start = await InstanceSpec.start(spec, {
            egress: this.#egress(spec.scope),
            sampling: 1,
            identity: await this.#identity(spec.scope),
        });
        identify(start.secret);

        // run the workload in this process
        const runner = await WorkloadRunner.start(
            { ...this.#runner(spec), ephemeral: bunEphemeral },
            start,
            spec.build,
            async () => ({ shutdown: async () => {} }),
            (error) => this.#report(error),
        );

        return { spec, runner, secret: start.secret };
    }
}
