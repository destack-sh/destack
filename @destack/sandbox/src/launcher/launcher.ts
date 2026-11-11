import { SandboxManager, getDefaultWritePaths } from "@anthropic-ai/sandbox-runtime";
import { getApplySeccompBinaryPath } from "@anthropic-ai/sandbox-runtime/dist/sandbox/generate-seccomp-filter.js";
import {
    type Environment,
    type OperatingSystem,
    type Process,
    FileSystemError,
    ProcessError,
    type Signal,
    type Subprocess,
} from "@destack/environment";
import { SandboxError } from "../error/index.ts";
import { LauncherRequest, type SandboxExit, type SandboxOptions } from "../sandbox/message.ts";
import { SandboxTool } from "../sandbox/tools.ts";

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
export async function runLauncher(
    environment: Environment<"process" | "host" | "files">,
): Promise<void> {
    // reject direct invocation without the supervising process and its temporary directory
    const { host, process } = environment;
    const { parent } = process;
    const temporary = process.variables["CLAUDE_CODE_TMPDIR"];
    if (parent === undefined || temporary === undefined) {
        throw SandboxError.of("START_BROKEN", "sandbox launcher requires a parent connection");
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
        request.reject(SandboxError.of("START_BROKEN", "sandbox parent disconnected"));
    });
    using _terminated = process.on("SIGTERM", () => stopped.abort());
    using _interrupted = process.on("SIGINT", () => stopped.abort());
    using _received = parent.on((received) => {
        // stop on a request the launcher protocol has no shape for, reporting it to the parent
        const parsed = LauncherRequest.safeParse(received);
        if (!parsed.success) {
            request.reject(
                SandboxError.of(
                    "START_BROKEN",
                    "launcher request does not match the launcher protocol",
                ),
            );
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
        // require the system files of this platform, and on Linux the seccomp helper bwrap runs
        const options = await request.promise;
        const runtime = RUNTIME_PATHS[host.os];
        const helper = host.os === "linux" ? getApplySeccompBinaryPath() : undefined;
        if (runtime === undefined || helper === null) {
            throw SandboxError.of(
                "HOST_UNSUPPORTED",
                `sandboxing on host '${host.os}' is not supported`,
            );
        }

        // find a release's own tools
        const ripgrep = SandboxTool.path("ripgrep", process, host.os);
        const socat = SandboxTool.path("socat", process, host.os);

        // configure a fresh manager with no access to other application state
        await SandboxManager.initialize(
            {
                ...(ripgrep === undefined ? {} : { ripgrep: { command: ripgrep } }),
                ...(socat === undefined ? {} : { socatPath: socat }),
                filesystem: {
                    denyRead: ["/"],
                    allowRead: [
                        ...runtime,
                        ...(helper === undefined ? [] : [helper]),
                        ...(socat === undefined ? [] : [socat]),
                        options.executable,
                        ...options.read,
                        ...options.write,
                        temporary,
                    ],
                    // TODO #Cleanup: drop patches/@anthropic-ai%2Fsandbox-runtime@0.0.77.patch once upstream keeps
                    //  writes below read grants on Linux, which it mounts read-only over them
                    allowWrite: [...options.write, temporary],
                    denyWrite: getDefaultWritePaths().filter((path) => !path.startsWith("/dev/")),
                },
                network: {
                    allowedDomains: options.network,
                    deniedDomains: [],
                    allowUnixSockets: [...options.sockets, temporary],
                    // allow Unix sockets on Linux, bounded by the sandbox's mounts and network namespace
                    allowAllUnixSockets: host.os === "linux",
                    // bind loopback ports for the workload's own processes when it may serve
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

        // ask the workload to stop when the parent stops or disconnects, then kill its process group
        const grace = new AbortController();
        const stop = () => {
            void terminate(workload, isExited, options.executable, environment).catch(
                (error: unknown) => parent.send({ type: "error", message: String(error) }),
            );
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

/** Ask the workload itself to stop: its process group on macOS, its processes on Linux. */
async function terminate(
    child: Subprocess,
    isExited: boolean,
    executable: string,
    environment: Environment<"process" | "host" | "files">,
): Promise<void> {
    // leave an exited workload, and signal its process group on macOS, where it runs in place
    const { process, files, host } = environment;
    if (isExited) {
        return;
    } else if (host.os !== "linux") {
        signalGroup(child, isExited, "SIGTERM", process);

        return;
    }

    // signal each process below bubblewrap running the workload's executable, as bubblewrap forwards no signal
    // TODO #Cleanup: signal the workload alone once sandbox-runtime forwards signals through bubblewrap
    const target = await files.realPath(executable);
    for (const id of await descendants(child.id, environment)) {
        const running = await files.realPath(`/proc/${id}/exe`).catch(FileSystemError.absent);
        if (running === target) {
            signalProcess(id, "SIGTERM", process);
        }
    }
}

/** List the processes below a process, as Linux lists each one's children in procfs. */
async function descendants(id: number, environment: Environment<"files">): Promise<number[]> {
    // read the process's children, none once it exited
    const listed = await environment.files
        .readTextFile(`/proc/${id}/task/${id}/children`)
        .catch(FileSystemError.absent);
    const children = (listed ?? "")
        .split(" ")
        .filter((each) => each !== "")
        .map(Number);

    // add each child's own descendants
    const below = await Promise.all(children.map((child) => descendants(child, environment)));

    return [...children, ...below.flat()];
}

/** Preserve one literal shell argument. */
function quote(value: string): string {
    return `'${value.replaceAll("'", "'\\''")}'`;
}

/** Signal the workload's process group, leaving an exited one, whose group id may be reused. */
function signalGroup(child: Subprocess, isExited: boolean, signal: Signal, process: Process): void {
    if (!isExited) {
        signalProcess(-child.id, signal, process);
    }
}

/** Signal a process, or a group by its negated id, accepting one already gone. */
function signalProcess(id: number, signal: Signal, process: Process): void {
    try {
        process.kill(id, signal);
    } catch (error) {
        // accept gone processes, which macOS reports as not permitted for exited, unreaped ones
        const isGone =
            ProcessError.is(error, "FILE_NOT_FOUND") || ProcessError.is(error, "PERMISSION_DENIED");
        if (!isGone) {
            throw error;
        }
    }
}
