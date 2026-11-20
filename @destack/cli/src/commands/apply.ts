import { canonicalize, type Identifier, present } from "@destack/schema";
import objectPackage from "@destack/object/package";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { ObjectReference, RequestId } from "@destack/sync";
import { type Command, CommandOption, type CommandCall } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "../installation/status.ts";

/** The installation argument of plan and approve, the space's stack by default. */
const INSTALLATION_ARGUMENT = {
    name: "installation",
    usage: "name",
    default: STACK_INSTALLATION,
} as const;

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
export const applyCommand: Command = {
    description: "Apply a checkout's stack to the space, following it until applied",
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

/** Show an installation, the space's stack by default, its resources and the plan it waits on. */
export const planCommand: Command = {
    description:
        "Show an installation, the space's stack by default, its resources and the plan it waits on",
    arguments: [INSTALLATION_ARGUMENT],
    isJson: true,
    options: { watch: { type: "boolean", description: "Follow it as it changes" } },
    async run({ arguments: { installation: name }, options, isJson: json }, context) {
        // show it once, or each change when watched
        const space = await context.requireSpace();
        const objects = await context.package(objectPackage, await context.locate());
        const { id } = await objects.installation.get({
            scope: space,
            id: present(name, "the installation argument"),
        });
        const live = context.terminal.live(json);
        for await (const status of InstallationStatus.follow(id, context)) {
            await live.show(status.show(json, context));
            if (options["watch"] !== true) {
                break;
            }
        }
    },
};

/** Approve the plan an installation, the space's stack by default, waits on. */
export const approveCommand: Command = decideCommand("approve");

/** Reject the plan an installation, the space's stack by default, waits on. */
export const rejectCommand: Command = decideCommand("reject");

/** Decide the plan an installation waits on through its change request, by the reviewed digest. */
function decideCommand(decision: "approve" | "reject"): Command {
    const verb = decision === "approve" ? "Approve" : "Reject";

    return {
        description: `${verb} the plan an installation, the space's stack by default, waits on`,
        arguments: [INSTALLATION_ARGUMENT],
        isJson: true,
        options: {
            plan: {
                type: "string",
                description: "The digest of the plan you reviewed",
                argument: "digest",
            },
        },
        async run({ arguments: { installation: name }, options, isJson: json }, context) {
            // decide the reviewed plan's digest
            const plan = CommandOption.string(options["plan"], "plan");
            if (plan === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    "usage is invalid: give the reviewed plan's digest with --plan",
                );
            }
            const space = await context.requireSpace();
            const objects = await context.package(objectPackage, await context.locate());
            const { id } = await objects.installation.get({
                scope: space,
                id: present(name, "the installation argument"),
            });
            const { environment } = await context.native();
            const stack = await context.package(stackPackage, await context.locate());
            const waiting = await stack.rollout.plan({ spaceId: space, id });
            if (waiting.digest !== plan || waiting.approval === null) {
                throw CliError.of(
                    "PLAN_BLOCKED",
                    `cannot apply plan '${plan}': '${name}' waits on no change request for it`,
                );
            }
            await objects.approval[decision]({
                scope: space,
                id: waiting.approval,
                requestId: RequestId.create(environment),
            });
            const done = decision === "approve" ? "Approved" : "Rejected";
            await context.terminal.print(
                json
                    ? JSON.stringify({ id, plan, decision })
                    : context.terminal.event(done, "done", `plan '${plan}'`),
            );
        },
    };
}

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
    const objects = await context.package(objectPackage, location);
    const { items } = await objects.installation.list({
        scope: space,
        where: { name: STACK_INSTALLATION },
        limit: 1,
    });
    const [existing] = items;
    const followed = {
        ...ObjectReference.columns("follows", submission.follows),
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
        installed = await objects.installation.create({
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
        installed = await objects.installation.update({
            scope: space,
            id: existing.id,
            requestId: RequestId.create(environment),
            revision: existing.revision,
            ...followed,
        });
    }

    // deploy the stack at the submitted build
    const stack = await context.package(stackPackage, location);
    await stack.rollout.deploy({
        spaceId: space,
        id: installed.id,
        requestId: RequestId.create(environment),
        build: submission.build,
    });

    return installed.id;
}
