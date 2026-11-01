import type { Environment } from "@destack/environment";
import { RequestBody } from "@destack/object";
import { MachineError } from "../../error/index.ts";

/** The host capabilities OS commands run with: files, processes and the platform. */
export type CommandHost = Environment<"files" | "process" | "host">;

/** An OS command that ran to its exit: its exit code and what it wrote. */
export interface CommandResult {
    /** The exit code. */
    readonly code: number;
    /** What it wrote to standard output. */
    readonly output: string;
    /** What it wrote to standard error. */
    readonly errors: string;
}

/** How long an OS service command may take, in milliseconds: a systemd stop waits up to 20 s. */
export const SERVICE_MILLISECONDS = 25_000;

/** OS commands a host runs to their exit, without a terminal or input, their outputs bounded. */
export const Command = {
    /** Run a command to its exit within a timeout, refusing an exit it does not accept. */
    async run(
        host: CommandHost,
        command: readonly string[],
        options: {
            /** The command's environment variables, the host's by default. */
            readonly variables?: Readonly<Record<string, string>>;
            /** How long it may run, in milliseconds. */
            readonly timeout: number;
            /** The exit codes it may end with beside 0, none by default. */
            readonly accepted?: readonly number[];
        },
    ): Promise<CommandResult> {
        // run it without a terminal or input, ended once it outlasts its timeout
        const child = host.process.spawn(command, {
            variables: options.variables ?? host.process.variables,
            stdin: "ignore",
            stdout: "pipe",
            stderr: "pipe",
            signal: host.clock.timeout(options.timeout),
        });

        // read its exit code and both outputs, each within the body limit
        const name = command[0] ?? "a command";
        const [code, output, errors] = await Promise.all([
            child.exited,
            RequestBody.text(new Response(child.stdout), `${name}'s output`),
            RequestBody.text(new Response(child.stderr), `${name}'s errors`),
        ]);

        // refuse an exit it does not accept, with its diagnostic
        if (code !== 0 && !(options.accepted ?? []).includes(code)) {
            throw MachineError.of(
                "COMMAND_FAILED",
                `${name} failed with ${code}: ${errors.trim()}`,
            );
        }

        return { code, output, errors };
    },
};
