import { type Identifier, present } from "@destack/schema";
import { STACK_INSTALLATION } from "@destack/stack/package";
import { CommandOption, type CommandCall, type CommandBody } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "../catalog/status.ts";
import { LocalCheckout } from "./checkout.ts";

/** The options selecting the stack export and its parameters. */
const EXPORT_OPTIONS = {
    export: {
        type: "string",
        description: "The exported stack to evaluate",
        argument: "name",
    },
    parameters: {
        type: "string",
        description: "JSON arguments for a parameterised definition, - for stdin, @path for a file",
        argument: "json",
    },
} as const;

/** Apply a checkout's stack to the space and follow it until applied. */
export const applyCommand: CommandBody = {
    arguments: [{ name: "directory", default: ".", isDirectory: true }],
    isJson: true,
    options: {
        ...EXPORT_OPTIONS,
        yes: { type: "boolean", description: "Approve each plan the stack waits on" },
    },
    async run({ arguments: { directory }, options, isJson: json }, context) {
        // submit the checkout's stack, then follow it until applied, approving each waiting plan once
        const id = await applyStack(present(directory, "the directory argument"), options, context);
        await InstallationStatus.drive(id, options["yes"] === true, json, context);
    },
};

/** Install a checkout's stack in the space, or switch the space's stack to it. */
async function applyStack(
    directory: string,
    values: CommandCall["options"],
    context: CommandContext,
): Promise<Identifier<"installation">> {
    // require the export and read the parameters
    const exported = CommandOption.string(values["export"], "export");
    if (exported === undefined) {
        throw CliError.of(
            "USAGE_INVALID",
            "usage is invalid: select the exported definition with --export",
        );
    }
    const parameters = await CommandOption.json(
        values["parameters"],
        "parameters",
        context.environment,
        context.terminal,
    );

    // push the checkout, and install the space's stack following it, or switch the stack to it
    const target = await (await LocalCheckout.open(directory, context)).target(context);
    const stack = { export: exported, parameters };
    const { id } = await target.install(STACK_INSTALLATION, stack, { kind: "none" }, context);

    return id;
}
