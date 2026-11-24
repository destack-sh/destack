import { join } from "node:path";
import objectPackage from "@destack/object/package";
import { PackageDefinition, type PackageId, PackageName } from "@destack/package";
import { ForgeError } from "@destack/forge/error";
import { LATEST_TAG, reference, release, tag } from "@destack/registry/object";
import type { Lease } from "@destack/resource";
import { type Identifier, present, schema, Version } from "@destack/schema";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { ObjectReference, RequestId } from "@destack/sync";
import { CommandOption, type CommandBody } from "../command/command.ts";
import type { CommandContext, CommandEnvironment } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "./status.ts";

/** The separator of a package specifier's version or tag. */
const VERSION_SEPARATOR = "@";

/** The separator of a repository specifier's branch. */
const BRANCH_SEPARATOR = "#";

/** The prefix of a branch's full reference name. */
const BRANCH_PREFIX = "refs/heads/";

/** The installation argument of plan, approve and reject, the space's stack by default. */
const INSTALLATION_ARGUMENT = {
    name: "installation",
    usage: "name",
    default: STACK_INSTALLATION,
} as const;

/** What install reads: a package's release or tag, or a repository branch's package directory. */
export type PackageSpecifier =
    | {
          /** A package's release by version, or the release a tag points at. */
          readonly kind: "release";
          /** The package's name, such as @ada/tasks. */
          readonly name: string;
          /** The version or tag, latest by default. */
          readonly release: string;
      }
    | {
          /** A repository branch's package directory. */
          readonly kind: "branch";
          /** The repository's name, such as @ada/apps. */
          readonly repository: string;
          /** The package directory within it, its root by default. */
          readonly directory: string;
          /** The branch. */
          readonly branch: string;
      };

/** Read package specifiers. */
export const PackageSpecifier = {
    /** Read `@scope/name[@version|@tag]` or `@handle/repository[/directory]#branch`. */
    read(text: string): PackageSpecifier {
        // read a repository's branch after its separator
        if (text.includes(BRANCH_SEPARATOR)) {
            const [path = "", branch = ""] = text.split(BRANCH_SEPARATOR);
            const [handle = "", name = "", ...below] = path.split("/");
            const repository = `${handle}/${name}`;
            if (!PackageName.safeParse(repository).success || branch === "") {
                throw CliError.of(
                    "USAGE_INVALID",
                    `repository '${text}' is invalid: use @handle/repository[/directory]#branch`,
                );
            }

            return {
                kind: "branch",
                repository,
                directory: below.length === 0 ? "." : below.join("/"),
                branch,
            };
        }

        // read a package's version or tag after its name's leading separator
        const separator = text.lastIndexOf(VERSION_SEPARATOR);
        const name = separator > 0 ? text.slice(0, separator) : text;
        if (!PackageName.safeParse(name).success) {
            throw CliError.of(
                "USAGE_INVALID",
                `package '${text}' is invalid: use @scope/name[@version]`,
            );
        }

        return {
            kind: "release",
            name,
            release: separator > 0 ? text.slice(separator + 1) : LATEST_TAG,
        };
    },

    /** Read `@scope/name[@version|@tag]`, refusing a repository's branch. */
    release(text: string): Extract<PackageSpecifier, { readonly kind: "release" }> {
        const specifier = PackageSpecifier.read(text);
        if (specifier.kind !== "release") {
            throw CliError.of(
                "USAGE_INVALID",
                `package '${text}' is invalid: use @scope/name[@version], not a branch`,
            );
        }

        return specifier;
    },
};

