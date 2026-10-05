import { SandboxManager, getDefaultWritePaths } from "@anthropic-ai/sandbox-runtime";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SandboxError } from "../error/index.ts";
import { LauncherRequest, type SandboxExit, type SandboxOptions } from "../sandbox/message.ts";

/** The system files every process reads, by platform: libraries, the dynamic linker, the shell and random devices. */
export const RUNTIME_PATHS: Partial<Record<NodeJS.Platform, readonly string[]>> = {
    darwin: [
        "/System/Library",
        "/usr/lib",
        "/usr/share",
        "/bin",
        "/private/var/select/sh",
        "/dev/null",
        "/dev/urandom",
    ],
    linux: [
        "/usr",
        "/bin",
        "/lib",
        "/lib64",
        "/etc/ld.so.cache",
        "/etc/ld.so.conf",
        "/etc/ld.so.conf.d",
    ],
};

/** Run one sandbox manager under a private parent connection. */
export async function runLauncher(): Promise<void> {
    // reject direct invocation without the supervising process
    if (!process.send) {
        throw new SandboxError("START_FAILED", "sandbox launcher requires a parent connection");
    }
    const request = Promise.withResolvers<SandboxOptions>();
    const stopped = new AbortController();
    let child: ChildProcess | undefined;
    let exit: SandboxExit | undefined;
    let gracePeriod = 0;
    process.on("disconnect", () => {
        stopped.abort();
        request.reject(new SandboxError("START_FAILED", "sandbox parent disconnected"));
    });
    process.on("SIGTERM", () => stopped.abort());
    process.on("SIGINT", () => stopped.abort());
    process.on("message", (received: unknown) => {
        const message = LauncherRequest.parse(received);
        if (message.type === "start") {
            request.resolve(message.options);
        } else if (message.type === "stop") {
            gracePeriod = message.gracePeriod;
            stopped.abort();
        }
    });

    // point the manager at a fresh temporary directory
    const temporary = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-")));
    process.env["CLAUDE_CODE_TMPDIR"] = temporary;
    try {
        // require the system files of this platform
        const options = await request.promise;
        const runtime = RUNTIME_PATHS[process.platform];
        if (runtime === undefined) {
            throw new SandboxError(
                "UNSUPPORTED",
                `sandboxing is unsupported on ${process.platform}`,
            );
        }

        // configure a fresh manager with no access to other application state
        await SandboxManager.initialize(
            {
                filesystem: {
                    denyRead: ["/"],
                    allowRead: [
                        ...runtime,
                        options.executable,
                        ...options.read,
                        ...options.write,
                        temporary,
                    ],
                    allowWrite: [...options.write, temporary],
                    denyWrite: getDefaultWritePaths().filter((path) => !path.startsWith("/dev/")),
                },
                network: {
                    allowedDomains: options.network,
                    deniedDomains: [],
                    allowUnixSockets: [...(options.sockets ?? []), temporary],
                    allowLocalBinding: options.allowsListening === true,
                },
            },
            undefined,
            false,
        );
        stopped.signal.throwIfAborted();

        // quote each argument before upstream embeds the command in its shell invocation
        const arguments_ = [options.executable, ...options.arguments].map(quote).join(" ");
        const command = `export NO_PROXY= no_proxy=; exec ${arguments_}`;
        const wrapped = await SandboxManager.wrapWithSandbox(
            command,
            "/bin/sh",
            undefined,
            stopped.signal,
        );
        const environment = { ...options.environment };
        delete environment["NODE_CHANNEL_FD"];
        delete environment["NODE_CHANNEL_SERIALIZATION_MODE"];
        const workload = spawn("/bin/sh", ["-c", wrapped], {
            cwd: options.directory,
            env: environment,
            detached: true,
            stdio: ["inherit", "inherit", "inherit"],
        });
        child = workload;
        const completed = new Promise<SandboxExit>((resolve) => {
            workload.once("exit", (code, signal) => resolve({ code, signal }));
        });
        await once(workload, "spawn");
        process.send?.({ type: "ready" });

        // terminate the process group when the parent stops or disconnects
        let timeout: ReturnType<typeof setTimeout> | undefined;
        const stop = () => {
            signalGroup(workload, "SIGTERM");
            timeout = setTimeout(() => signalGroup(workload, "SIGKILL"), gracePeriod);
        };
        stopped.signal.addEventListener("abort", stop, { once: true });
        if (stopped.signal.aborted) {
            stop();
        }
        exit = await completed;
        clearTimeout(timeout);
        stopped.signal.removeEventListener("abort", stop);
        signalGroup(workload, "SIGKILL");
    } catch (error) {
        if (child) {
            signalGroup(child, "SIGKILL");
        }
        if (process.connected) {
            process.send?.({
                type: "error",
                message: error instanceof Error ? error.message : String(error),
            });
        }
        process.exitCode = 1;
    } finally {
        // report completion only after releasing proxies and temporary storage
        try {
            try {
                await SandboxManager.reset();
            } finally {
                await rm(temporary, { recursive: true });
            }
            if (exit && process.connected) {
                process.send?.({ type: "exit", exit });
            }
        } finally {
            if (process.connected) {
                process.disconnect?.();
            }
        }
    }
}

/** Preserve one literal shell argument. */
function quote(value: string): string {
    return `'${value.replaceAll("'", "'\\''")}'`;
}

/** Signal the workload process group, accepting an already terminated group. */
function signalGroup(child: ChildProcess, signal: NodeJS.Signals): void {
    // leave exited workloads alone, since their process group id may already be reused
    if (child.pid === undefined || child.exitCode !== null || child.signalCode !== null) {
        return;
    }
    try {
        process.kill(-child.pid, signal);
    } catch (error) {
        // accept gone groups, which macOS reports as not permitted for exited, unreaped workloads
        const isGone =
            error instanceof Error &&
            "code" in error &&
            (error.code === "ESRCH" || error.code === "EPERM");
        if (!isGone) {
            throw error;
        }
    }
}
