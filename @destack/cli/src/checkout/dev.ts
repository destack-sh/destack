import { outputsOf } from "@destack/build";
import { readPackageJson } from "@destack/build/local";
import { type PackageJson, PackageName } from "@destack/package";
import { type Identifier, present, type Version } from "@destack/schema";
import * as stackObject from "@destack/stack/object";
import { CommandOption, type CommandCall, type CommandBody } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";

import { RequestId } from "@destack/sync";
import { LocalCheckout } from "./checkout.ts";
import { InstallationStatus } from "../catalog/status.ts";
import stackPackage from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";
import { Interrupt } from "../command/failure.ts";
import { INSTALLATION_PARENT_OPTIONS, InstallationParentOption } from "../catalog/installation.ts";

/** The option selecting the outputs a build builds. */
const OUTPUTS_OPTION = {
    outputs: {
        type: "string",
        description:
            "The outputs to build, comma separated, every output the package implies by default",
        argument: "names",
    },
} as const;

/** Install a checkout's package in the space and push its working tree on each change. */
export const devCommand: CommandBody = {
    arguments: [{ name: "package", default: ".", isDirectory: true }],
    options: INSTALLATION_PARENT_OPTIONS,
    isJson: true,
    async run({ arguments: { package: directory }, options, isJson: json }, context) {
        // install the checkout's package, or switch its installation to the checkout
        const checkout = await LocalCheckout.open(
            present(directory, "the package argument"),
            context,
        );
        const target = await checkout.target(context);
        const parent = InstallationParentOption.read(options);
        const { id, name } = await target.install(undefined, undefined, parent, context);
        const { terminal } = context;

        // print where the installation answers, as a development server prints its address
        const launcher = await context.launcher();
        const url = await launcher.origin(
            await launcher.address(await context.requireSpace()),
            name,
        );
        await terminal.print(
            json
                ? JSON.stringify({ id, name, url, ...target.output })
                : terminal.event(
                      "Developing",
                      "running",
                      `installation '${name}' ${url}`,
                      target.detail,
                  ),
        );

        // show the installation at each change, failures included, while pushing each change, until interrupted
        using interrupt = new Interrupt(context.environment.process);
        const showing = (async () => {
            for await (const status of InstallationStatus.follow(id, context, interrupt.signal)) {
                await terminal.print(status.show(json, context));
            }
        })();
        await checkout.watch(context, interrupt.signal, async (commit) => {
            await terminal.print(
                json
                    ? JSON.stringify({ id, name, commit })
                    : terminal.event(
                          "Pushed",
                          "done",
                          `commit '${commit}'`,
                          `installation '${name}'`,
                      ),
            );
        });
        await showing;
        interrupt.require();
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

    // push the checkout's working tree, or its HEAD for a release, which refuses uncommitted changes
    const checkout = await LocalCheckout.open(directory, context);
    const { commit } = await checkout.push(context, version === "release" ? "head" : "tree");

    // read the space and the outputs
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);
    const named = CommandOption.string(values["outputs"], "outputs");
    const read = await readPackageJson(
        await context.environment.files.realPath(directory),
        context.environment.files,
    );
    const implied = outputsOf(read);
    const outputs = named?.split(",") ?? Object.keys(implied);

    // create the build of the pushed commit, which the space's runner compiles
    const created = await stack.build.create({
        spaceId: space,
        requestId: RequestId.create(environment),
        source: {
            kind: "commit",
            repository: checkout.repository,
            commit,
            directory: checkout.path,
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

/** Show each state of a space's build until it finishes, leaving it running on an interrupt. */
async function follow(
    context: CommandContext,
    id: Identifier<"build">,
    json: boolean,
): Promise<stackObject.Build> {
    // copy the space's builds until an interrupt, which leaves the build running for whoever shares it
    const space = await context.requireSpace();
    const client = await context.native();
    const launcher = await context.launcher();
    await using opened = new AsyncDisposableStack();
    const builds = await client.open(launcher, { space }, { build: stackObject.build }, opened);
    using interrupt = new Interrupt(context.environment.process);

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

    // end as interrupted, else refuse a build followed no longer
    interrupt.require();
    throw CliError.of(
        "FOLLOW_UNAVAILABLE",
        `build '${id}' is unavailable: following stopped before it finished`,
    );
}

/** Render a build's progress, or how it finished. */
function render(current: stackObject.Build): string {
    // show a running build
    if (current.state === "running") {
        return "Building";
    }
    // show how a finished build ended
    else {
        return `Build ${current.state}`;
    }
}