/** Install a package or change its version, and follow it until its space serves it. */
export const installCommand: CommandBody = {
    arguments: [
        {
            name: "package",
            usage: "@scope/name[@version|@tag] or @handle/repository[/directory]#branch",
        },
    ],
    options: {
        name: {
            type: "string",
            description: "The installation's name in the space, the package's name by default",
            argument: "name",
        },
        yes: { type: "boolean", description: "Approve each plan the installation waits on" },
    },
    isJson: true,
    async run({ arguments: { package: given }, options, isJson: json }, context) {
        // follow a repository's branch, or a package's release or tag
        const specifier = present(given, "the package argument");
        const named = CommandOption.string(options["name"], "name");
        const read = PackageSpecifier.read(specifier);
        const target =
            read.kind === "branch"
                ? await branchTarget(read, context)
                : await releaseTarget(read, context);

        // install it, and follow it until its space serves it
        const { id, name } = await upsert(context, target, named);
        await InstallationStatus.drive(id, options["yes"] === true, json, context);

        // report what it runs and its version
        await context.terminal.print(
            json
                ? JSON.stringify({ id, name, ...target.output })
                : context.terminal.event(
                      "Installed",
                      "done",
                      `installation '${name}'`,
                      target.detail,
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

/** Show an installation, the space's stack by default, its resources and the plan it waits on. */
export const planCommand: CommandBody = {
    arguments: [INSTALLATION_ARGUMENT],
    isJson: true,
    options: { watch: { type: "boolean", description: "Follow it as it changes" } },
    async run({ arguments: { installation: name }, options, isJson: json }, context) {
        // show it once, or each change when watched
        const space = await context.requireSpace();
        const stack = await context.package(stackPackage, await context.locate());
        const { id } = await stack.installation.get({
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
export const approveCommand: CommandBody = decideCommand("approve");

/** Reject the plan an installation, the space's stack by default, waits on. */
export const rejectCommand: CommandBody = decideCommand("reject");

/** Decide the plan an installation waits on through its change request, by the reviewed digest. */
function decideCommand(decision: "approve" | "reject"): CommandBody {
    return {
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
            // require the reviewed plan's digest
            const plan = CommandOption.string(options["plan"], "plan");
            if (plan === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    "usage is invalid: give the reviewed plan's digest with --plan",
                );
            }

            // read the plan the installation waits on, refusing another digest
            const space = await context.requireSpace();
            const location = await context.locate();
            const stack = await context.package(stackPackage, location);
            const { id } = await stack.installation.get({
                scope: space,
                id: present(name, "the installation argument"),
            });
            const waiting = await stack.installation.plan({ scope: space, id });
            if (waiting.digest !== plan || waiting.approval === null) {
                throw CliError.of(
                    "PLAN_BLOCKED",
                    `cannot apply plan '${plan}': '${name}' waits on no change request for it`,
                );
            }

            // decide its change request's approval
            const objects = await context.package(objectPackage, location);
            await objects.approval[decision]({
                scope: space,
                id: waiting.approval,
                requestId: RequestId.create(context.environment),
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

/** Find a package by its name at its forge. */
async function packageOf(name: string, context: CommandContext): Promise<PackageId> {
    const { client, home } = await context.registry(name);

    return (await client.package.resolve({ spaceId: home, name })).id;
}

/** The version an installation runs: the package, its version's row there, and what to report. */
interface InstallTarget {
    /** The installed package. */
    readonly packageId: PackageId;
    /** The release, tag or branch reference it runs. */
    readonly version: ObjectReference;
    /** The package's directory in a followed repository, absent for a release. */
    readonly directory: string | null;
    /** The installation's name unless one is asked for. */
    readonly fallback: string;
    /** What the JSON output reports of it. */
    readonly output: Readonly<Record<string, string | null>>;
    /** What the terminal output says it runs and follows. */
    readonly detail: string;
}

/** Resolve a package's release or tag: a version pins its release, else the tag updates it. */
async function releaseTarget(
    specifier: Extract<PackageSpecifier, { readonly kind: "release" }>,
    context: CommandContext,
): Promise<InstallTarget> {
    // take the package's name and the release or tag asked for
    const { name: packageName, release: asked } = specifier;

    // resolve the release at the package's forge
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
    const released = resolved.version;

    // pin the release for a version, else run the tag
    const isPinned = Version.safeParse(asked).success;
    const version = isPinned
        ? release.reference(resolved.scope, resolved.id)
        : await taggedRow(client, home, found.id, asked);
    const tagged = isPinned ? null : asked;
    const updates = tagged === null ? "pinned" : `${tagged}, updates automatically`;

    return {
        packageId: found.id,
        version,
        directory: null,
        fallback: PackageName.split(packageName).local,
        output: { package: packageName, version: released, tag: tagged },
        detail: `${packageName} ${released} · ${updates}`,
    };
}

/** Resolve the package at a repository branch's directory, which updates on each push. */
async function branchTarget(
    specifier: Extract<PackageSpecifier, { readonly kind: "branch" }>,
    context: CommandContext,
): Promise<InstallTarget> {
    // find the repository at its account's forge
    const { repository: named, directory, branch } = specifier;
    const { client, home, accountId } = await context.registry(named);
    const local = PackageName.split(named).local;
    const repositories = await client.repository.list({
        spaceId: home,
        where: { accountId, name: local },
        limit: 1,
    });
    const [found] = repositories.items;
    if (found === undefined) {
        throw CliError.of("REPOSITORY_NOT_FOUND", `repository '${named}' not found`);
    }

    // find the branch's reference
    const references = await client.reference.list({
        spaceId: home,
        where: {
            parentId: found.id,
            name: `${BRANCH_PREFIX}${branch}`,
            deletedAt: { isNull: true },
        },
        limit: 1,
    });
    const [row] = references.items;
    if (row === undefined) {
        throw CliError.of("BRANCH_NOT_FOUND", `branch '${branch}' of '${named}' not found`);
    }

    // read the package at the branch through a read lease
    const lease = await client.repository.open({ spaceId: home, id: found.id, mode: "read" });
    const read = (path: string) =>
        readAtBranch(lease, branch, join(directory, path), context.environment);
    const definition = PackageDefinition.read(await read("destack.json"));
    const manifest = schema.looseObject({ name: PackageName });
    const packageName = manifest.parse(JSON.parse(await read("package.json"))).name;

    return {
        packageId: definition.id,
        version: reference.reference(row.scope, row.id),
        directory,
        fallback: PackageName.split(packageName).local,
        output: { package: packageName, repository: named, branch, directory },
        detail: `${packageName} from ${named}#${branch} · updates on push`,
    };
}

/** Read a file at a branch's commit through a read lease, fetching that commit alone. */
async function readAtBranch(
    lease: Lease,
    branch: string,
    path: string,
    environment: CommandEnvironment,
): Promise<string> {
    // fetch the branch's commit into an empty repository, sending the lease's headers
    const directory = await environment.files.makeTemporaryDirectory("destack-branch-");
    const headers = Object.entries(lease.headers).flatMap(([name, value]) => [
        "-c",
        `http.extraHeader=${name}: ${value}`,
    ]);

    // run a git command in it, failing with git's message without the lease's address
    const git = async (verb: string, ...words: readonly string[]) => {
        // run the command, reading its output
        const command = ["git", "-C", directory, ...headers, verb, ...words];
        const { code, stdout, stderr } = await environment.process.run(command);

        // fail with git's message, its address left out
        if (code !== 0) {
            const message = new TextDecoder()
                .decode(stderr)
                .trim()
                .replaceAll(lease.url, "the lease");
            throw CliError.of(
                "GIT_FETCH_BROKEN",
                `git '${verb}' exited with status ${code}: ${message}`,
            );
        }

        return new TextDecoder().decode(stdout);
    };

    // read the file at the fetched commit, removing the repository after
    try {
        await git("init", "--quiet", "--bare");
        await git("fetch", "--quiet", "--depth", "1", lease.url, `${BRANCH_PREFIX}${branch}`);

        return await git("show", `FETCH_HEAD:${path}`);
    } finally {
        await environment.files.remove(directory, { recursive: true });
    }
}

/** Read the tag of a package an installation runs, refusing a tag the package lacks. */
async function taggedRow(
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

/** Create a package's installation under a name, or change the version its installed one runs. */
async function upsert(
    context: CommandContext,
    target: InstallTarget,
    asked: string | undefined,
): Promise<{ readonly id: Identifier<"installation">; readonly name: string }> {
    // read the package's installation, of which a space has one, and the one of that name
    const { environment } = await context.native();
    const space = await context.requireSpace();
    const stack = await context.package(stackPackage, await context.locate());
    const find = async (where: { readonly packageId: PackageId } | { readonly name: string }) =>
        (await stack.installation.list({ scope: space, where, limit: 1 })).items[0];
    const { packageId, version, directory } = target;
    const installed = await find({ packageId });
    const name = asked ?? installed?.name ?? target.fallback;

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
            ...ObjectReference.columns("version", version),
            ...(directory === null ? {} : { directory }),
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
    // change the version it runs
    else {
        await stack.installation.update({
            scope: space,
            id: installed.id,
            requestId: RequestId.create(environment),
            revision: installed.revision,
            ...ObjectReference.columns("version", version),
            directory,
        });

        return { id: installed.id, name };
    }
}
