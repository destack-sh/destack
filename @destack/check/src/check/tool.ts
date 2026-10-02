import { spawn } from "node:child_process";
import { mkdtemp, open, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
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

/** Run an installed tool on the current Bun runtime, writing its output to a file. */
export async function runTool(
    executable: string,
    toolArguments: string[],
    directory: string,
    signal?: AbortSignal,
    environment?: NodeJS.ProcessEnv,
): Promise<ToolResult> {
    // collect the tool's output in a temporary file
    const tool = basename(executable);
    const folder = await mkdtemp(join(tmpdir(), "destack-tool-"));
    const path = join(folder, "stdout");
    const output = await open(path, "w");
    try {
        // run the child with its output in the file
        const { code, stderr } = await new Promise<{ code: number; stderr: string }>(
            (complete, reject) => {
                // bound the child lifetime and propagate caller cancellation
                const child = spawn(
                    process.execPath,
                    ["run", "--no-env-file", executable, ...toolArguments],
                    {
                        cwd: directory,
                        env: environment,
                        stdio: ["ignore", output.fd, "pipe"],
                        signal: AbortSignal.any([
                            AbortSignal.timeout(toolTimeout),
                            ...(signal ? [signal] : []),
                        ]),
                    },
                );

                // capture the error stream until the child closes
                let captured = "";
                child.stderr?.setEncoding("utf8").on("data", (chunk: string) => {
                    captured += chunk;
                });

                // reject launch failures and signal termination
                child.on("error", (error) =>
                    reject(new CheckError("tool", `cannot start ${tool}`, { cause: error })),
                );
                child.on("close", (exit, terminated) => {
                    if (exit === null) {
                        reject(
                            new CheckError("tool", `${tool} terminated by ${String(terminated)}`),
                        );
                    } else {
                        complete({ code: exit, stderr: captured });
                    }
                });
            },
        );

        return { code, stdout: await readFile(path, "utf8"), stderr };
    } finally {
        await output.close();
        await rm(folder, { recursive: true, force: true });
    }
}
