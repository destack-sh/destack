import { readEndpoint } from "../endpoint/index.ts";
import { localDirectory } from "../daemon/directory.ts";
import { DaemonError } from "../error/index.ts";
import { connect } from "./client.ts";
import { stat } from "node:fs/promises";
import { join } from "node:path";
import type { Status } from "../service/service.ts";

/** Local discovery results, independent of internet access and cloud sign-in. */
export type ConnectionStatus =
    | { status: "uninitialized" }
    | { status: "disconnected" }
    | { status: "connected"; daemon: Status };

/** Discover and call the local daemon. */
export class LocalClient {
    /** Private directory containing persistent state and daemon discovery. */
    readonly directory: string;

    /** Select local state without creating files or starting a process. */
    constructor(directory = localDirectory()) {
        this.directory = directory;
    }

    /** Inspect existing local state without creating files or starting the daemon. */
    async inspect(): Promise<ConnectionStatus> {
        // distinguish first launch from an existing but unavailable daemon
        try {
            await stat(join(this.directory, "daemon.db"));
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") {
                return { status: "uninitialized" };
            }
            throw error;
        }

        // retain malformed endpoint and permission errors for explicit repair
        try {
            return { status: "connected", daemon: await this.status() };
        } catch (error) {
            if (error instanceof DaemonError && error.code === "NOT_RUNNING") {
                return { status: "disconnected" };
            }
            throw error;
        }
    }

    /** Create a client that resolves current endpoint credentials for every request. */
    async connect() {
        const { port } = await readEndpoint(this.directory);

        return connect({
            url: `http://127.0.0.1:${port}`,
            fetch: async (request, options) => {
                // read the endpoint and credential together after any daemon restart
                const current = await readEndpoint(this.directory);
                const url = new URL(request.url);
                url.port = String(current.port);
                const headers = new Headers(request.headers);
                headers.set("authorization", `Bearer ${current.token}`);

                // preserve cancellation and reject redirects carrying local credentials
                try {
                    return await fetch(url.href, {
                        ...options,
                        method: request.method,
                        headers,
                        body: request.body,
                        redirect: "error",
                        signal: request.signal,
                    });
                } catch (error) {
                    if (error instanceof TypeError) {
                        throw new DaemonError("NOT_RUNNING", "cannot reach the local daemon", {
                            cause: error,
                        });
                    }
                    throw error;
                }
            },
        });
    }

    /** Read the running daemon status. */
    async status() {
        return (await this.connect()).status(undefined, { signal: AbortSignal.timeout(3000) });
    }

    /** Request graceful daemon shutdown. */
    async stop(): Promise<void> {
        await (await this.connect()).stop();
    }
}
