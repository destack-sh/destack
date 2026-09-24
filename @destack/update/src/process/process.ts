import { join } from "node:path";
import { UpdateError } from "../error/error.ts";
import { mkdir, writeFile, rename, readFile } from "node:fs/promises";
import { FileLock } from "@destack/fs";
import { FileSystemError } from "@destack/fs/error";

/** Maximum time allowed for a native application to shut down. */
const STOP_TIMEOUT = 30_000;
/** Private route served by the desktop's existing HTTP server. */
const STOP_PATH = "/_destack/update/stop";

/** A desktop process registered for authenticated update shutdown. */
export class UpdateProcess implements AsyncDisposable {
    /** Lock retained until the application releases its native resources. */
    private readonly lock: FileLock;
    /** Loopback port serving the desktop application. */
    private readonly port: number;
    /** Credential available only to this user's native processes. */
    private readonly token: string;
    /** Operation that drains the application's requests and exits. */
    private readonly shutdown: () => Promise<void>;

    /** Retain a locked process and its authenticated shutdown operation. */
    private constructor(
        lock: FileLock,
        port: number,
        token: string,
        shutdown: () => Promise<void>,
    ) {
        // retain the lock, port, token and shutdown callback
        this.lock = lock;
        this.port = port;
        this.token = token;
        this.shutdown = shutdown;
    }

    /** Register the desktop's existing loopback server. */
    static async register(
        directory: string,
        port: number,
        shutdown: () => Promise<void>,
    ): Promise<UpdateProcess> {
        // serialize desktop instances with an operating-system lock
        await mkdir(directory, { recursive: true, mode: 0o700 });
        const lock = await FileLock.tryAcquire(join(directory, "desktop.lock"));
        if (!lock) {
            throw new UpdateError("BUSY", "Destack desktop is already running.");
        }
        try {
            // publish the endpoint atomically without exposing credentials to the webview
            const token = crypto.getRandomValues(new Uint8Array(32)).toHex();
            const temporary = join(directory, `desktop.${crypto.randomUUID()}.json`);
            await writeFile(temporary, JSON.stringify({ port, token }), {
                flag: "wx",
                mode: 0o600,
            });
            await rename(temporary, join(directory, "desktop.json"));

            return new UpdateProcess(lock, port, token, shutdown);
        } catch (error) {
            await lock.close();
            throw error;
        }
    }

    /** Handle native shutdown requests before dispatching application routes. */
    handle(request: Request): Response | undefined {
        // accept only authenticated local stop requests
        const url = new URL(request.url);
        if (url.pathname !== STOP_PATH) {
            return undefined;
        }
        if (
            url.origin !== `http://127.0.0.1:${this.port}` ||
            request.headers.has("origin") ||
            request.headers.get("authorization") !== `Bearer ${this.token}`
        ) {
            return new Response(null, { status: 403 });
        }
        if (request.method !== "POST") {
            return new Response(null, { status: 405 });
        }

        // let the response complete before the application drains its HTTP server
        setTimeout(() => {
            this.shutdown().catch((error) => console.error("Desktop shutdown failed:", error));
        }, 0);

        return new Response(null, { status: 202 });
    }

    /** Release registration after the application has finished shutting down. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.lock.close();
    }

    /** Stop a registered desktop, returning whether it was running. */
    static async stop(directory: string): Promise<boolean> {
        const path = join(directory, "desktop.lock");
        try {
            await using available = await FileLock.tryAcquire(path);
            if (available) {
                return false;
            }
        } catch (error) {
            if (error instanceof FileSystemError && (error.code === "ENOENT" || error.code === 3)) {
                return false;
            }
            throw error;
        }

        // consult the live lock before trusting a persisted endpoint
        {
            const endpoint = JSON.parse(await readFile(join(directory, "desktop.json"), "utf8"));
            if (
                !Number.isInteger(endpoint.port) ||
                endpoint.port < 1 ||
                endpoint.port > 65535 ||
                typeof endpoint.token !== "string" ||
                !/^[0-9a-f]{64}$/.test(endpoint.token)
            ) {
                throw new UpdateError("INSTALL", "Invalid desktop update endpoint.");
            }
            const response = await fetch(`http://127.0.0.1:${endpoint.port}${STOP_PATH}`, {
                method: "POST",
                headers: { authorization: `Bearer ${endpoint.token}` },
                redirect: "error",
                signal: AbortSignal.timeout(STOP_TIMEOUT),
            });
            await response.body?.cancel();
            if (response.status !== 202) {
                throw new UpdateError("INSTALL", `Desktop refused shutdown: ${response.status}.`);
            }

            // require resource release without terminating a persisted process identifier
            await using stopped = await FileLock.acquire(path, {
                signal: AbortSignal.timeout(STOP_TIMEOUT),
            });

            return true;
        }
    }
}
