import { ForgeError } from "@destack/forge/error";
import { LATEST_TAG, release, tag } from "@destack/registry/object";
import { type PackageId, PackageName } from "@destack/package";
import { type Identifier, present, Version } from "@destack/schema";
import stackPackage from "@destack/stack/package";
import { ObjectReference, RequestId } from "@destack/sync";
import { CommandOption, type CommandBody } from "../command/command.ts";
import type { CommandContext } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "../installation/status.ts";

/** The separator of a package specifier's version. */
const VERSION_SEPARATOR = "@";

/** Install a package or change what it follows, and follow it until its space serves it. */
export const installCommand: CommandBody = {
    arguments: [{ name: "package", usage: "@scope/name[@version|@tag]" }],
    options: {
        name: {
            type: "string",
            description: "The installation's name in the space, the package's name by default",
            argument: "name",
        },
        yes: { type: "boolean", description: "Approve each plan the installation waits on" },
    },
    isJson: true,
    async run({ arguments: { package: specifier }, options, isJson: json }, context) {
        // read the package's name, the release or tag asked for and the installation's name
        const { name: packageName, release: asked } = readSpecifier(
            present(specifier, "the package argument"),
        );
        const named = CommandOption.string(options["name"], "name");

        // resolve the release, stay on a version or follow a tag, and follow it until its space serves it
        const { client, home } = await context.registry(packageName);
        const found = await client.package.resolve({ spaceId: home, name: packageName });
        const resolved = await client.release
            .resolve({ spaceId: home, parentId: found.id, release: asked })
            .catch((error: unknown) => {
                // name the package the person asked for, as the forge names it by its id
                if (ForgeError.is(error, "RELEASE_NOT_FOUND")) {
                    throw CliError.of(
                        "RELEASE_NOT_FOUND",
                        `release '${asked}' of '${packageName}' not found`,
                        { cause: error },
                    );
                }
                throw error;
            });
        const { version } = resolved;

        // follow the release for a version, else the tag
        const isPinned = Version.safeParse(asked).success;
        const follows = isPinned
            ? release.reference(resolved.scope, resolved.id)
            : await followedTag(client, home, found.id, asked);
        const fallback = PackageName.split(packageName).local;
        const { id, name } = await upsert(context, found.id, named, fallback, follows);
        await InstallationStatus.drive(id, options["yes"] === true, json, context);

        // report the release it runs and what it follows
        const tagged = isPinned ? null : asked;
        const followed = tagged === null ? "staying on it" : `following ${tagged}`;
        await context.terminal.print(
            json
                ? JSON.stringify({ id, name, package: packageName, version, tag: tagged })
                : context.terminal.event(
                      "Installed",
                      "done",
                      `installation '${name}'`,
                      `${packageName}@${version}, ${followed}`,
                  ),
        );
    },
};

/** Uninstall a package's installation, or the one of a name, following its removal. */
export const uninstallCommand: CommandBody = {
    arguments: [{ name: "installation", usage: "@scope/name|name" }],
    options: {
        yes: { type: "boolean", description: "Uninstall without asking" },
    },
    isJson: true,
    async run({ arguments: { installation: given }, options, isJson: json }, context) {
        // find the package's one installation in the space, or the one of the name
        const asked = present(given, "the installation argument");
        const space = await context.requireSpace();
        const stack = await context.package(stackPackage, await context.locate());
        const where = PackageName.safeParse(asked).success
            ? { packageId: await packageOf(asked, context) }
            : { name: asked };
        const { items } = await stack.installation.list({ scope: space, where, limit: 1 });
        const [installed] = items;
        if (installed === undefined) {
            throw CliError.of(
                "INSTALLATION_NOT_FOUND",
                `installation '${asked}' of '${space}' not found`,
            );
        }

        // ask before deleting its resources, on a terminal or with --yes
        const { name } = installed;
        const isConfirmed =
            options["yes"] === true ||
            (context.environment.process.stdin.isTerminal &&
                (await context.terminal.prompt.confirm(
                    `Uninstall installation '${name}' and delete its resources?`,
                )));
        if (!isConfirmed) {
            throw CliError.of(
                "COMMAND_BLOCKED",
                `cannot uninstall '${name}': it deletes the installation's resources, so confirm it with --yes`,
            );
        }

        // delete it, then follow it until its controller removed it with its resources
        const { environment } = await context.native();
        await stack.installation.delete({
            scope: space,
            id: installed.id,
            requestId: RequestId.create(environment),
        });
        await InstallationStatus.followRemoval(installed.id, context);
        await context.terminal.print(
            json
                ? JSON.stringify({ id: installed.id, name })
                : context.terminal.event("Uninstalled", "done", `installation '${name}'`),
        );
    },
};

/** Find a package by its name at its forge. */
async function packageOf(name: string, context: CommandContext): Promise<PackageId> {
    const { client, home } = await context.registry(name);

    return (await client.package.resolve({ spaceId: home, name })).id;
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
            `package '${specifier}' is invalid: use @scope/name[@version]`,
        );
    }

    return { name, release: separator > 0 ? specifier.slice(separator + 1) : LATEST_TAG };
}

/** Read the tag of a package an installation follows, refusing a tag the package lacks. */
async function followedTag(
    client: Awaited<ReturnType<CommandContext["registry"]>>["client"],
    home: string,
    packageId: PackageId,
    name: string,
): Promise<ObjectReference> {
    // read the tag by its name at the package's forge
    const { items } = await client.tag.list({
        spaceId: home,
        where: { parentId: packageId, name },
        limit: 1,
    });
    const [tagged] = items;
    if (tagged === undefined) {
        throw CliError.of("RELEASE_NOT_FOUND", `tag '${name}' of '${packageId}' not found`);
    }

    return tag.reference(tagged.scope, tagged.id);
}

/** Create a package's installation under a name, or change what its installed one follows. */
async function upsert(
    context: CommandContext,
    packageId: PackageId,
    asked: string | undefined,
    fallback: string,
    follows: ObjectReference,
): Promise<{ readonly id: Identifier<"installation">; readonly name: string }> {
    // read the package's installation, of which a space has one, and the one of that name
    const { environment } = await context.native();
    const space = await context.requireSpace();
    const stack = await context.package(stackPackage, await context.locate());
    const find = async (where: { readonly packageId: PackageId } | { readonly name: string }) =>
        (await stack.installation.list({ scope: space, where, limit: 1 })).items[0];
    const installed = await find({ packageId });
    const name = asked ?? installed?.name ?? fallback;

    // create it, refusing a name another package's installation has
    if (installed === undefined) {
        if ((await find({ name })) !== undefined) {
            throw CliError.of(
                "INSTALLATION_NAME_EXISTS",
                `installation '${name}' already exists: it installs another package, so choose another name with --name`,
            );
        }
        const created = await stack.installation.create({
            scope: space,
            requestId: RequestId.create(environment),
            packageId,
            ...ObjectReference.columns("follows", follows),
            name,
        });

        return { id: created.id, name };
    }
    // refuse another name for the package's one installation
    else if (installed.name !== name) {
        throw CliError.of(
            "INSTALLATION_NAME_EXISTS",
            `installation '${installed.name}' already exists: it is the package's one installation in the space, so leave --name out`,
        );
    }
    // change what it follows
    else {
        await stack.installation.update({
            scope: space,
            id: installed.id,
            requestId: RequestId.create(environment),
            revision: installed.revision,
            ...ObjectReference.columns("follows", follows),
            directory: null,
        });

        return { id: installed.id, name };
    }
}
