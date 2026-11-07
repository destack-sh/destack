import { MachineProcess } from "@destack/machine/computer";
import { Stage } from "@destack/update/release";
import { type Verb, type Group, OptionValue } from "../command/command.ts";

import type { Context } from "../context/context.ts";
import { Completion, SHELLS } from "./completion.ts";
import { Setup } from "./setup.ts";
import { CliError } from "../error/index.ts";

/** Install, start, stop, update and uninstall this Destack distribution itself. */
export function selfCommand(context: Context): Group {
    return {
        description: "Install, start, stop, update and uninstall this Destack distribution",
        commands: {
            start: {
                description: "Start this computer's machine, or use the one running",
                json: true,
                async run({ json }) {
                    // start the machine beside this executable, reusing one that runs
                    const { environment } = context;
                    const setup = await Setup.current(environment);
                    await setup.start(
                        MachineProcess.executable(environment),
                        environment.process.runtime.isStandalone,
                    );
                    await context.terminal.print(
                        json ? JSON.stringify({ started: true }) : "Started the machine.",
                    );
                },
            },
            stop: {
                description: "Stop this computer's machine",
                json: true,
                async run({ json }) {
                    // stop the machine, reporting whether one ran
                    const stopped = await (await Setup.current(context.environment)).stop();
                    const line = stopped ? "Stopped the machine." : "No machine was running.";
                    await context.terminal.print(json ? JSON.stringify({ stopped }) : line);
                },
            },
            install: {
                description: "Install Destack for this user",
                json: false,
                options: {
                    archive: {
                        type: "string",
                        argument: "path",
                        description: "Verify and install a downloaded archive",
                    },
                },
                async run({ options }) {
                    await Setup.install(
                        context.environment,
                        OptionValue.string(options["archive"], "archive"),
                    );
                },
            },
            update: {
                description: "Stage the latest signed Destack release and restart into it",
                json: true,
                options: {
                    stage: {
                        type: "boolean",
                        description: "Download and verify without restarting",
                    },
                },
                async run({ options, json }) {
                    const setup = await Setup.current(context.environment);
                    await setup.update({ isStaging: options["stage"] === true, json });
                },
            },
            activate: {
                description:
                    "Restart into the staged release, as the machine asks once a restart is chosen",
                json: false,
                isHidden: true,
                async run() {
                    await (await Setup.current(context.environment)).activate();
                },
            },
            uninstall: {
                description: "Uninstall Destack and retain user state",
                json: false,
                options: {
                    native: {
                        type: "boolean",
                        description: "Remove user registration before native package removal",
                        isHidden: true,
                    },
                },
                async run({ options }) {
                    const setup = await Setup.current(context.environment);
                    await setup.uninstall({ isNative: options["native"] === true });
                },
            },
        },
    };
}

/** Print the completion script of a shell. */
export function completionCommand(context: Context): Verb {
    return {
        description: "Print the completion script of a shell: bash, zsh or fish",
        arguments: [{ name: "shell", complete: async () => SHELLS }],
        json: false,
        async run({ arguments: { shell } }) {
            // require a shell with a completion script
            const found = SHELLS.find((name) => name === shell);
            if (found === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `no completion for ${shell}: use bash, zsh or fish`,
                );
            }

            // print the script for this stage's command
            const { command } = Stage.current(context.environment);
            await context.terminal.print(Completion.write(found, command));
        },
    };
}
