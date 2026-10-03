import { mkdir, realpath, writeFile } from "node:fs/promises";
import { constants, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";
import type { Readable } from "node:stream";
import type { PackageOutput } from "@destack/package/manifest";
import { Sandbox, type SandboxExit } from "@destack/sandbox";
import type { Identifier } from "@destack/schema";
import { Egress, ServiceMount } from "@destack/service";
import { ServiceKind } from "@destack/service/declare";
import { AUTHENTICATION_HEADER, type Authentication } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { CallKey } from "@destack/service/request";
import { WEBHOOK_PATH, WorkloadReady, type WorkloadStart } from "@destack/service/workload";
import type { InstanceSpec, Runtime } from "./runtime.ts";
import { WorkloadSandbox } from "./sandbox.ts";

/** How long a runner may take to serve, in milliseconds: a Bun start and a workload's migrations under load. */
const START_TIMEOUT_MILLISECONDS = 10_000;

/** How long a stopping runner may drain before it is killed, in milliseconds: a service's drain timeout and cleanup. */
const STOP_TIMEOUT_MILLISECONDS = 15_000;

/** The bytes of the secret a host and a runner prove each other's requests with: 256 bits. */
const SECRET_BYTES = 32;

/** The folders an instance runs in: its files, and its installation's data and cache. */
interface InstanceFolders {
    /** The instance's directory with the output's files. */
    readonly directory: string;
    /** The installation's data folder. */
    readonly data: string;
    /** The installation's cache folder. */
    readonly cache: string;
}

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
    /** The starts in progress, by instance. */
    readonly #starting = new Map<string, Promise<void>>();
    /** The running processes, by the secret they prove requests with. */
    readonly #secrets = new Map<string, Child>();

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
        // join a start in progress, and leave a serving runner as it is
        const instanceId = spec.instanceId;
        const pending = this.#starting.get(instanceId);
        if (pending !== undefined) {
            return pending;
        } else if (this.#children.has(instanceId)) {
            return;
        }

        // spawn the runner once, forgetting the start when it settles
        const spawning = this.#spawn(spec, exited).finally(() => this.#starting.delete(instanceId));
        this.#starting.set(instanceId, spawning);

        return spawning;
    }

    /** Report whether an instance's runner serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#children.has(instanceId);
    }

    /** Find the spec of the instance whose runner has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#secrets.get(secret)?.spec;
    }

    /** Stop an instance after any start in progress, draining its runner before killing it. */
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // wait for a start in progress, whose caller sees its failure
        await Promise.allSettled([this.#starting.get(instanceId)]);

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

    /** Extract an instance's server output, start its runner, and wait until it serves. */
    async #spawn(spec: InstanceSpec, exited: (code: number) => Promise<void>): Promise<void> {
        // confine the runner to its files, its installation's folders, its resources and the egress
        const { output, runner } = BunRuntime.#entry(spec);
        const folders = await this.#folders(spec);
        const options = WorkloadSandbox.options(spec, {
            executable: process.execPath,
            ...folders,
            runner,
            runtime: await realpath(process.env["XDG_RUNTIME_DIR"] ?? tmpdir()),
            egress: this.#egress,
            environment: process.env,
            which: (command) => Bun.which(command) ?? undefined,
        });

        // write the output's files, then start the runner with its input and wait until it serves
        await BunRuntime.#extract(spec, output.directory, folders);
        const start = await this.#start(spec);
        const sandbox = await Sandbox.start(options);
        void this.#capture(spec, sandbox.stderr);
        sandbox.stdin.write(`${JSON.stringify(start)}\n`);
        const port = await BunRuntime.#serving(sandbox);

        // report an exit nobody asked for, and a sandbox failing as an exit
        const child: Child = { spec, exited, sandbox, port, secret: start.secret };
        this.#children.set(spec.instanceId, child);
        this.#secrets.set(child.secret, child);
        void sandbox.exited.then(
            (exit) => this.#exited(child, BunRuntime.#code(exit)),
            (error: unknown) => {
                this.#output(spec, error instanceof Error ? error.message : String(error));

                return this.#exited(child, 1);
            },
        );
    }

    /** Find the output and the runner file of an instance's workload, refusing an output without its entry. */
    static #entry(spec: InstanceSpec): {
        readonly output: PackageOutput;
        readonly runner: string;
    } {
        // find the workload's entry among the output's exports
        const output = spec.build.manifest.outputs[spec.output];
        const entrypoint = output?.workloads[spec.workload]?.entrypoint;
        const runner = entrypoint === undefined ? undefined : output?.exports[entrypoint];
        if (output === undefined || runner === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: `output ${spec.output} carries no entry of workload ${spec.workload}`,
            });
        }

        return { output, runner };
    }

    /** Place an instance's directory and its installation's data and cache folders below the runtime's directory. */
    async #folders(spec: InstanceSpec): Promise<InstanceFolders> {
        await mkdir(this.#directory, { recursive: true });
        const root = await realpath(this.#directory);

        return {
            directory: join(root, spec.instanceId),
            data: join(root, spec.installationId, "data"),
            cache: join(root, spec.installationId, "cache"),
        };
    }

    /** Create an instance's folders and write the files of a build's output directory below its directory. */
    static async #extract(
        spec: InstanceSpec,
        output: string,
        folders: InstanceFolders,
    ): Promise<void> {
        // create each folder
        for (const folder of [folders.directory, folders.data, folders.cache]) {
            await mkdir(folder, { recursive: true });
        }

        // write the output's files
        for (const file of await spec.build.distributed()) {
            if (file.path.startsWith(`${output}/`)) {
                const path = join(folders.directory, file.path);
                await mkdir(dirname(path), { recursive: true });
                await writeFile(path, await spec.build.load(file.path));
            }
        }
    }

    /** Describe a runner's start: its instance, its resources by name, the egress with a fresh secret, its sampling and its journal key. */
    async #start(spec: InstanceSpec): Promise<WorkloadStart> {
        const secret = crypto.getRandomValues(new Uint8Array(SECRET_BYTES)).toHex();

        return {
            instance: spec.instanceId,
            scope: spec.scope,
            installation: spec.installationId,
            ...(spec.manifest === undefined ? {} : { manifest: spec.manifest }),
            bindings: BunRuntime.#bindings(spec, this.#egress, secret),
            secret,
            egress: this.#egress,
            sampling: await this.#sampling(spec.scope, spec.installationId),
            callKey: (await CallKey.derive(await this.#callKey(), spec.installationId)).toHex(),
        };
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
                provider: bound.provider,
                reference: isService ? Egress.url(egress, bound.reference) : bound.reference,
                ...(isService ? { credential: secret } : {}),
            };
        }

        return bindings;
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
