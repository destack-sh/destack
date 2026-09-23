import { SandboxManager, getDefaultWritePaths } from "@anthropic-ai/sandbox-runtime";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SandboxError } from "../error/index.ts";
import type { SandboxExit, SandboxOptions } from "../sandbox/index.ts";

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
    let gracePeriodMs = 0;
    process.on("disconnect", () => {
        stopped.abort();
        request.reject(new SandboxError("START_FAILED", "sandbox parent disconnected"));
    });
    process.on("SIGTERM", () => stopped.abort());
    process.on("SIGINT", () => stopped.abort());
    process.on("message", (message: LauncherRequest) => {
        if (message.type === "start") {
            request.resolve(message.options);
        } else if (message.type === "stop") {
            gracePeriodMs = message.gracePeriodMs;
            stopped.abort();
        }
    });

    const temporary = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-")));
    process.env.CLAUDE_CODE_TMPDIR = temporary;
    try {
        // configure a fresh manager with no access to other application state
        const options = await request.promise;
        await SandboxManager.initialize(
            {
                filesystem: {
                    denyRead: ["/"],
                    allowRead: [
                        "/System/Library",
                        "/usr/lib",
                        "/usr/share",
                        "/bin",
                        "/private/var/select/sh",
                        "/dev/null",
                        "/dev/urandom",
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
                    allowUnixSockets: options.sockets ?? [],
                    allowLocalBinding: false,
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
        delete environment.NODE_CHANNEL_FD;
        delete environment.NODE_CHANNEL_SERIALIZATION_MODE;
        child = spawn("/bin/sh", ["-c", wrapped], {
            cwd: options.directory,
            env: environment,
            detached: true,
            stdio: ["ignore", "inherit", "inherit"],
        });
        const completed = once(child, "exit");
        await once(child, "spawn");
        process.send?.({ type: "ready" });

        // terminate the process group when the parent stops or disconnects
        let timeout: ReturnType<typeof setTimeout> | undefined;
        const stop = () => {
            signalGroup(child!, "SIGTERM");
            timeout = setTimeout(() => signalGroup(child!, "SIGKILL"), gracePeriodMs);
        };
        stopped.signal.addEventListener("abort", stop, { once: true });
        if (stopped.signal.aborted) {
            stop();
        }
        const [code, signal] = await completed;
        clearTimeout(timeout);
        stopped.signal.removeEventListener("abort", stop);
        signalGroup(child, "SIGKILL");
        exit = { code, signal };
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

/** Parent commands accepted by one launcher. */
type LauncherRequest =
    | {
          /** Start with fixed permissions. */
          type: "start";
          /** Authorized launch inputs. */
          options: SandboxOptions;
      }
    | {
          /** Stop this workload. */
          type: "stop";
          /** Graceful shutdown duration in milliseconds. */
          gracePeriodMs: number;
      };

/** Preserve one literal shell argument. */
function quote(value: string): string {
    return `'${value.replaceAll("'", "'\\''")}'`;
}

/** Signal the workload process group, accepting an already terminated group. */
function signalGroup(child: ChildProcess, signal: NodeJS.Signals): void {
    if (!child.pid) {
        return;
    }
    try {
        process.kill(-child.pid, signal);
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ESRCH") {
            throw error;
        }
    }
}
