import type { ServerRuntime } from "@destack/package/runtime";
import type { Identifier } from "@destack/schema";
import { Egress, ServiceMount } from "@destack/service";
import type { Authentication } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { type CallKey, VERSION_HEADER } from "@destack/service/request";
import { type RunnerOptions, WEBHOOK_PATH, WorkloadRunner } from "@destack/service/workload";
import { InstanceSpec, InstanceStarts, type Runtime } from "../runtime/index.ts";

/** The bytes of the secret a host and a runner prove each other's requests with: 256 bits. */
const SECRET_BYTES = 32;

/** The origin a runner in this process answers forwarded requests at (RFC 6761 6.2). */
const RUNNER_ORIGIN = "http://runner.test";

/** A workload running in this process, with its spec. */
interface Running {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** The runner serving it. */
    readonly runner: WorkloadRunner;
}

/** Run instances' workloads in the test's own process, with the runner options each spec selects. */
export class LocalRuntime implements Runtime {
    /** The server runtime the workloads' builds target. */
    readonly name: ServerRuntime;
    /** Report a failure of a workload's background work. */
    readonly #report: (error: unknown) => void;
    /** The host's egress, below which the instances call addresses. */
    readonly #egress: string;
    /** Read the host's journal key. */
    readonly #callKey: CallKey;
    /** Select the runner options of an instance's workload. */
    readonly #runner: (spec: InstanceSpec) => RunnerOptions;
    /** The running workloads, by instance. */
    readonly #running = new Map<string, Running>();
    /** The starts in progress, which concurrent starts of one instance join. */
    readonly #starts = new InstanceStarts();
    /** The running workloads, by the secret they prove requests with. */
    readonly #secrets = new Map<string, Running>();

    /** Run workloads calling addresses through a host's egress, each with the runner options its spec selects. */
    constructor(options: {
        /** The server runtime the workloads' builds target, bun by default. */
        readonly name?: ServerRuntime;
        /** Report a failure of a workload's background work. */
        readonly report: (error: unknown) => void;
        /** The host's egress, below which the instances call addresses. */
        readonly egress: string;
        /** Read the host's journal key. */
        readonly callKey: CallKey;
        /** Select the runner options of an instance's workload. */
        readonly runner: (spec: InstanceSpec) => RunnerOptions;
    }) {
        // keep the runtime's name, the egress, the journal key and the workloads
        this.name = options.name ?? "bun";
        this.#egress = options.egress;
        this.#callKey = options.callKey;
        this.#runner = options.runner;
        this.#report = options.report;
    }

    /** Start an instance's workload in this process, joining a start in progress and leaving a running one as it is. */
    start(spec: InstanceSpec): Promise<void> {
        // start the workload unless it runs, joining a start in progress
        return this.#starts.join(spec.instanceId, async () => {
            if (!this.#running.has(spec.instanceId)) {
                await this.#start(spec);
            }
        });
    }

    /** Stop an instance after any start in progress, draining its workload. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // wait for a start in progress, whose caller sees its failure
        await this.#starts.settle(instanceId);

        // stop nothing for an instance not running
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            return;
        }

        // forget it, then drain it
        this.#running.delete(instanceId);
        this.#secrets.delete(running.runner.start.secret);
        await running.runner.close();
    }

    /** Report whether an instance's workload runs. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#running.has(instanceId);
    }

    /** Find the spec of the instance whose workload has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#secrets.get(secret)?.spec;
    }

    /** Stop every instance. */
    async close(): Promise<void> {
        for (const running of this.#running.values()) {
            await this.stop(running.spec.instanceId);
        }
    }

    /** Forward a request below an instance's service to its workload, as a verified caller. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication,
    ): Promise<Response> {
        // forward below the package's mount with the runner's secret and the verified caller
        const running = this.#require(instanceId);
        const packageId = running.spec.build.manifest.package.id;
        const url = new URL(request.url);
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${running.runner.start.secret}`);
        authentication.forward(headers);
        headers.delete("cookie");
        const target = `${RUNNER_ORIGIN}${ServiceMount.path(packageId)}${path}${url.search}`;

        return running.runner.fetch(new Request(target, new Request(request, { headers })));
    }

    /** Forward a webhook request below an instance's webhooks to its workload, which verifies it. */
    async receive(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // forward below the runner's webhook path with its secret alone
        const running = this.#require(instanceId);
        const url = new URL(request.url);
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${running.runner.start.secret}`);
        headers.delete("cookie");
        const target = `${RUNNER_ORIGIN}${WEBHOOK_PATH}${path}${url.search}`;

        return running.runner.fetch(new Request(target, new Request(request, { headers })));
    }

    /** Call an address through the host's egress as the running instance of an installation, as its workload does. */
    async call(
        installationId: Identifier<"installation">,
        address: string,
        path: string,
        init: RequestInit = {},
    ): Promise<Response> {
        // find the installation's running workload
        const running = [...this.#running.values()].find(
            (entry) => entry.spec.installationId === installationId,
        );
        if (running === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `installation ${installationId} runs no instance here`,
            });
        }

        // call with its secret, speaking its package's release
        const headers = new Headers(init.headers);
        headers.set("authorization", `Bearer ${running.runner.start.secret}`);
        headers.set(VERSION_HEADER, running.spec.build.manifest.package.version);

        return fetch(`${Egress.url(this.#egress, address)}${path}`, { ...init, headers });
    }

    /** Start an instance's workload from its start message and keep it running. */
    async #start(spec: InstanceSpec): Promise<void> {
        // start the workload from its start message
        const start = await InstanceSpec.start(spec, {
            egress: this.#egress,
            secret: crypto.getRandomValues(new Uint8Array(SECRET_BYTES)).toHex(),
            sampling: 1,
            callKey: this.#callKey,
        });
        const runner = await WorkloadRunner.start(
            this.#runner(spec),
            start,
            spec.build,
            async () => ({ shutdown: async () => {} }),
            (error) => this.#report(error),
        );
        const running = { spec, runner };
        this.#running.set(spec.instanceId, running);
        this.#secrets.set(start.secret, running);
    }

    /** Find a running workload, refusing an instance that does not run. */
    #require(instanceId: Identifier<"instance">): Running {
        const running = this.#running.get(instanceId);
        if (running === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `instance ${instanceId} is not running`,
            });
        }

        return running;
    }
}
