import { outputsOf } from "@destack/build";
import { readPackageJson } from "@destack/build/local";
import { type PackageJson, PackageName } from "@destack/package";
import { type Identifier, present, type Version } from "@destack/schema";
import { RequestId } from "@destack/sync";
import * as stackObject from "@destack/stack/object";
import { CommandOption, type CommandCall, type CommandBody } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";

import { preview } from "@destack/machine/checkout";
import stackPackage from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";
import { Interrupt } from "../command/failure.ts";

/** What a build shows while it waits or runs, by state and phase. */
const PROGRESS = {
    pending: "Waiting for the checkout's host",
    preparing: "Preparing",
    building: "Building",
    storing: "Storing",
} satisfies Readonly<Record<"pending" | (typeof stackObject.BUILD_PHASES)[number], string>>;

/** The option selecting the outputs a build builds. */
const OUTPUTS_OPTION = {
    outputs: {
        type: "string",
        description:
            "The outputs to build, comma separated, every output the package implies by default",
        argument: "names",
    },
} as const;

/** Preview a checkout's package live against the space. */
export const devCommand: CommandBody = {
    arguments: [{ name: "package", default: ".", isDirectory: true }],
    isJson: true,
    async run({ arguments: { package: directory }, isJson: json }, context) {
        await previewCheckout(context, present(directory, "the package argument"), json);
    },
};

/** Build a checkout's package in the space, following its phases. */
export const buildCommand: CommandBody = {
    arguments: [{ name: "package", default: ".", isDirectory: true }],
    isJson: true,
    options: OUTPUTS_OPTION,
    async run({ arguments: { package: directory }, options, isJson: json }, context) {
        const built = await build(
            context,
            present(directory, "the package argument"),
            json,
            options,
        );
        await context.terminal.print(
            json
                ? JSON.stringify(built)
                : context.terminal.event("Built", "done", `build '${built.image}'`),
        );
    },
};

/** Build a checkout's package in the space and publish the build to the registry. */
export const publishCommand: CommandBody = {
    arguments: [{ name: "package", default: ".", isDirectory: true }],
    isJson: true,
    options: {
        ...OUTPUTS_OPTION,
        tag: {
            type: "string",
            description: "The distribution tag pointing at the release, latest by default",
            argument: "name",
        },
        yes: { type: "boolean", description: "Allow the space's stack to publish as you" },
    },
    async run({ arguments: { package: directory }, options, isJson: json }, context) {
        await publish(context, present(directory, "the package argument"), json, options);
    },
};

/** Build a checkout's package in a space and publish it with its unpublished dependencies. */
async function publish(
    context: CommandContext,
    directory: string,
    json: boolean,
    values: CommandCall["options"],
): Promise<void> {
    // build the package, and read the account of its name's handle
    const native = await context.native();
    const built = await build(context, directory, json, values, "release");
    const accountId = await native.resolve(PackageName.split(built.package.name).handle);

    // publish the build and the account's builds it pins that the forge lacks, as the person
    const consent = values["yes"] === true ? "approve" : "ask";
    const stack = await context.package(stackPackage, await context.locate(), consent);
    const tag = CommandOption.string(values["tag"], "tag");
    const published = await stack.build.publish({
        spaceId: built.space,
        id: built.id,
        requestId: RequestId.create(native.environment),
        account: accountId,
        ...(tag === undefined ? {} : { tag }),
    });

    // print each release, its dependencies before it
    const lines = published.releases.map((release) =>
        context.terminal.event(
            "Published",
            "done",
            `release '${release.package.name}@${release.package.version}'`,
        ),
    );
    await context.terminal.print(json ? JSON.stringify(published) : lines.join("\n"));
}

/** Ask the forge of a package name's account for the version its next release takes. */
async function nextRelease(context: CommandContext, name: string): Promise<Version> {
    const { client, home } = await context.registry(name);
    const { version } = await client.package.next({ spaceId: home, name });

    return version;
}

/** Build the checkout containing a directory in a space. */
async function build(
    context: CommandContext,
    directory: string,
    json: boolean,
    values: CommandCall["options"],
    version: "development" | "release" = "development",
): Promise<{
    readonly space: string;
    readonly id: Identifier<"build">;
    readonly image: string;
    readonly package: PackageJson["package"];
}> {
    // draw request identifiers from the device's environment
    const { environment } = await context.native();
    const space = await context.requireSpace();

    // find the package directory in its checkout, the space and the outputs
    const registered = await context.registerCheckout(directory);
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);
    const named = CommandOption.string(values["outputs"], "outputs");
    const read = await readPackageJson(
        await context.environment.files.realPath(directory),
        context.environment.files,
    );
    const implied = outputsOf(read);
    const outputs = named?.split(",") ?? Object.keys(implied);

    // create the build, which the checkout's environment runs
    const created = await stack.build.create({
        spaceId: space,
        requestId: RequestId.create(environment),
        source: {
            kind: "checkout",
            machine: registered.checkout.scope,
            checkout: registered.checkout.id,
            directory: registered.path,
        },
        outputs,
        ...(version === "release"
            ? { version: await nextRelease(context, read.package.name) }
            : {}),
    });

    // follow it until it finishes
    const finished = await follow(context, created.id, json);

    return {
        space,
        id: created.id,
        image: stackObject.Build.image(finished),
        package: read.package,
    };
}

