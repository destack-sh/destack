import { CommandOption, type CommandBody } from "../command/command.ts";

import { Setup } from "./setup.ts";

/** Install, start, stop, update and uninstall this Destack distribution itself. */
export const selfCommand: CommandBody = {
    commands: {
        start: {
            description: "Start this computer's machine, or use the one running",
            isJson: true,
            async run({ isJson: json }, context) {
                // start the machine of this executable, reusing one that runs
                const { environment } = context;
                const setup = await Setup.current(environment);
                await setup.start(
                    environment.process.executable,
                    environment.process.runtime.isStandalone,
                );
                await context.terminal.print(
                    json
                        ? JSON.stringify({ started: true })
                        : context.terminal.event("Started", "done", "the machine"),
                );
            },
        },
        stop: {
            description: "Stop this computer's machine",
            isJson: true,
            async run({ isJson: json }, context) {
                // stop the machine, reporting whether one ran
                const stopped = await (await Setup.current(context.environment)).stop();
                const line = stopped
                    ? context.terminal.event("Stopped", "done", "the machine")
                    : context.terminal.hint("none", "no machine was running");
                await context.terminal.print(json ? JSON.stringify({ stopped }) : line);
            },
        },
        install: {
            description: "Install Destack for this user",
            options: {
                source: {
                    type: "string",
                    argument: "directory",
                    description: "Verify and install a downloaded release's files",
                },
            },
            async run({ options }, context) {
                await Setup.install(
                    context.environment,
                    CommandOption.string(options["source"], "source"),
                );
            },
        },
        update: {
            description: "Stage the latest signed Destack release and restart into it",
            isJson: true,
            options: {
                stage: {
                    type: "boolean",
                    description: "Download and verify without restarting",
                },
            },
            async run({ options, isJson: json }, context) {
                const setup = await Setup.current(context.environment);
                await setup.update({ isStaging: options["stage"] === true, json });
            },
        },
        activate: {
            description:
                "Restart into the staged release, as the machine asks once a restart is chosen",
            isHidden: true,
            async run(_call, context) {
                await (await Setup.current(context.environment)).activate();
            },
        },
        uninstall: {
            description: "Uninstall Destack and retain user state",
            options: {
                native: {
                    type: "boolean",
                    description: "Remove user registration before native package removal",
                    isHidden: true,
                },
            },
            async run({ options }, context) {
                const setup = await Setup.current(context.environment);
                await setup.uninstall({ isNative: options["native"] === true });
            },
        },
    },
};
