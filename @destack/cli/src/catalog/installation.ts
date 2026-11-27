import { GitReference } from "@destack/git";
import { join } from "node:path";
import objectPackage from "@destack/object/package";
import { PackageDefinition, type PackageId, PackageName } from "@destack/package";
import { ForgeError } from "@destack/forge/error";
import { LATEST_TAG, reference, release, tag } from "@destack/registry/object";
import { GitLease } from "@destack/git/local";
import type { Lease } from "@destack/resource";
import { type Identifier, type JsonObject, present, schema, Version } from "@destack/schema";
import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import { ObjectReference, RequestId } from "@destack/sync";
import { type CommandBody, type CommandCall, CommandOption } from "../command/command.ts";
import { HOME_APPLICATION, MAIN_APPLICATION } from "@destack/account/object";
import type { CommandContext, CommandEnvironment } from "../command/context.ts";
import { CliError } from "../error/index.ts";
import { InstallationStatus } from "./status.ts";
import { LocalCheckout } from "../checkout/checkout.ts";

/** The separator of a package specifier's version or tag. */
const VERSION_SEPARATOR = "@";

/** The separator of a repository specifier's branch. */
const BRANCH_SEPARATOR = "#";

/** A directory argument: one starting with `.` or `/`, such as `.`, `./site` or `/work/site`. */
const DIRECTORY_PATTERN = /^[./]/u;

/** The installation argument of plan, approve and reject, the space's stack by default. */
const INSTALLATION_ARGUMENT = {
    name: "installation",
    usage: "name",
    default: STACK_INSTALLATION,
} as const;

/** What install reads: a release or tag, or a package directory of a branch or a checkout. */
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
      }
    | {
          /** A package directory of a checkout on this computer. */
          readonly kind: "checkout";
          /** The directory, relative to the working directory or absolute. */
          readonly directory: string;
      };

