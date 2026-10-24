import { SandboxManager, getDefaultWritePaths } from "@anthropic-ai/sandbox-runtime";
import {
    type Environment,
    type OperatingSystem,
    type Process,
    ProcessError,
    type Signal,
    type Subprocess,
} from "@destack/environment";
import { SandboxError } from "../error/index.ts";
import { LauncherRequest, type SandboxExit, type SandboxOptions } from "../sandbox/message.ts";

/** The system files every process reads, by platform. */
export const RUNTIME_PATHS: Partial<Record<OperatingSystem, readonly string[]>> = {
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
export async function runLauncher(environment: Environment<"process" | "host">): Promise<void> {
    // reject direct invocation without the supervising process and its temporary directory
    const { host, process } = environment;
    const { parent } = process;
    const temporary = process.variables["CLAUDE_CODE_TMPDIR"];
    if (parent === undefined || temporary === undefined) {
        throw SandboxError.of("START_FAILED", "sandbox launcher requires a parent connection");
    }
    const request = Promise.withResolvers<SandboxOptions>();
    const stopped = new AbortController();
    let child: Subprocess | undefined;
    let exit: SandboxExit | undefined;
    let gracePeriod = 0;
    let isConnected = true;
    let isExited = false;
    void parent.disconnected.then(() => {
        isConnected = false;
        stopped.abort();
        request.reject(SandboxError.of("START_FAILED", "sandbox parent disconnected"));
    });
    using _terminated = process.on("SIGTERM", () => stopped.abort());
    using _interrupted = process.on("SIGINT", () => stopped.abort());
    using _received = parent.on((received) => {
        // stop on a request the launcher protocol has no shape for, reporting it to the parent
        const parsed = LauncherRequest.safeParse(received);
        if (!parsed.success) {
            request.reject(SandboxError.of("START_FAILED", "invalid launcher request"));
            stopped.abort();

            return;
        }
        const message = parsed.data;
        if (message.type === "start") {
            request.resolve(message.options);
        } else if (message.type === "stop") {
            gracePeriod = message.gracePeriod;
            stopped.abort();
        }
    });
    try {
        // require the system files of this platform
        const options = await request.promise;
        const runtime = RUNTIME_PATHS[host.os];
        if (runtime === undefined) {
            throw SandboxError.of("UNSUPPORTED", `sandboxing is unsupported on ${host.os}`);
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
                    allowUnixSockets: [...options.sockets, temporary],
                    allowLocalBinding: options.listening === "loopback",
                },
            },
            undefined,
            false,
        );
        stopped.signal.throwIfAborted();

        // quote each argument before upstream embeds the command in its shell invocation
        const quoted = [options.executable, ...options.arguments].map(quote).join(" ");
        const command = `export NO_PROXY= no_proxy=; exec ${quoted}`;
        const wrapped = await SandboxManager.wrapWithSandbox(
            command,
            "/bin/sh",
            undefined,
            stopped.signal,
        );
        const workload = process.spawn(["/bin/sh", "-c", wrapped], {
            workingDirectory: options.directory,
            variables: options.environment,
            isDetached: true,
            stdin: "inherit",
            stdout: "inherit",
            stderr: "inherit",
        });
        child = workload;
        const completed = workload.exited.then((code): SandboxExit => {
            isExited = true;
            const { signal } = workload;

            return signal === undefined ? { code, signal: null } : { code: null, signal };
        });
        parent.send({ type: "ready" });

        // terminate the process group when the parent stops or disconnects
        const grace = new AbortController();
        const stop = () => {
            signalGroup(workload, isExited, "SIGTERM", process);
            void environment.clock.sleep(gracePeriod, grace.signal).then(
                () => signalGroup(workload, isExited, "SIGKILL", process),
                () => undefined,
            );
        };
        stopped.signal.addEventListener("abort", stop, { once: true });
        if (stopped.signal.aborted) {
            stop();
        }
        exit = await completed;
        grace.abort();
        stopped.signal.removeEventListener("abort", stop);
    } catch (error) {
        if (child) {
            signalGroup(child, isExited, "SIGKILL", process);
        }
        if (isConnected) {
            parent.send({
                type: "error",
                message: error instanceof Error ? error.message : String(error),
            });
        }
        process.exitCode = 1;
    } finally {
        // report completion only after releasing proxies
        try {
            await SandboxManager.reset();
            if (exit && isConnected) {
                parent.send({ type: "exit", exit });
            }
        } finally {
            if (isConnected) {
                parent.disconnect();
            }
        }
    }
}

/** Preserve one literal shell argument. */
function quote(value: string): string {
    return `'${value.replaceAll("'", "'\\''")}'`;
}

/** Signal the workload process group, accepting an already terminated group. */
function signalGroup(
    child: Subprocess,
    isExited: boolean,
    signal: Signal,
    process: Process,
): void {
    // leave exited workloads alone, since their process group id may already be reused
    if (isExited) {
        return;
    }
    try {
        process.kill(-child.id, signal);
    } catch (error) {
        // accept gone groups, which macOS reports as not permitted for exited, unreaped workloads
        const isGone =
            ProcessError.is(error, "NOT_FOUND") || ProcessError.is(error, "PERMISSION_DENIED");
        if (!isGone) {
            throw error;
        }
    }
}
