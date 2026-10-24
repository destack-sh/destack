import { basename } from "node:path";
import type { Environment } from "@destack/environment";
import { CheckError } from "../error/index.ts";

/** Captured output from a checking tool at a fixed version. */
export interface ToolResult {
    /** The exit status, including ordinary diagnostic failures. */
    code: number;
    /** Machine-readable diagnostics or formatted file names. */
    stdout: string;
    /** Tool failure details. */
    stderr: string;
}

/** Run an installed tool on the current Bun runtime, reading its output. */
export async function runTool(
    executable: string,
    toolArguments: string[],
    directory: string,
    environment: Environment<"process">,
    options: {
        /** Cancel the tool. */
        readonly signal?: AbortSignal;
        /** The variables beside the process's own. */
        readonly variables?: Readonly<Record<string, string>>;
    } = {},
): Promise<ToolResult> {
    // run the tool under the caller's cancellation, refusing a tool the host cannot start
    const { process } = environment;
    const tool = basename(executable);
    const child = (() => {
        try {
            return process.spawn(
                [process.executable, "run", "--no-env-file", executable, ...toolArguments],
                {
                    workingDirectory: directory,
                    variables: { ...process.variables, ...options.variables, BUN_BE_BUN: "1" },
                    stdout: "pipe",
                    stderr: "pipe",
                    ...(options.signal === undefined ? {} : { signal: options.signal }),
                },
            );
        } catch (error) {
            throw CheckError.of("TOOL_FAILED", `cannot run ${tool}`, { cause: error });
        }
    })();

    // read both of its output streams, refusing a tool a signal ended
    const { code, signal, stdout, stderr } = await child.output();
    if (signal !== undefined) {
        throw CheckError.of("TOOL_FAILED", `${tool} terminated by ${signal}`);
    }
    const decoder = new TextDecoder();

    return { code, stdout: decoder.decode(stdout), stderr: decoder.decode(stderr) };
}