/** Read package specifiers. */
export const PackageSpecifier = {
    /** Read `@scope/name[@version|@tag]`, `@handle/repository[/directory]#branch` or `./dir`. */
    read(text: string): PackageSpecifier {
        // read a checkout's directory by its path
        if (DIRECTORY_PATTERN.test(text)) {
            return { kind: "checkout", directory: text };
        }
        // read a repository's branch after its separator
        else if (text.includes(BRANCH_SEPARATOR)) {
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

/** The option placing an installation below a parent, home by default, else at the top. */
export const INSTALLATION_PARENT_OPTIONS = {
    parent: {
        type: "string",
        description:
            "The installation it is placed below, home by default; --no-parent places it at the top",
        argument: "name",
        isNegatable: true,
    },
} as const;

/** The parent a command asks for: home for a new installation by default, none, or a named one. */
export type InstallationParentOption =
    | { readonly kind: "default" }
    | { readonly kind: "none" }
    | { readonly kind: "named"; readonly name: string };

/** Parent options read from a command's options. */
export const InstallationParentOption = {
    /** Read the parent the options ask for: a name, none, or the default. */
    read(options: CommandCall["options"]): InstallationParentOption {
        const parent = options["parent"];

        return parent === false
            ? { kind: "none" }
            : typeof parent === "string"
              ? { kind: "named", name: parent }
              : { kind: "default" };
    },
};

/** Install a package or change its version, and follow it until its space serves it. */
export const installCommand: CommandBody = {
    arguments: [
        {
            name: "package",
            usage: "@scope/name[@version|@tag], @handle/repository[/directory]#branch or ./directory",
        },
    ],
    options: {
        name: {
            type: "string",
            description: "The installation's name in the space, the package's name by default",
            argument: "name",
        },
        ...INSTALLATION_PARENT_OPTIONS,
        standalone: {
            type: "boolean",
            description:
                "Install it apart from the space's main installation, on an origin of its own",
        },
        yes: { type: "boolean", description: "Approve each plan the installation waits on" },
    },
    isJson: true,
    async run({ arguments: { package: given }, options, isJson: json }, context) {
        // follow a checkout's or a repository's branch, or a package's release or tag
        const specifier = present(given, "the package argument");
        const named = CommandOption.string(options["name"], "name");
        const read = PackageSpecifier.read(specifier);
        let target: InstallationTarget;
        // push a checkout and follow its reference
        if (read.kind === "checkout") {
            target = await (await LocalCheckout.open(read.directory, context)).target(context);
        }
        // follow a repository's branch
        else if (read.kind === "branch") {
            target = await InstallationTarget.branch(read, context);
        }
        // follow a release or a tag
        else {
            target = await InstallationTarget.release(read, context);
        }

        // add a release or tag to the space's main installation, which serves it on its origin, unless asked apart
        const isApproved = options["yes"] === true;
        const main =
            read.kind === "release" && options["standalone"] !== true
                ? await target.add(isApproved, context)
                : undefined;
        if (main !== undefined) {
            await InstallationStatus.drive(main, isApproved, json, context);
            await context.terminal.print(
                json
                    ? JSON.stringify({ id: main, name: MAIN_APPLICATION, ...target.output })
                    : context.terminal.event(
                          "Added",
                          "done",
                          `to '${MAIN_APPLICATION}'`,
                          target.detail,
                      ),
            );

            return;
        }

        // install it, and follow it until its space serves it
        const { id, name } = await target.install(
            named,
            undefined,
            InstallationParentOption.read(options),
            context,
        );
        await InstallationStatus.drive(id, isApproved, json, context);

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

/** The export a stack installation evaluates, and its arguments. */
export interface InstallationStack {
    /** The exported stack. */
    readonly export: string;
    /** The arguments of a parameterised stack. */
    readonly parameters: JsonObject;
}

/** The version an installation runs: the package, its version's row there, and what to report. */
export class InstallationTarget {
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

    /** Take the package, its version and what to report. */
    constructor(
        fields: Pick<
            InstallationTarget,
            "packageId" | "version" | "directory" | "fallback" | "output" | "detail"
        >,
    ) {
        // set the package, its version and what to report
        this.packageId = fields.packageId;
        this.version = fields.version;
        this.directory = fields.directory;
        this.fallback = fields.fallback;
        this.output = fields.output;
        this.detail = fields.detail;
    }

    /** Resolve a package's release or tag: a version pins its release, else the tag updates it. */
    static async release(
        specifier: Extract<PackageSpecifier, { readonly kind: "release" }>,
        context: CommandContext,
    ): Promise<InstallationTarget> {
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

        return new InstallationTarget({
            packageId: found.id,
            version,
            directory: null,
            fallback: PackageName.split(packageName).local,
            output: { package: packageName, version: released, tag: tagged },
            detail: `${packageName} ${released} · ${updates}`,
        });
    }

    /** Resolve the package at a repository branch's directory, which updates on each push. */
    static async branch(
        specifier: Extract<PackageSpecifier, { readonly kind: "branch" }>,
        context: CommandContext,
    ): Promise<InstallationTarget> {
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
                name: GitReference.branch(branch),
                deletedAt: { isNull: true },
            },
            limit: 1,
        });
        const [row] = references.items;
        if (row === undefined) {
            throw CliError.of("BRANCH_NOT_FOUND", `branch '${branch}' of '${named}' not found`);
        }

        // read the package at the branch through a read lease
        const lease = await client.repository.pull({ spaceId: home, id: found.id });
        const read = (path: string) =>
            readAtBranch(lease, branch, join(directory, path), context.environment);
        const definition = PackageDefinition.read(await read("destack.json"));
        const manifest = schema.looseObject({ name: PackageName });
        const packageName = manifest.parse(JSON.parse(await read("package.json"))).name;

        return new InstallationTarget({
            packageId: definition.id,
            version: reference.reference(row.scope, row.id),
            directory,
            fallback: PackageName.split(packageName).local,
            output: { package: packageName, repository: named, branch, directory },
            detail: `${packageName} from ${named}#${branch} · updates on push`,
        });
    }

    /** Add the package to the space's main installation, none without a main. */
    async add(
        isApproved: boolean,
        context: CommandContext,
    ): Promise<Identifier<"installation"> | undefined> {
        // find the space's main installation, which a space without one lacks
        const space = await context.requireSpace();
        const consent = isApproved ? "approve" : "ask";
        const stacks = await context.package(stackPackage, await context.locate(), consent);
        const [main] = (
            await stacks.installation.list({
                scope: space,
                where: { name: MAIN_APPLICATION },
                limit: 1,
            })
        ).items;
        if (main === undefined) {
            return undefined;
        }

        // require the package at its tag, else its version, committing the change to main's branch
        const name = String(this.output["package"]);
        const range = this.output["tag"] ?? this.output["version"] ?? LATEST_TAG;
        const { run } = await stacks.installation.edit({
            scope: space,
            id: main.id,
            requestId: RequestId.create((await context.native()).environment),
            operations: [{ kind: "dependency.add", package: name, range }],
            message: `add ${name}`,
        });

        // wait until main serves the edit, refusing a failed one
        await InstallationStatus.edited(main.id, run, main.appliedRevisionId, context);

        return main.id;
    }

    /** Create the package's installation under a name, or change its installed one's version. */
    async install(
        asked: string | undefined,
        stack: InstallationStack | undefined,
        parent: InstallationParentOption,
        context: CommandContext,
    ): Promise<{ readonly id: Identifier<"installation">; readonly name: string }> {
        // read the package's installation, of which a space has one, and the one of that name
        const { environment } = await context.native();
        const space = await context.requireSpace();
        const stacks = await context.package(stackPackage, await context.locate());
        const find = async (where: { readonly packageId: PackageId } | { readonly name: string }) =>
            (await stacks.installation.list({ scope: space, where, limit: 1 })).items[0];
        const { packageId, version, directory } = this;
        const installed = await find({ packageId });
        const name = asked ?? installed?.name ?? this.fallback;

        // find the parent asked for, the space's home for a new installation by default
        const parentName =
            parent.kind === "named"
                ? parent.name
                : parent.kind === "default" && installed === undefined
                  ? HOME_APPLICATION
                  : undefined;
        const found = parentName === undefined ? undefined : await find({ name: parentName });
        if (parentName !== undefined && found === undefined) {
            throw CliError.of(
                "INSTALLATION_NOT_FOUND",
                `installation '${parentName}' not found: place '${name}' below an installation of the space`,
            );
        }
        const placed =
            found !== undefined
                ? { parentId: found.id }
                : parent.kind === "none"
                  ? { parentId: null }
                  : {};

        // create it, refusing a name another package's installation has
        if (installed === undefined) {
            if ((await find({ name })) !== undefined) {
                throw CliError.of(
                    "INSTALLATION_NAME_EXISTS",
                    `installation '${name}' already exists: it installs another package, so choose another name with --name`,
                );
            }
            const created = await stacks.installation.create({
                scope: space,
                requestId: RequestId.create(environment),
                packageId,
                ...ObjectReference.columns("version", version),
                ...(directory === null ? {} : { directory }),
                ...stack,
                ...(found === undefined ? {} : { parentId: found.id }),
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
            await stacks.installation.update({
                scope: space,
                id: installed.id,
                requestId: RequestId.create(environment),
                revision: installed.revision,
                ...ObjectReference.columns("version", version),
                directory,
                ...stack,
                ...placed,
                kind: "requested",
            });

            return { id: installed.id, name };
        }
    }
}

/** Read a file at a branch's head through a read lease, fetching that commit alone. */
async function readAtBranch(
    lease: Lease,
    branch: string,
    path: string,
    environment: CommandEnvironment,
): Promise<string> {
    // check the branch's head out into a new directory, removing it after
    const directory = await environment.files.makeTemporaryDirectory("destack-branch-");
    try {
        await GitLease.branch(lease, branch, directory, environment, new AbortController().signal);

        return await environment.files.readTextFile(join(directory, path));
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
