import { LATEST_TAG } from "@destack/registry/object";
import { type PackageId, PackageName } from "@destack/package";
import { type Identifier, present, Version } from "@destack/schema";
import type { InstallationSelection } from "@destack/object";
import objectPackage from "@destack/object/package";
import { RequestId } from "@destack/sync";
import { type Verb, OptionValue } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { CliError } from "../error/index.ts";
import { openForge } from "./forge.ts";
import { driveInstallation } from "./status.ts";

/** The separator of a package specifier's version. */
const VERSION_SEPARATOR = "@";

/** Install a package or change what it follows, and follow it until its space serves it. */
export function installCommand(context: Context): Verb {
    return {
        description:
            "Install a package, following its latest release or a tag, or staying on a version",
        arguments: [{ name: "package", usage: "@scope/name[@version|@tag]" }],
        options: {
            name: {
                type: "string",
                description: "The installation's name in the space, the package's name by default",
                argument: "name",
            },
            yes: { type: "boolean", description: "Approve each plan the installation waits on" },
        },
        json: true,
        async run({ arguments: { package: specifier }, options, json }) {
            // read the package's name, the release or tag asked for and the installation's name
            const { name: packageName, release } = readSpecifier(
                present(specifier, "the package argument"),
            );
            const name =
                OptionValue.string(options["name"], "name") ?? PackageName.split(packageName).local;

            // resolve the release, stay on a version or follow a tag, and follow it until its space serves it
            const { client, home } = await openForge(packageName, context);
            const found = await client.package.resolve({ spaceId: home, name: packageName });
            const { version, manifest } = await client.release.resolve({
                spaceId: home,
                parentId: found.id,
                release,
            });
            const selection: InstallationSelection = Version.safeParse(release).success
                ? { kind: "release", version, manifest }
                : { kind: "tag", tag: release };
            const id = await upsert(context, found.id, name, selection);
            await driveInstallation(id, options["yes"] === true, json, context);

            // report the release it runs and what it follows
            const follows =
                selection.kind === "tag" ? `following ${selection.tag}` : "staying on it";
            await context.terminal.print(
                json
                    ? JSON.stringify({ id, name, package: packageName, version, selection })
                    : `Installed ${packageName}@${version} as ${name}, ${follows}.`,
            );
        },
    };
}

/** Read a specifier `@scope/name[@version|@tag]`: the name, and the version, tag or latest. */
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

/** Create an installation of a package by its name, or change what the installed one follows. */
async function upsert(
    context: Context,
    packageId: PackageId,
    name: string,
    selection: InstallationSelection,
): Promise<Identifier<"installation">> {
    // read the installation of that name
    const { environment } = await context.native();
    const space = await context.requireSpace();
    const objects = await context.package(objectPackage, await context.locate());
    const { items } = await objects.installation.list({ scope: space, where: { name }, limit: 1 });
    const [installed] = items;

    // create it
    if (installed === undefined) {
        const created = await objects.installation.create({
            scope: space,
            requestId: RequestId.create(environment),
            packageId,
            selection,
            name,
        });

        return created.id;
    }
    // refuse a name another package's installation has
    else if (installed.packageId !== packageId) {
        throw CliError.of(
            "INSTALLATION_NAME_TAKEN",
            `${name} installs another package: choose another name with --name`,
        );
    }
    // change what it follows
    else {
        await objects.installation.update({
            scope: space,
            id: installed.id,
            requestId: RequestId.create(environment),
            revision: installed.revision,
            selection,
        });

        return installed.id;
    }
}
