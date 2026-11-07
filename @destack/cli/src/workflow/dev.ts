import { outputsOf } from "@destack/build";
import { readPackageJson } from "@destack/build/local";
import type { PackageJson } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import * as stackObject from "@destack/stack/object";
import { type Verb, OptionValue, type Options } from "../command/command.ts";
import type { Context } from "../context/context.ts";

import { Forge } from "./forge.ts";
import stackPackage from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";
import { Interrupt } from "../command/error.ts";

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
export function devCommand(context: Context): Verb {
    return {
        description: "Preview a checkout's package live against the space",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        json: true,
        async run({ arguments: { package: directory }, json }) {
            await preview(context, present(directory, "the package argument"), json);
        },
    };
}

/** Build a checkout's package in the space, following its phases. */
export function buildCommand(context: Context): Verb {
    return {
        description: "Build a checkout's package in the space, following its phases",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        json: true,
        options: OUTPUTS_OPTION,
        async run({ arguments: { package: directory }, options, json }) {
            const built = await build(
                context,
                present(directory, "the package argument"),
                json,
                options,
            );
            await context.terminal.print(json ? JSON.stringify(built) : `Built ${built.manifest}.`);
        },
    };
}

/** Build a checkout's package in the space and publish the build to the registry. */
export function publishCommand(context: Context): Verb {
    return {
        description: "Build a checkout's package in the space and publish it to the registry",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        json: true,
        options: {
            ...OUTPUTS_OPTION,
            tag: {
                type: "string",
                description: "The distribution tag pointing at the release, latest by default",
                argument: "name",
            },
        },
        async run({ arguments: { package: directory }, options, json }) {
            await publish(context, present(directory, "the package argument"), json, options);
        },
    };
}

/** Build a checkout's package in a space, push the build to the registry and release it. */
async function publish(
    context: Context,
    directory: string,
    json: boolean,
    values: Options,
): Promise<void> {
    // draw request identifiers from the device's environment
    const { environment } = await context.native();

    // build the package, and open the forge of its account
    const built = await build(context, directory, json, values);
    const described = built.package;
    const forge = await Forge.open(described.name, context);

    // create the package on its first publish
    await forge.createPackage(described);

    // push the build from its space to the package's forge and release it there
    const stack = await context.package(stackPackage, await context.locate());
    await stack.build.push({
        spaceId: built.space,
        id: built.id,
        requestId: RequestId.create(environment),
    });
    const tag = OptionValue.string(values["tag"], "tag");
    const released = await forge.client.release.create({
        spaceId: forge.home,
        requestId: RequestId.create(environment),
        parentId: described.id,
        manifest: built.manifest,
        ...(tag === undefined ? {} : { tag }),
    });
    await context.terminal.print(
        json ? JSON.stringify(released) : `Published ${described.name}@${released.version}.`,
    );
}

/** Build the checkout containing a directory in a space. */
async function build(
    context: Context,
    directory: string,
    json: boolean,
    values: Options,
): Promise<{
    readonly space: string;
    readonly id: Identifier<"build">;
    readonly manifest: string;
    readonly package: PackageJson["package"];
}> {
    // draw request identifiers from the device's environment
    const { environment } = await context.native();
    const space = await context.requireSpace();

    // find the package directory in its checkout, the space and the outputs
    const registered = await context.registerCheckout(directory);
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);
    const named = OptionValue.string(values["outputs"], "outputs");
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
    });

    // follow it until it finishes
    const finished = await follow(context, created.id, json);

    return {
        space,
        id: created.id,
        manifest: stackObject.Build.manifest(finished),
        package: read.package,
    };
}

/** Show each phase of a space's build until it finishes, cancelling it on an interrupt. */
async function follow(
    context: Context,
    id: Identifier<"build">,
    json: boolean,
): Promise<stackObject.Build> {
    // open the space's builds
    const space = await context.requireSpace();
    const location = await context.locate();
    const stack = await context.package(stackPackage, location);

    // copy the space's builds, cancelling the build on an interrupt
    const client = await context.native();
    const person = await context.person();
    await using opened = new AsyncDisposableStack();
    const builds = await person.open({ space }, { build: stackObject.build }, opened);
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
    throw CliError.of("FOLLOW_STOPPED", `stopped following build ${id} before it finished`);
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
async function preview(context: Context, directory: string, json: boolean): Promise<void> {
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
        login: (await context.person()).login.row.id,
    });
    await context.terminal.print(
        json ? JSON.stringify(created) : `Previewing ${created.package} (${created.id}).`,
    );
}
