import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import type { Subprocess } from "bun";
import type { Identifier } from "@destack/schema";
import { Egress, ServiceMount } from "@destack/service";
import { ServiceKind } from "@destack/service/declare";
import { CALLER_HEADER, type Caller } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { Journal, type JournalKey } from "@destack/service/database";
import { WEBHOOK_PATH, WorkloadReady, type WorkloadStart } from "@destack/service/workload";
import type { InstanceSpec, Runtime } from "./runtime.ts";

/** How long a runner may take to serve, in milliseconds: a Bun start and a workload's migrations under load. */
const START_TIMEOUT_MILLISECONDS = 10_000;

/** How long a stopping runner may drain before it is killed, in milliseconds: a service's drain timeout and cleanup. */
const STOP_TIMEOUT_MILLISECONDS = 15_000;

/** A runner process of one instance. */
interface Child {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** Report an exit nobody asked for. */
    readonly exited: (code: number) => Promise<void>;
    /** The process. */
    readonly process: Subprocess<"pipe", "pipe", "pipe">;
    /** The loopback port it serves on. */
    readonly port: number;
    /** The secret the host and the runner prove each other's requests with. */
    readonly secret: string;
}

/** Run instances as Bun processes on this host, each serving its workload on a loopback port. */
export class BunRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "bun";
    /** The directory of each instance's files. */
    readonly #directory: string;
    /** The host's egress, below which the instances reach addresses. */
    readonly #egress: string;
    /** Resolve the share of traces an installation keeps. */
    readonly #sampling: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<number>;
    /** Record a line a runner writes to standard error. */
    readonly #output: (spec: InstanceSpec, line: string) => void;
    /** Read the host's journal key. */
    readonly #journalKey: JournalKey;
    /** The running processes, by instance. */
    readonly #children = new Map<string, Child>();
    /** The running processes, by the secret they prove requests with. */
    readonly #secrets = new Map<string, Child>();

    /** Run instances below a directory, reaching addresses through the host's egress. */
    constructor(options: {
        /** The directory of each instance's files. */
        readonly directory: string;
        /** The host's egress, below which the instances reach addresses. */
        readonly egress: string;
        /** Resolve the share of traces an installation keeps, as its space's settings place it. */
        readonly sampling: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<number>;
        /** Record a line a runner writes to standard error. */
        readonly output: (spec: InstanceSpec, line: string) => void;
        /** Read the host's journal key, which each installation's own key derives from. */
        readonly journalKey: JournalKey;
    }) {
        // keep the directory, the egress, the output and the journal key
        this.#directory = options.directory;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#output = options.output;
        this.#journalKey = options.journalKey;
    }

    /** Start an instance's runner and resolve when it serves, leaving a serving one as it is. */
    async start(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void> {
        if (!this.#children.has(spec.instanceId)) {
            await this.#spawn(spec, exited);
        }
    }

    /** Report whether an instance's runner serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#children.has(instanceId);
    }

    /** Find the spec of the instance whose runner has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#secrets.get(secret)?.spec;
    }

    /** Stop an instance, draining its runner before killing it. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // forget the child, so its exit reads as asked for
        const child = this.#children.get(instanceId);
        if (child === undefined) {
            return;
        }
        this.#forget(child);

        // drain it, killing it once the drain runs out
        child.process.kill("SIGTERM");
        const isDrained = await Promise.race([
            child.process.exited.then(() => true),
            Bun.sleep(STOP_TIMEOUT_MILLISECONDS).then(() => false),
        ]);
        if (!isDrained) {
            child.process.kill("SIGKILL");
            await child.process.exited;
        }
    }

    /** Stop every instance, as the host stops. */
    async close(): Promise<void> {
        const running = [...this.#children.values()].map((child) => child.spec.instanceId);
        await Promise.all(running.map((instanceId) => this.stop(instanceId)));
    }

    /** Forward a request below an instance's service to its runner, as a verified caller. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        caller: Caller,
    ): Promise<Response> {
        // require the instance's runner
        const child = this.#children.get(instanceId);
        if (child === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `instance ${instanceId} is not running`,
            });
        }

        // forward the request below the package's mount with the verified caller
        const packageId = child.spec.build.manifest.package.id;
        const url = new URL(request.url);
        const target = `http://127.0.0.1:${child.port}${ServiceMount.path(packageId)}${path}${url.search}`;
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${child.secret}`);
        caller.forward(headers);
        headers.delete("cookie");

        return fetch(new Request(target, new Request(request, { headers, redirect: "manual" })));
    }

    /** Forward a webhook request below an instance's webhooks to its runner, which verifies it. */
    async receive(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // require the instance's runner
        const child = this.#children.get(instanceId);
        if (child === undefined) {
            throw new ServiceError("SERVICE_UNAVAILABLE", {
                message: `instance ${instanceId} is not running`,
            });
        }

        // forward the request below the runner's webhook path with the host's secret
        const url = new URL(request.url);
        const target = `http://127.0.0.1:${child.port}${WEBHOOK_PATH}${path}${url.search}`;
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${child.secret}`);
        headers.delete(CALLER_HEADER);
        headers.delete("cookie");

        return fetch(new Request(target, new Request(request, { headers, redirect: "manual" })));
    }

    /** Extract an instance's server output, start its runner, and wait until it serves. */
    async #spawn(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void> {
        // require an output carrying the workload's entry
        const output = spec.build.manifest.outputs[spec.output];
        const entrypoint = output?.workloads[spec.workload]?.entrypoint;
        const runner = entrypoint === undefined ? undefined : output?.exports[entrypoint];
        if (output === undefined || runner === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `output ${spec.output} carries no entry of workload ${spec.workload}`,
            });
        }

        // write the output's files below the instance's directory
        const directory = join(this.#directory, spec.instanceId);
        for (const file of await spec.build.inventory()) {
            if (file.path.startsWith(`${output.directory}/`)) {
                const path = join(directory, file.path);
                await mkdir(dirname(path), { recursive: true });
                await writeFile(path, await spec.build.load(file.path));
            }
        }

        // bind the running package's resources by name, reaching addresses through the egress with the instance's secret
        const secret = crypto.getRandomValues(new Uint8Array(32)).toHex();
        const start: WorkloadStart = {
            instance: spec.instanceId,
            scope: spec.scope,
            installation: spec.installationId,
            bindings: BunRuntime.#bindings(spec, this.#egress, secret),
            secret,
            egress: this.#egress,
            sampling: await this.#sampling(spec.scope, spec.installationId),
            journalKey: (
                await Journal.derive(await this.#journalKey(), spec.installationId)
            ).toHex(),
        };

        // start the runner with its input, its errors going to the host's output
        const process = Bun.spawn([globalThis.process.execPath, join(directory, runner)], {
            cwd: directory,
            stdin: "pipe",
            stdout: "pipe",
            stderr: "pipe",
            env: { PATH: globalThis.process.env.PATH },
        });
        void this.#capture(spec, process.stderr);
        void process.stdin.write(`${JSON.stringify(start)}\n`);

        // wait for its first line, killing a runner that exits or stays silent
        let port: number;
        try {
            port = await BunRuntime.#ready(process);
        } catch (error) {
            process.kill("SIGKILL");
            await process.exited;
            throw error;
        }

        // report an exit nobody asked for
        const child: Child = { spec, exited, process, port, secret: start.secret };
        this.#children.set(spec.instanceId, child);
        this.#secrets.set(child.secret, child);
        void process.exited.then((code) => this.#exited(child, code));
    }

    /** Report a runner's exit to its cell when its instance should still run. */
    async #exited(child: Child, code: number): Promise<void> {
        if (this.#children.get(child.spec.instanceId) === child) {
            this.#forget(child);
            await child.exited(code);
        }
    }

    /** Forget a runner, so that its exit reads as asked for and its secret proves nothing. */
    #forget(child: Child): void {
        this.#children.delete(child.spec.instanceId);
        this.#secrets.delete(child.secret);
    }

    /** Record each line a runner writes to standard error until the stream ends. */
    async #capture(spec: InstanceSpec, stream: ReadableStream<Uint8Array>): Promise<void> {
        // record each complete line and keep the partial rest
        const decoder = new TextDecoder();
        let rest = "";
        for await (const chunk of stream) {
            const lines = (rest + decoder.decode(chunk, { stream: true })).split("\n");
            rest = lines.pop()!;
            for (const line of lines) {
                this.#output(spec, line);
            }
        }

        // record a last line without a line break
        if (rest !== "") {
            this.#output(spec, rest);
        }
    }

    /** Bind the running package's resources by their declared names, services at the egress with the secret, and refuse unprovisioned or foreign ones. */
    static #bindings(
        spec: InstanceSpec,
        egress: string,
        secret: string,
    ): WorkloadStart["bindings"] {
        // bind each resource of the running package under its declared name
        const packageId = spec.build.manifest.package.id;
        const bindings: WorkloadStart["bindings"] = {};
        for (const bound of spec.resources) {
            // refuse a resource of another package or one not provisioned
            if (bound.packageId !== packageId || bound.reference === null) {
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: `resource ${bound.id} binds no provisioned resource of ${packageId}`,
                });
            }

            // bind a service at the host's egress with the secret, and any other resource at its reference
            const isService = bound.kind === ServiceKind.name;
            bindings[bound.name] = {
                resource: bound.id,
                kind: bound.kind,
                provider: bound.providerCode,
                reference: isService ? Egress.url(egress, bound.reference) : bound.reference,
                ...(isService ? { credential: secret } : {}),
            };
        }

        return bindings;
    }

    /** Read a runner's first output line after it serves. */
    static async #ready(process: Subprocess<"pipe", "pipe", "pipe">): Promise<number> {
        // read until the first line ends, the process exits, or the timeout passes
        const reader = process.stdout.getReader();
        const decoder = new TextDecoder();
        let line = "";
        const deadline = Bun.sleep(START_TIMEOUT_MILLISECONDS).then(() => undefined);
        try {
            while (!line.includes("\n")) {
                const chunk = await Promise.race([reader.read(), deadline]);
                if (chunk === undefined || chunk.done) {
                    throw new ServiceError("SERVICE_UNAVAILABLE", {
                        message: "the workload runner did not serve",
                    });
                }
                line += decoder.decode(chunk.value, { stream: true });
            }
        } finally {
            reader.releaseLock();
        }

        return WorkloadReady.parse(JSON.parse(line.slice(0, line.indexOf("\n")))).port;
    }
}
