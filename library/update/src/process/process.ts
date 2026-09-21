import { join } from "node:path";
import { UpdateError } from "../error/error.ts";

/** Maximum time allowed for a native application to shut down. */
const STOP_TIMEOUT = 30_000;
/** Private route served by the desktop's existing HTTP server. */
const STOP_PATH = "/_destack/update/stop";

/** A desktop process registered for authenticated update shutdown. */
export class UpdateProcess implements Disposable {
    /** Lock retained until the application releases its native resources. */
    private readonly lock: Deno.FsFile;
    /** Loopback port serving the desktop application. */
    private readonly port: number;
    /** Credential available only to this user's native processes. */
    private readonly token: string;
    /** Operation that drains the application's requests and exits. */
    private readonly shutdown: () => Promise<void>;

    /** Retain a locked process and its authenticated shutdown operation. */
    private constructor(
        lock: Deno.FsFile,
        port: number,
        token: string,
        shutdown: () => Promise<void>,
    ) {
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
        await Deno.mkdir(directory, { recursive: true, mode: 0o700 });
        const lock = await Deno.open(join(directory, "desktop.lock"), {
            create: true,
            read: true,
            write: true,
            mode: 0o600,
        });
        try {
            if (!(await lock.tryLock(true))) {
                throw new UpdateError("BUSY", "Destack desktop is already running.");
            }

            // publish the endpoint atomically without exposing credentials to the webview
            const token = crypto.getRandomValues(new Uint8Array(32)).toHex();
            const temporary = join(directory, `desktop.${crypto.randomUUID()}.json`);
            await Deno.writeTextFile(temporary, JSON.stringify({ port, token }), {
                createNew: true,
                mode: 0o600,
            });
            await Deno.rename(temporary, join(directory, "desktop.json"));

            return new UpdateProcess(lock, port, token, shutdown);
        } catch (error) {
            lock.close();
            throw error;
        }
    }

    /** Handle native shutdown requests before dispatching application routes. */
    handle(request: Request): Response | undefined {
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
    [Symbol.dispose](): void {
        this.lock.close();
    }

    /** Stop a registered desktop, returning whether it was running. */
    static async stop(directory: string): Promise<boolean> {
        let lock: Deno.FsFile;
        try {
            lock = await Deno.open(join(directory, "desktop.lock"), { read: true, write: true });
        } catch (error) {
            if (error instanceof Deno.errors.NotFound) {
                return false;
            }
            throw error;
        }

        // consult the live lock before trusting a persisted endpoint
        try {
            if (await lock.tryLock(true)) {
                return false;
            }
            const endpoint = JSON.parse(await Deno.readTextFile(join(directory, "desktop.json")));
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
            const deadline = performance.now() + STOP_TIMEOUT;
            while (!(await lock.tryLock(true))) {
                if (performance.now() >= deadline) {
                    throw new UpdateError("INSTALL", "Desktop did not finish shutting down.");
                }
                await new Promise((resolve) => setTimeout(resolve, 25));
            }

            return true;
        } finally {
            lock.close();
        }
    }
}
