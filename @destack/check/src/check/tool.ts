import { spawn } from "node:child_process";
import { basename } from "node:path";
import process from "node:process";
import { CheckError } from "../error/index.ts";

/** Maximum time for one checking tool invocation. */
const toolTimeout = 30_000;

/** Captured output from a pinned checking tool. */
export interface ToolResult {
    /** The exit status, including ordinary diagnostic failures. */
    code: number;
    /** Machine-readable diagnostics or formatted file names. */
    stdout: string;
    /** Tool failure details. */
    stderr: string;
}

/** Run the installed tool using the current Bun runtime. */
export function runTool(
    executable: string,
    toolArguments: string[],
    directory: string,
    signal?: AbortSignal,
    environment?: NodeJS.ProcessEnv,
): Promise<ToolResult> {
    const tool = basename(executable);

    return new Promise((complete, reject) => {
        // bound the child lifetime and propagate caller cancellation
        const child = spawn(
            process.execPath,
            ["run", "--no-env-file", executable, ...toolArguments],
            {
                cwd: directory,
                env: environment,
                stdio: ["ignore", "pipe", "pipe"],
                signal: AbortSignal.any([
                    AbortSignal.timeout(toolTimeout),
                    ...(signal ? [signal] : []),
                ]),
            },
        );

        // capture both output streams until the child closes
        let stdout = "";
        let stderr = "";
        child.stdout.setEncoding("utf8").on("data", (chunk) => {
            stdout += chunk;
        });
        child.stderr.setEncoding("utf8").on("data", (chunk) => {
            stderr += chunk;
        });

        // reject launch failures and signal termination
        child.on("error", (error) =>
            reject(new CheckError("tool", `cannot start ${tool}`, { cause: error })),
        );
        child.on("close", (code, signal) => {
            if (code === null) {
                reject(new CheckError("tool", `${tool} terminated by ${signal}`));
            } else {
                complete({ code, stdout, stderr });
            }
        });
    });
}