/** Show each phase of a space's build until it finishes, cancelling it on an interrupt. */
async function follow(
    context: CommandContext,
    id: Identifier<"build">,
    json: boolean,
): Promise<stackObject.Build> {
    // open the space's builds
    const space = await context.requireSpace();
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);

    // copy the space's builds, cancelling the build on an interrupt
    const client = await context.native();
    const launcher = await context.launcher();
    await using opened = new AsyncDisposableStack();
    const builds = await client.open(launcher, { space }, { build: stackObject.build }, opened);
    using interrupt = new Interrupt(context.environment.process);
    const cancelled = new Promise<unknown>((resolve) => {
        interrupt.signal.addEventListener("abort", () => {
            resolve(
                stack.build.cancel({
                    spaceId: space,
                    id,
                    requestId: RequestId.create(client.environment),
                }),
            );
        });
    });

    // show each phase until the build finishes
    const live = context.terminal.live(json);
    for await (const row of builds.query.build
        .findFirst({ where: { id } })
        .watch(interrupt.signal)) {
        // show each state, and end once the build finished
        if (row !== undefined) {
            const current = stackObject.Build.of(row);
            await live.show(json ? JSON.stringify(current) : render(current));
            if (stackObject.Build.isFinished(current)) {
                return current;
            }
        }
    }

    // cancel an interrupted build, waiting for the cancel until a second interrupt
    if (interrupt.signal.aborted) {
        await Promise.race([cancelled, interrupt.again]);
        interrupt.require();
    }

    // refuse a build followed no longer
    throw CliError.of(
        "FOLLOW_UNAVAILABLE",
        `build '${id}' is unavailable: following stopped before it finished`,
    );
}

/** Render a build's progress, or how it finished. */
function render(current: stackObject.Build): string {
    // show the phase of a running build
    if (current.state === "running") {
        return PROGRESS[present(current.phase, "a running build's phase")];
    }
    // show a pending build
    else if (current.state === "pending") {
        return PROGRESS.pending;
    }
    // show how a finished build ended
    else {
        return `Build ${current.state}`;
    }
}

/** Start previewing a checkout's package live against a space, as the person signed in. */
async function previewCheckout(
    context: CommandContext,
    directory: string,
    json: boolean,
): Promise<void> {
    // find the checkout, the package directory in it, and the space
    const registered = await context.registerCheckout(directory);
    const space = await context.requireSpace();

    // start the preview, which its controller builds and runs
    const client = await context.native();
    const created = await client.machine().preview.create({
        machineId: registered.checkout.scope,
        requestId: RequestId.create(client.environment),
        checkoutId: registered.checkout.id,
        package: registered.path,
        space,
        login: (await context.signedIn()).row.id,
    });

    // follow the preview until the machine ran its generation or failed to
    await using opened = new AsyncDisposableStack();
    const previews = await client.follow({ preview }, opened);
    using interrupt = new Interrupt(context.environment.process);
    for await (const row of previews.query.preview
        .findFirst({ where: { id: created.id } })
        .watch(interrupt.signal)) {
        // wait for the machine to observe the generation
        const ready = row?.conditions["Ready"];
        if (row === undefined || ready?.observedGeneration !== row.generation) {
            continue;
        }

        // refuse a preview the machine failed to build or run, with its reason
        if (ready.status !== "true") {
            throw CliError.of(
                "PREVIEW_BROKEN",
                `preview of '${directory}' is broken: ${ready.message}`,
            );
        }
        const views = Object.keys(row.views);
        const { terminal } = context;
        const lines = [
            terminal.event("Previewing", "running", `'${directory}'`, row.id),
            ...(views.length === 0 ? [] : [terminal.hint("views", views.join(" · "))]),
        ];
        await terminal.print(json ? JSON.stringify(row) : lines.join("\n"));

        return;
    }
    interrupt.require();

    // refuse a preview followed no longer
    throw CliError.of(
        "FOLLOW_UNAVAILABLE",
        `preview '${created.id}' is unavailable: following stopped before it ran`,
    );
}
