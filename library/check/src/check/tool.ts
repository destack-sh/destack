import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
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

/** Run the installed tool using the current Deno runtime. */
export function runTool(
    tool: "oxlint" | "oxfmt",
    args: string[],
    directory: string,
    signal?: AbortSignal,
): Promise<ToolResult> {
    // resolve the pinned package without downloading tools during checks
    const require = createRequire(import.meta.url);
    const packagePath = require.resolve(`${tool}/package.json`);
    const executable = resolve(dirname(packagePath), "bin", tool);

    return new Promise((complete, reject) => {
        // bound the child lifetime and propagate caller cancellation
        const child = spawn(process.execPath, ["run", "-A", executable, ...args], {
            cwd: directory,
            stdio: ["ignore", "pipe", "pipe"],
            signal: AbortSignal.any([
                AbortSignal.timeout(toolTimeout),
                ...(signal ? [signal] : []),
            ]),
        });

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
