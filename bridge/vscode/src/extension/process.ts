import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import type * as vscode from "vscode";
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
};

/**
 * The spawned server process and stream transport pair.
 */
export type SpawnServerResult = {
    /**
     * The spawned language server process handle.
     */
    process: ChildProcessWithoutNullStreams;
    /**
     * The language client stream transport wiring.
     */
    streamInfo: StreamInfo;
};

/**
 * Spawn the language server process and build language client stream info.
 */
export async function spawnServerProcess(options: SpawnServerOptions): Promise<SpawnServerResult> {
    const { command, args, cwd, workspaceRoot, serverLog } = options;

    // create and initialize the process
    return await new Promise((resolve, reject) => {
        // spawn the child process
        const serverProcess = spawn(command, args, {
            stdio: ["pipe", "pipe", "pipe"],
            cwd,
            env: process.env,
            shell: false,
        });

        // forward stderr to the server output channel
        serverProcess.stderr.setEncoding("utf8");
        serverProcess.stderr.on("data", (chunk: string) => {
            serverLog.append(chunk);
        });

        // resolve stream transports once the process is ready
        serverProcess.once("spawn", () => {
            const workingDirectory = cwd || workspaceRoot;
            serverLog.info(
                `spawned ${command} ${args.join(" ")} (pid ${serverProcess.pid ?? ""}) cwd=${workingDirectory}`,
            );

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
    // skip stop handling when no active process exists
    if (!serverProcess || serverProcess.killed) {
        return;
    }

    // resolve process id once for signal operations
    const serverProcessId = serverProcess.pid;
    if (serverProcessId == undefined) {
        serverLog.appendLine("server process has no pid, skipping signal shutdown");
        return;
    }

    // close stdin to allow graceful process shutdown
    try {
        serverProcess.stdin.end();
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        serverLog.appendLine(`failed to close server stdin: ${message}`);
    }

    // send termination signal first
    try {
        process.kill(serverProcessId, "SIGTERM");
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        serverLog.appendLine(`failed to send SIGTERM to server process: ${message}`);
    }

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
        process.kill(serverProcessId, "SIGKILL");
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        serverLog.appendLine(`failed to send SIGKILL to server process: ${message}`);
    }
}
