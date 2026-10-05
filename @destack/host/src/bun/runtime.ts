import { mkdir, realpath, writeFile } from "node:fs/promises";
import { constants } from "node:os";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";
import type { Readable } from "node:stream";
import { runtimeDirectory } from "@destack/fs";
import { MANIFEST_PATH } from "@destack/package/manifest";
import { Sandbox, type SandboxExit, type SandboxOptions } from "@destack/sandbox";
import type { Identifier } from "@destack/schema";
import { ServiceMount } from "@destack/service";
import { AUTHENTICATION_HEADER, type Authentication } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { CallKey } from "@destack/service/request";
import { WEBHOOK_PATH, WorkloadReady, type WorkloadStart } from "@destack/service/workload";
import { InstanceSpec, InstanceStarts, type Runtime } from "../runtime/runtime.ts";
import { type InstanceDirectories, WorkloadSandbox } from "./sandbox.ts";

/** How long a runner may take to serve, in milliseconds: a Bun start and a workload's migrations under load. */
const START_TIMEOUT_MILLISECONDS = 10_000;

/** How long a stopping runner may drain before it is killed, in milliseconds: a service's drain timeout and cleanup. */
const STOP_TIMEOUT_MILLISECONDS = 15_000;

/** The bytes of the secret a host and a runner prove each other's requests with: 256 bits. */
const SECRET_BYTES = 32;

/** A runner process of one instance. */
interface Child {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** Report an exit nobody asked for. */
    readonly exited: (code: number) => Promise<void>;
    /** The sandboxed process. */
    readonly sandbox: Sandbox;
    /** The loopback port it serves on. */
    readonly port: number;
    /** The secret the host and the runner prove each other's requests with. */
    readonly secret: string;
}

