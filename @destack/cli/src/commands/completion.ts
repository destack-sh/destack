import { Universe } from "@destack/update/release";
import type { Command } from "../command/command.ts";
import { Completion, SHELLS } from "../distribution/completion.ts";
import { CliError } from "../error/index.ts";

/** Print the completion script of a shell. */
export const completionCommand: Command = {
    description: "Print the completion script of a shell: bash, zsh or fish",
    arguments: [{ name: "shell", complete: async () => SHELLS }],
    async run({ arguments: { shell } }, context) {
        // require a shell with a completion script
        const found = SHELLS.find((name) => name === shell);
        if (found === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `shell '${shell}' is invalid: it has no completion, so use bash, zsh or fish`,
            );
        }

        // print the script for this universe's command
        const { command } = Universe.current(context.environment);
        await context.terminal.print(Completion.write(found, command));
    },
};
