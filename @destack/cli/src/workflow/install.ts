import { LATEST_TAG } from "@destack/registry/object";
import { PackageName } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import { installation } from "@destack/object";
import { RelationalQuery } from "@destack/object/client";
import { Rollout, rollout } from "@destack/stack/object";
import objectPackage from "@destack/object/package";
import { RequestId } from "@destack/sync";
import { type Verb, OptionValue } from "../command/command.ts";

import type { Context } from "../context/context.ts";

import { Interrupt } from "../command/error.ts";
import { CliError } from "../error/index.ts";
import { Forge } from "./forge.ts";

/** The separator of a package specifier's version. */
const VERSION_SEPARATOR = "@";

/** Install a package's release in the space. */
export function installCommand(context: Context): Verb {
    return {
        description:
            "Install a package's release in the space, the latest by default, following that release",
        arguments: [{ name: "package", usage: "@scope/name[@version]" }],
        options: {
            name: {
                type: "string",
                description: "The installation's name in the space, the package's name by default",
                argument: "name",
            },
        },
        json: true,
        async run({ arguments: { package: specifier }, options, json }) {
            // read the package's name, the release asked for and the installation's name
            const { name: packageName, release } = readSpecifier(
                present(specifier, "the package argument"),
            );
            const name =
                OptionValue.string(options["name"], "name") ?? PackageName.split(packageName).local;

            // resolve the release, install it, and follow it until it serves the release
            const forge = await Forge.open(packageName, context);
            const resolved = await forge.resolve(packageName, release);
            const id = await install(context, resolved, name);
            await follow(context, id);
            const { version } = resolved.release;
            await context.terminal.print(
                json
                    ? JSON.stringify({ id, name, package: packageName, version })
                    : `Installed ${packageName}@${version} as ${name}.`,
            );
        },
    };
}

/** Read a package specifier `@scope/name[@version]`: the name, and the version or latest. */
export function readSpecifier(specifier: string): {
    readonly name: string;
    readonly release: string;
} {
    // split the version after the name's leading separator
    const separator = specifier.lastIndexOf(VERSION_SEPARATOR);
    const name = separator > 0 ? specifier.slice(0, separator) : specifier;
    if (!PackageName.safeParse(name).success) {
        throw CliError.of(
            "USAGE_INVALID",
            `${specifier} is not a package: use @scope/name[@version]`,
        );
    }

    return { name, release: separator > 0 ? specifier.slice(separator + 1) : LATEST_TAG };
}

/** Create an installation following a release, pinned to the manifest the forge resolved. */
async function install(
    context: Context,
    resolved: Awaited<ReturnType<Forge["resolve"]>>,
    name: string,
): Promise<Identifier<"installation">> {
    // create the installation following the release in the space, which its controller deploys
    const { environment } = await context.native();
    const space = await context.requireSpace();
    const objects = await context.package(objectPackage, await context.locate());
    const { version, manifest } = resolved.release;
    const created = await objects.installation.create({
        scope: space,
        requestId: RequestId.create(environment),
        packageId: resolved.packageId,
        selection: { kind: "release", version, manifest },
        name,
    });

    return created.id;
}

/** Follow an installation until it serves its release, showing its waits and refusing a failure. */
async function follow(context: Context, id: Identifier<"installation">): Promise<void> {
    // follow the installation and its rollout as the person until interrupted
    const space = await context.requireSpace();
    const person = await context.person();
    await using opened = new AsyncDisposableStack();
    const { query } = await person.open({ space }, { installation, rollout }, opened);
    using interrupt = new Interrupt(context.environment.process);
    const status = RelationalQuery.latest(
        {
            installed: query.installation.findFirst({ where: { id } }),
            rolled: query.rollout.findFirst({ where: { installationId: id } }),
        },
        interrupt.signal,
    );

    // end once it serves its release, refuse a failed rollout, and show each new reason it waits on
    let shown: string | undefined;
    for await (const { installed, rolled } of status) {
        // read how far the rollout is, and the readiness it reports
        const target =
            installed === undefined || rolled === undefined
                ? undefined
                : Rollout.join(rolled, installed);
        const progress = target === undefined ? "waiting" : Rollout.progress(target);
        const ready = target?.installation.conditions["Ready"];

        // end once it serves its release
        if (progress === "served") {
            return;
        }
        // refuse a rollout that failed, with its reason
        else if (progress === "failed") {
            throw CliError.of(
                "INSTALLATION_FAILED",
                `installation ${id} failed: ${ready?.reason} ${ready?.message}`,
            );
        }
        // show a reason it waits on once
        else if (ready !== undefined && ready.status === "false" && ready.reason !== shown) {
            shown = ready.reason;
            await context.terminal.printDiagnostic(ready.message);
        }
    }

    // stop following once interrupted
    interrupt.require();
    throw CliError.of("FOLLOW_STOPPED", `stopped following installation ${id} before it served`);
}
