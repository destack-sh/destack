import { canonicalize, type Identifier, present } from "@destack/schema";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { ObjectReference, RequestId } from "@destack/sync";
import { CommandOption, type CommandCall, type CommandBody } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "../catalog/status.ts";

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
        const id = await submitStack(
            present(directory, "the directory argument"),
            options,
            context,
        );
        await InstallationStatus.drive(id, options["yes"] === true, json, context);
    },
};

/** Evaluate a checkout's stack and submit it to the space, creating its stack installation once. */
async function submitStack(
    directory: string,
    values: CommandCall["options"],
    context: CommandContext,
): Promise<Identifier<"installation">> {
    // require the export and read the parameters
    const { environment } = await context.native();
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

    // evaluate it in the innermost registered checkout containing the directory
    const space = await context.requireSpace();
    const registered = await context.registerCheckout(directory);
    const { packageId, submission } = await (await context.native()).machine().checkout.submit({
        machineId: registered.checkout.scope,
        id: registered.checkout.id,
        requestId: RequestId.create(environment),
        space,
        login: (await context.signedIn()).row.id,
        directory: registered.path,
        export: exported,
        parameters,
    });

    // create the stack installation when the space has none
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);
    const { items } = await stack.installation.list({
        scope: space,
        where: { name: STACK_INSTALLATION },
        limit: 1,
    });
    const [existing] = items;
    const followed = {
        ...ObjectReference.columns("version", submission.version),
        directory: submission.directory ?? null,
        export: submission.stack.export,
        parameters: submission.stack.parameters,
    };

    // leave an installation that follows the checkout with this export and these arguments already
    const stored: Readonly<Record<string, unknown>> = existing ?? {};
    const current = Object.fromEntries(
        Object.keys(followed).map((key) => [key, stored[key] ?? null]),
    );
    const isFollowing =
        existing?.packageId === packageId && canonicalize(current) === canonicalize(followed);

    // follow the checkout with the export and its arguments on a new installation
    let installed: { readonly id: Identifier<"installation"> };
    if (existing === undefined) {
        installed = await stack.installation.create({
            scope: space,
            requestId: RequestId.create(environment),
            packageId,
            ...followed,
            name: STACK_INSTALLATION,
        });
    }
    // leave the space's installation following it already
    else if (isFollowing) {
        installed = existing;
    }
    // follow it on the space's installation
    else {
        installed = await stack.installation.update({
            scope: space,
            id: existing.id,
            requestId: RequestId.create(environment),
            revision: existing.revision,
            ...followed,
        });
    }

    // reconcile the stack at the submitted build
    await stack.installation.reconcile({
        scope: space,
        id: installed.id,
        requestId: RequestId.create(environment),
        build: submission.build,
    });

    return installed.id;
}
