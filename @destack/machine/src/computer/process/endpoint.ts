import type { Select } from "@destack/db";
import { join } from "node:path";
import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { type PackageHandle, PackageMount } from "@destack/object";
import { type Bookmark, type ClientOptions, createClient } from "@destack/object/client";
import { type Environment, FileSystemError } from "@destack/environment";
import { RequestId } from "@destack/sync";
import machinePackage from "../../package.ts";
import { MachineError } from "../../error/index.ts";
import type { computer } from "../device/object/index.ts";

/** The file a running machine publishes its endpoint in, within its directory. */
export const ENDPOINT_FILE = "machine.json";

/** How long a status read may take, in milliseconds: a loopback round trip under load. */
const STATUS_TIMEOUT_MILLISECONDS = 3000;

/** The file a running machine locks, within its directory. */
export const LOCK_FILE = "machine.lock";

/** The authenticated loopback endpoint one machine process publishes. */
export const LoopbackEndpoint = schema.object({
    /** The IPv4 loopback port the machine mounts its packages at. */
    port: schema.number().int().min(1).max(65535),
    /** The owner's credential, replaced after each restart. */
    token: schema.string().regex(/^[0-9a-f]{64}$/u),
    /** The machine the process runs, whose identifier its computer record has. */
    machine: schema.identifier("machine"),
    /** The computer's device, whose partition stores its computer record. */
    device: schema.identifier("device"),
});

/** The authenticated loopback endpoint one machine process publishes. */
export type LoopbackEndpoint = schema.Infer<typeof LoopbackEndpoint>;

/** The owner's loopback connection to the machine running in a state directory. */
export class Loopback {
    /** The machine's state directory, where it publishes its endpoint. */
    readonly directory: string;
    /** The environment reading the endpoint and sending requests. */
    readonly environment: Environment<"files">;
    /** The positions the owner's calls saw, which later calls wait for. */
    readonly bookmark: Bookmark;

    /** Connect to the machine of a state directory. */
    constructor(directory: string, environment: Environment<"files">, bookmark: Bookmark) {
        this.directory = directory;
        this.environment = environment;
        this.bookmark = bookmark;
    }

    /** Read the current endpoint without creating or repairing machine state. */
    async read(): Promise<LoopbackEndpoint> {
        // read the published file
        let text: string;
        try {
            text = await this.environment.files.readTextFile(join(this.directory, ENDPOINT_FILE));
        } catch (cause) {
            if (FileSystemError.is(cause, "NOT_FOUND")) {
                throw MachineError.of("MACHINE_STOPPED", "the Destack machine is not running", {
                    cause,
                });
            }
            throw cause;
        }

        // report corrupt discovery separately from a stopped machine
        try {
            return LoopbackEndpoint.parse(JSON.parse(text));
        } catch (cause) {
            throw MachineError.of("ENDPOINT_INVALID", "invalid local machine endpoint", {
                cause,
            });
        }
    }

    /** Create a client of a package the machine mounts on loopback, as its owner. */
    connect<Target extends PackageHandle>(target: Target) {
        return createClient<Target["router"]>(target, this.options(target.id), this.environment);
    }

    /** Read the client options of a package the machine mounts on loopback, as its owner. */
    options(packageId: PackageId): ClientOptions {
        return {
            url: `http://127.0.0.1${PackageMount.path(packageId)}`,
            fetch: (request, options) => this.#send(request, options),
            bookmark: this.bookmark,
        };
    }

    /** Read the computer the running machine records, failing once it does not answer in time. */
    async computer(): Promise<Select<typeof computer.table>> {
        const { machine, device } = await this.read();
        const signal = this.environment.clock.timeout(STATUS_TIMEOUT_MILLISECONDS);

        return this.connect(machinePackage).computer.get(
            { deviceId: device, id: machine },
            { signal },
        );
    }

    /** Ask the running machine to stop. */
    async stop(): Promise<void> {
        const { machine, device } = await this.read();
        await this.connect(machinePackage).computer.stop({
            deviceId: device,
            id: machine,
            requestId: RequestId.create(this.environment),
        });
    }

    /** Send a request to the running machine's current port with its current credential. */
    async #send(request: Request, options: RequestInit | undefined): Promise<Response> {
        // read the endpoint and credential together after any restart
        const current = await this.read();
        const url = new URL(request.url);
        url.port = String(current.port);
        const headers = new Headers(request.headers);
        headers.set("authorization", `Bearer ${current.token}`);

        // pass cancellation and refuse redirects with the owner's credential
        try {
            return await this.environment.network.fetch(
                new Request(url.href, {
                    ...options,
                    method: request.method,
                    headers,
                    body: request.body,
                    redirect: "error",
                    signal: request.signal,
                }),
            );
        } catch (error) {
            // refuse a call nothing listens for, or whose connection the machine closed, as a stopped machine
            const code = error instanceof TypeError && "code" in error ? error.code : undefined;
            if (code === "ConnectionRefused" || code === "ConnectionClosed") {
                throw MachineError.of("MACHINE_STOPPED", "cannot reach the local machine", {
                    cause: error,
                });
            }
            throw error;
        }
    }
}
