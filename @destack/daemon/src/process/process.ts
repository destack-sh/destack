import { spawn } from "node:child_process";
import { createDirectory } from "../daemon/directory.ts";
import { open } from "node:fs/promises";
import { dirname, join } from "node:path";
import { LocalClient } from "../client/local.ts";
import { DaemonError } from "../error/index.ts";
import { fileURLToPath } from "node:url";
import { FileLock } from "@destack/fs";

/** Operating-system values needed by the daemon, excluding application credentials. */
const ENVIRONMENT = [
    "HOME",
    "USERPROFILE",
    "PATH",
    "SystemRoot",
    "SYSTEMROOT",
    "WINDIR",
    "LOCALAPPDATA",
    "APPDATA",
    "TMPDIR",
    "TMP",
    "TEMP",
    "LANG",
    "LC_ALL",
];

/** Allow request draining plus database, audit and telemetry cleanup. */
const STOP_TIMEOUT_MS = 15000;

/** Start a detached daemon and wait until it accepts requests. */
export async function start(
    executable = Bun.isStandaloneExecutable
        ? join(
              dirname(process.execPath),
              process.platform === "win32" ? "destack-daemon.exe" : "destack-daemon",
          )
        : process.execPath,
    client = new LocalClient(),
    isStandalone = Bun.isStandaloneExecutable,
): Promise<void> {
    // reuse an authenticated daemon that is already running
    try {
        await client.status();
        return;
    } catch (error) {
        if (!isUnavailable(error)) {
            throw error;
        }
    }

    // serialize concurrent client launches and recheck after acquiring ownership
    await createDirectory(client.directory);
    const startup = await FileLock.acquire(join(client.directory, "daemon.start.lock"), {
        signal: AbortSignal.timeout(10000),
    });
    try {
        try {
            await client.status();

            return;
        } catch (error) {
            if (!isUnavailable(error)) {
                throw error;
            }
        }
        await launch(executable, client, isStandalone);
    } finally {
        await startup.close();
    }
}

/** Launch one detached process while its caller holds the startup lock. */
async function launch(
    executable: string,
    client: LocalClient,
    isStandalone: boolean,
): Promise<void> {
    // preserve the daemon output independently of the launching terminal
    const environment: Record<string, string> = { DESTACK_DIRECTORY: client.directory };
    for (const name of ENVIRONMENT) {
        const value = process.env[name];
        if (value !== undefined) {
            environment[name] = value;
        }
    }
    const log = await open(join(client.directory, "daemon.log"), "a", 0o600);
    const arguments_ = !isStandalone
        ? ["run", "--no-env-file", fileURLToPath(new URL("../main.ts", import.meta.url))]
        : [];
    const child = spawn(executable, arguments_, {
        detached: true,
        stdio: ["ignore", log.fd, log.fd],
        env: environment,
    });
    try {
        await new Promise<void>((resolve, reject) => {
            child.once("spawn", resolve);
            child.once("error", reject);
        });
    } finally {
        await log.close();
    }
    child.unref();

    // bound startup while the child publishes its endpoint
    const deadline = performance.now() + 5000;
    while (true) {
        try {
            await client.status();
            return;
        } catch (error) {
            if (!isUnavailable(error)) {
                throw error;
            }
            if (
                child.exitCode !== null ||
                child.signalCode !== null ||
                performance.now() >= deadline
            ) {
                child.kill("SIGTERM");
                throw new DaemonError(
                    "START_FAILED",
                    `daemon did not start; read ${client.directory}/daemon.log`,
                    { cause: error },
                );
            }
        }
        await new Promise((resolve) => setTimeout(resolve, 50));
    }
}

/** Stop the daemon and wait until it releases its database and process lock. */
export async function stop(client = new LocalClient()): Promise<void> {
    // request shutdown through the authenticated endpoint
    await client.stop();
    // observe completion without forcing a process to exit
    try {
        const lock = await FileLock.acquire(join(client.directory, "daemon.lock"), {
            signal: AbortSignal.timeout(STOP_TIMEOUT_MS),
        });
        await lock.close();
    } catch (cause) {
        if (
            cause instanceof Error &&
            (cause.name === "TimeoutError" || cause.name === "AbortError")
        ) {
            throw new DaemonError("STOP_FAILED", "daemon did not stop within fifteen seconds", {
                cause,
            });
        }
        throw cause;
    }
}

/** Recognize connection failures retained by the service client's transport error. */
export function isUnavailable(error: unknown): boolean {
    if (error instanceof DaemonError) {
        return error.code === "NOT_RUNNING";
    }
    if (error instanceof Error && error.cause !== undefined) {
        return isUnavailable(error.cause);
    }

    return false;
}