/** Run instances as sandboxed Bun processes on this host, each serving its workload on a loopback port. */
export class BunRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "bun";
    /** The directory of each instance's files. */
    readonly #directory: string;
    /** The host's egress, below which the instances call addresses. */
    readonly #egress: string;
    /** Resolve the share of traces an installation keeps. */
    readonly #sampling: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<number>;
    /** Record a line a runner writes to standard error. */
    readonly #output: (spec: InstanceSpec, line: string) => void;
    /** Read the host's journal key. */
    readonly #callKey: CallKey;
    /** The running processes, by instance. */
    readonly #children = new Map<string, Child>();
    /** The starts in progress, which concurrent starts of one instance join. */
    readonly #starts = new InstanceStarts();
    /** The running processes, by the secret they prove requests with. */
    readonly #secrets = new Map<string, InstanceSpec>();

    /** Run instances below a directory, calling addresses through the host's egress. */
    constructor(options: {
        /** The directory of each instance's files. */
        readonly directory: string;
        /** The host's egress, below which the instances call addresses. */
        readonly egress: string;
        /** Resolve the share of traces an installation keeps, as its space's settings place it. */
        readonly sampling: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<number>;
        /** Record a line a runner writes to standard error. */
        readonly output: (spec: InstanceSpec, line: string) => void;
        /** Read the host's journal key, which each installation's key derives from. */
        readonly callKey: CallKey;
    }) {
        // keep the directory, the egress, the output and the journal key
        this.#directory = options.directory;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#output = options.output;
        this.#callKey = options.callKey;
    }

    /** Start an instance's runner and resolve when it serves, joining a start in progress and leaving a serving one as it is. */
    async start(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void> {
        // spawn the runner unless it serves, joining a start in progress
        return this.#starts.join(spec.instanceId, async () => {
            if (!this.#children.has(spec.instanceId)) {
                await this.#spawn(spec, exited);
            }
        });
    }

    /** Report whether an instance's runner serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#children.has(instanceId);
    }

    /** Find the spec of the instance whose runner has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#secrets.get(secret);
    }

    /** Stop an instance after any start in progress, draining its runner before killing it. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // wait for a start in progress, whose caller sees its failure
        await this.#starts.settle(instanceId);

        // forget the child, so its exit reads as asked for
        const child = this.#children.get(instanceId);
        if (child === undefined) {
            return;
        }
        this.#forget(child);

        // drain it, killing it once the drain runs out
        await child.sandbox.stop(STOP_TIMEOUT_MILLISECONDS);
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
        authentication: Authentication,
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
        authentication.forward(headers);
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
        headers.delete(AUTHENTICATION_HEADER);
        headers.delete("cookie");

        return fetch(new Request(target, new Request(request, { headers, redirect: "manual" })));
    }

    /** Extract an instance's build, start its runner, and wait until it serves. */
    async #spawn(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void> {
        // confine the runner to its build, its installation's directories, its resources and the egress
        const runner = BunRuntime.#entry(spec);
        const directories = await this.#directories(spec);
        const options = WorkloadSandbox.options(spec, {
            executable: process.execPath,
            directories,
            runner,
            runtime: await realpath(await runtimeDirectory()),
            egress: this.#egress,
            environment: process.env,
            which: (command) => Bun.which(command) ?? undefined,
        });

        // write the build's files, then start the runner with its input, identifying the calls it sends while it starts
        await BunRuntime.#extract(spec, directories);
        const start = await this.#start(spec);
        this.#secrets.set(start.secret, spec);
        const { sandbox, port } = await this.#run(options, start, spec).catch((error: unknown) => {
            this.#secrets.delete(start.secret);
            throw error;
        });

        // report an exit nobody asked for, and a sandbox failing as an exit
        const child: Child = { spec, exited, sandbox, port, secret: start.secret };
        this.#children.set(spec.instanceId, child);
        void sandbox.exited.then(
            (exit) => this.#exited(child, BunRuntime.#code(exit)),
            (error: unknown) => {
                this.#output(spec, error instanceof Error ? error.message : String(error));

                return this.#exited(child, 1);
            },
        );
    }

    /** Start a runner's sandbox with its input and wait until it serves on its port. */
    async #run(
        options: SandboxOptions,
        start: WorkloadStart,
        spec: InstanceSpec,
    ): Promise<{ readonly sandbox: Sandbox; readonly port: number }> {
        // start the sandbox, write the start to its input and record what it reports
        const sandbox = await Sandbox.start(options);
        void this.#capture(spec, sandbox.stderr);
        sandbox.stdin.write(`${JSON.stringify(start)}\n`);

        return { sandbox, port: await BunRuntime.#serving(sandbox) };
    }

    /** Find the runner file of an instance's workload, refusing an output without its entry. */
    static #entry(spec: InstanceSpec): string {
        // find the workload's entry among the output's exports
        const output = spec.build.manifest.outputs[spec.output];
        const entrypoint = output?.workloads[spec.workload]?.entrypoint;
        const runner = entrypoint === undefined ? undefined : output?.exports[entrypoint];
        if (output === undefined || runner === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `output ${spec.output} carries no entry of workload ${spec.workload}`,
            });
        }

        return runner;
    }

    /** Place an instance's build directory and its installation's data and cache directories below the runtime's directory. */
    async #directories(spec: InstanceSpec): Promise<InstanceDirectories> {
        await mkdir(this.#directory, { recursive: true });
        const root = await realpath(this.#directory);

        return {
            build: join(root, spec.instanceId),
            data: join(root, spec.installationId, "data"),
            cache: join(root, spec.installationId, "cache"),
        };
    }

    /** Create an instance's directories and write its build into the build directory: the manifest at the root and every distributed file. */
    static async #extract(spec: InstanceSpec, directories: InstanceDirectories): Promise<void> {
        // create each directory
        for (const directory of [directories.build, directories.data, directories.cache]) {
            await mkdir(directory, { recursive: true });
        }

        // write the manifest, then each distributed file at its path
        const manifest = JSON.stringify(spec.build.manifest);
        await writeFile(join(directories.build, MANIFEST_PATH), manifest);
        for (const file of await spec.build.distributed()) {
            const path = join(directories.build, file.path);
            await mkdir(dirname(path), { recursive: true });
            await writeFile(path, await spec.build.load(file.path));
        }
    }

    /** Describe a runner's start with a fresh secret. */
    async #start(spec: InstanceSpec): Promise<WorkloadStart> {
        return InstanceSpec.start(spec, {
            egress: this.#egress,
            secret: crypto.getRandomValues(new Uint8Array(SECRET_BYTES)).toHex(),
            sampling: await this.#sampling(spec.scope, spec.installationId),
            callKey: this.#callKey,
        });
    }

    /** Wait for a started runner's port, killing a runner that exits or stays silent. */
    static async #serving(sandbox: Sandbox): Promise<number> {
        try {
            return await BunRuntime.#ready(sandbox.stdout);
        } catch (error) {
            // kill the runner, reporting a failing kill beside the start's failure
            const stopped = await sandbox.stop(0).then(
                () => undefined,
                (failure: unknown) => failure,
            );
            throw stopped === undefined
                ? error
                : new AggregateError([error, stopped], "a runner failed to start and to stop");
        }
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
    async #capture(spec: InstanceSpec, stream: AsyncIterable<Uint8Array>): Promise<void> {
        // record each complete line and keep the partial rest
        const decoder = new TextDecoder();
        let rest = "";
        for await (const chunk of stream) {
            const text = rest + decoder.decode(chunk, { stream: true });
            const end = text.lastIndexOf("\n");
            const lines = end === -1 ? [] : text.slice(0, end).split("\n");
            rest = text.slice(end + 1);
            for (const line of lines) {
                this.#output(spec, line);
            }
        }

        // record a last line without a line break
        if (rest !== "") {
            this.#output(spec, rest);
        }
    }

    /** Read a runner's first output line after it serves, discarding what it writes after. */
    static async #ready(stdout: Readable): Promise<number> {
        // read until the first line ends, the process exits, or the timeout passes
        const lines = createInterface({ input: stdout });
        const deadline = Bun.sleep(START_TIMEOUT_MILLISECONDS).then(() => undefined);
        try {
            const first = await Promise.race([lines[Symbol.asyncIterator]().next(), deadline]);
            if (first === undefined || first.done === true) {
                throw new ServiceError("SERVICE_UNAVAILABLE", {
                    message: "the workload runner did not serve",
                });
            }

            return WorkloadReady.parse(JSON.parse(first.value)).port;
        } finally {
            lines.close();
            stdout.resume();
        }
    }

    /** Read a sandboxed process's exit as a code, a known signal as 128 plus its number and any other as 1. */
    static #code(exit: SandboxExit): number {
        const signal = Object.entries(constants.signals).find(([name]) => name === exit.signal);
        if (exit.code !== null) {
            return exit.code;
        }

        return signal === undefined ? 1 : 128 + signal[1];
    }
}
