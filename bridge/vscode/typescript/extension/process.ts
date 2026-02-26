import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { PassThrough } from "node:stream";
import * as vscode from "vscode";
import type { StreamInfo } from "vscode-languageclient/node";

/**
 * Spawn options for the Destack language server process.
 */
export type SpawnServerOptions = {
    /**
     * The executable command path.
     */
    command: string;
    /**
     * The executable arguments.
     */
    args: string[];
    /**
     * The optional process working directory.
     */
    cwd?: string;
    /**
     * The active workspace root.
     */
    workspaceRoot?: string;
    /**
     * The server output channel.
     */
    serverLog: vscode.LogOutputChannel;
    /**
     * The debug mode flag.
     */
    debug: boolean;
};

/**
 * Spawn the language server process and build language client stream info.
 */
export async function spawnServerProcess(
    options: SpawnServerOptions,
): Promise<{
    process: ChildProcessWithoutNullStreams;
    streamInfo: StreamInfo;
}> {
    const { command, args, cwd, workspaceRoot, serverLog, debug } = options;

    // create and initialize the process
    return await new Promise((resolve, reject) => {
        // build process environment
        const processEnvironment = { ...process.env };
        if (debug) {
            processEnvironment.WAIT_FOR_DEBUGGER = "1";
            processEnvironment.RUST_LOG = processEnvironment.RUST_LOG || "trace";
            processEnvironment.RUST_BACKTRACE = processEnvironment.RUST_BACKTRACE || "full";
        }

        // spawn the child process
        const serverProcess = spawn(command, args, {
            stdio: ["pipe", "pipe", "pipe"],
            cwd,
            env: processEnvironment,
            shell: false,
        });

        // forward stderr to the server output channel
        serverProcess.stderr.setEncoding("utf8");
        serverProcess.stderr.on("data", (chunk: string) => serverLog.append(chunk));

        // resolve stream transports once the process is ready
        serverProcess.once("spawn", () => {
            const workingDirectory = cwd || workspaceRoot;
            serverLog.info(
                `spawned ${command} ${args.join(" ")} (pid ${serverProcess.pid ?? ""}) cwd=${workingDirectory}`,
            );

            if (debug) {
                const streamInfo = createDebugStreamInfo(serverProcess, serverLog);
                resolve({
                    process: serverProcess,
                    streamInfo,
                });
                return;
            }

            resolve({
                process: serverProcess,
                streamInfo: {
                    reader: serverProcess.stdout,
                    writer: serverProcess.stdin,
                },
            });
        });

        // reject when spawn fails
        serverProcess.once("error", (error) => {
            serverLog.error(`failed to spawn ${command}: ${error.message}`);
            reject(error);
        });

        // log process exit for observability
        serverProcess.on("exit", (code, signal) => {
            serverLog.warn(`${command} exited (code=${code}, signal=${signal ?? ""})`);
        });
    });
}

/**
 * Stop the server process gracefully and force kill when needed.
 */
export async function stopServerProcess(
    serverProcess: ChildProcessWithoutNullStreams | undefined,
    serverLog: vscode.OutputChannel,
): Promise<void> {
    if (!serverProcess || serverProcess.killed) {
        return;
    }

    // close stdin to allow graceful process shutdown
    try {
        serverProcess.stdin.end();
    } catch {}

    // send termination signal first
    try {
        process.kill(serverProcess.pid!, "SIGTERM");
    } catch {}

    // wait briefly for clean exit
    const isExited = await new Promise<boolean>((resolve) => {
        const exitTimeout = setTimeout(() => resolve(false), 1500);
        serverProcess.once("exit", () => {
            clearTimeout(exitTimeout);
            resolve(true);
        });
        serverProcess.once("close", () => {
            clearTimeout(exitTimeout);
            resolve(true);
        });
    });

    if (isExited) {
        return;
    }

    // escalate to kill when graceful shutdown fails
    serverLog.appendLine("server did not exit on SIGTERM, sending SIGKILL.");
    try {
        process.kill(serverProcess.pid!, "SIGKILL");
    } catch {}
}

/**
 * Create debug stream wrappers that mirror transport traffic to logs.
 */
function createDebugStreamInfo(
    serverProcess: ChildProcessWithoutNullStreams,
    serverLog: vscode.LogOutputChannel,
): StreamInfo {
    // duplicate server stdout into the log output
    const serverOutputTee = new PassThrough();
    serverProcess.stdout.pipe(serverOutputTee);
    serverOutputTee.on("data", (chunk) => {
        serverLog.append(`[server -> client]\n${chunk.toString()}\n`);
    });

    // duplicate client input into the log output
    const clientInputTee = new PassThrough();
    clientInputTee.on("data", (chunk) => {
        serverLog.append(`[client -> server]\n${chunk.toString()}\n`);
    });
    clientInputTee.pipe(serverProcess.stdin);

    return {
        reader: serverOutputTee,
        writer: clientInputTee,
    };
}
