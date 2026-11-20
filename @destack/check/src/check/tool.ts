import { basename } from "node:path";
import type { Environment } from "@destack/environment";
import { CheckError } from "../error/index.ts";

/** The variables that turn color off in a tool whose output is read. */
const PLAIN_OUTPUT = { NO_COLOR: "1", FORCE_COLOR: "0" } as const;

/** Captured output from a checking tool at a fixed version. */
export interface ToolResult {
    /** The exit status, including ordinary diagnostic failures. */
    code: number;
    /** Machine-readable diagnostics or formatted file names. */
    stdout: string;
    /** Tool failure details. */
    stderr: string;
}

/** Run an installed tool's script on the current Bun runtime, reading its output. */
export async function runTool(
    executable: string,
    toolArguments: string[],
    directory: string,
    environment: Environment<"process">,
    options: ToolOptions = {},
): Promise<ToolResult> {
    return await runBun(
        ["run", "--no-env-file", executable, ...toolArguments],
        basename(executable),
        directory,
        environment,
        options,
    );
}

/** Run a command of the current Bun runtime, such as `lint`, reading its output. */
export async function runBun(
    command: readonly string[],
    tool: string,
    directory: string,
    environment: Environment<"process">,
    options: ToolOptions = {},
): Promise<ToolResult> {
    // start Bun under the caller's cancellation, its output plain for reading
    const { process } = environment;
    const child = (() => {
        try {
            return process.spawn([process.executable, ...command], {
                workingDirectory: directory,
                variables: {
                    ...process.variables,
                    ...options.variables,
                    ...PLAIN_OUTPUT,
                    BUN_BE_BUN: "1",
                },
                stdout: "pipe",
                stderr: "pipe",
                ...(options.signal === undefined ? {} : { signal: options.signal }),
            });
        } catch (error) {
            throw CheckError.of("TOOL_BROKEN", `cannot run '${tool}'`, { cause: error });
        }
    })();

    // refuse a tool a signal ended
    const { code, signal, stdout, stderr } = await child.output();
    if (signal !== undefined) {
        throw CheckError.of("TOOL_BROKEN", `'${tool}' terminated by '${signal}'`);
    }
    const decoder = new TextDecoder();

    return { code, stdout: decoder.decode(stdout), stderr: decoder.decode(stderr) };
}

/** How a tool runs: its cancellation and its variables beside the process's own. */
export interface ToolOptions {
    /** Cancel the tool. */
    readonly signal?: AbortSignal;
    /** The variables beside the process's own. */
    readonly variables?: Readonly<Record<string, string>>;
}
