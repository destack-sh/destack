import type { FileSystem } from "@destack/environment";
import { PackageName } from "@destack/package";
import type { CommandHost } from "../../process/command.ts";
import { join } from "node:path";
import { GitReference } from "@destack/git";
import { GitLease } from "@destack/git/local";
import * as check from "@destack/check/workspace";
import { Commit, Identifier } from "@destack/schema";
import { checkout, WorkingTree, type WorkingTreeSource } from "../object/index.ts";
import { Git } from "./git.ts";
import { type CompilerPool, Workspace } from "@destack/build";
import { Template } from "@destack/build/template";
import type { Machine } from "../../../machine/index.ts";
import { BuildImage, PackageManifest } from "@destack/package/manifest";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** How long a new package's install may take: 5 minutes, a cold cache of a few hundred packages. */
const INSTALL_MILLISECONDS = 5 * 60_000;

/** How long a push of a working tree may take: 1 minute, a 100 MB pack at 2 MB/s. */
const PUSH_MILLISECONDS = 60_000;

/** The file Bun reads a package's registries from. */
const BUNFIG_FILE = "bunfig.toml";

/** The bunfig table giving each scope its registry. */
const SCOPES_TABLE = "[install.scopes]";

/** The machine's checkouts: their working trees' registration and members. */
export class CheckoutServer {
    /** The machine registering the checkouts. */
    readonly machine: Machine;
    /** The machine's warm compilers, released with a checkout's working tree. */
    readonly compilers: CompilerPool;
    /** The host the working trees live on: its files, Git and platform. */
    readonly host: CommandHost;

    /** Serve the checkouts of a machine. */
    constructor(fields: Pick<CheckoutServer, "machine" | "compilers" | "host">) {
        // set the machine, its compilers and its host
        this.machine = fields.machine;
        this.compilers = fields.compilers;
        this.host = fields.host;
    }

    /** Implement checkouts: their Git working tree registration and their members. */
    checkout() {
        const { compilers, host } = this;

        return checkout.implement({
            methods: {
                create: {
                    prepare: async (context) => {
                        // open the working tree, then write a template's package into it
                        const { directory, source } = context.input;
                        const tree = await new Git(host).open(directory, source);
                        if (source.kind === "template") {
                            await this.#instantiate(directory, source);
                            await this.#introduce(tree.root, source);
                        }

                        return tree;
                    },
                    handle: (context, next) =>
                        next(
                            context.with({
                                input: {
                                    ...context.input,
                                    directory: context.prepared.root,
                                    remote: context.prepared.remote,
                                },
                            }),
                        ),
                },
                relocate: {
                    prepare: (context) => {
                        // require the directory the working tree moved to
                        const { directory } = context.input;
                        if (directory === undefined) {
                            throw MachineError.of(
                                "RELOCATION_TARGET_REQUIRED",
                                "relocation target is required: relocate requires the directory the working tree moved to",
                            );
                        }

                        return new Git(host).relocated(context.target.directory, directory);
                    },
                    handle: (context) =>
                        context.update({
                            directory: context.prepared.root,
                            remote: context.prepared.remote,
                        }),
                },
                delete: {
                    prepare: async (context) => context.target.directory,
                    // close the warm compilers of the unregistered working tree and of its mirror, then remove the mirror
                    confirm: async (_call, prepared) => {
                        await compilers.release(prepared);
                        const mirror = await new Git(host).removeMirror(prepared);
                        if (mirror !== undefined) {
                            await compilers.release(mirror);
                        }
                    },
                },
                push: {
                    prepare: async (context) => {
                        // commit the working tree as it is, refusing uncommitted changes when asked for HEAD
                        const { directory: root, id } = context.target;
                        const { commit, head } = await new Git(host).snapshot(
                            root,
                            `Snapshot ${id}`,
                            context.input.author,
                        );
                        if (context.input.of === "head" && commit !== head) {
                            throw MachineError.of(
                                "CHANGES_BLOCKED",
                                `cannot push the HEAD of '${root}': it has uncommitted changes, so commit them first`,
                            );
                        }
                        const reference = GitReference.checkout(this.machine.id, id);

                        // push the commit with the person's lease
                        const { lease } = context.input;
                        const signal = host.clock.timeout(PUSH_MILLISECONDS);
                        await GitLease.set(lease, root, reference, commit, host, signal);

                        return { commit: Commit.parse(commit), reference };
                    },
                    handle: (context) => Promise.resolve(context.prepared),
                },
                members: {
                    prepare: (context) =>
                        CheckoutServer.#members(context.target.directory, host.files),
                    handle: (context) => Promise.resolve(context.prepared),
                },
            },
        });
    }

    /** Install a new package's dependencies, then commit and push it with the person's lease. */
    async #introduce(
        root: string,
        source: Extract<WorkingTreeSource, { readonly kind: "template" }>,
    ): Promise<void> {
        // install the dependencies, which writes the lockfile a frozen install reads
        const { host } = this;
        await Workspace.install(root, host, {
            isFrozen: false,
            signal: host.clock.timeout(INSTALL_MILLISECONDS),
        });

        // commit everything as the person, and push the commit to its branch
        const { commit, branch } = await new Git(host).introduce(
            root,
            `Introduce ${source.name}`,
            source.author,
        );
        const signal = host.clock.timeout(PUSH_MILLISECONDS);
        await GitLease.set(source.lease, root, GitReference.branch(branch), commit, host, signal);
    }

    /** Write a package made from a template's published release into a new directory. */
    async #instantiate(
        directory: string,
        source: Extract<WorkingTreeSource, { readonly kind: "template" }>,
    ): Promise<void> {
        // pull the template's release whole, its source files among them
        const { machine, compilers, host } = this;
        const image = await machine.resolve(source.template, source.release);
        await machine.pull(source.template, { kind: "image", digest: image }, "all");
        const opened = await machine.builds.image(image);

        // read the packages the release links, each at the version it released with
        const links = Object.entries(BuildImage.links(opened, opened.root));
        const linked = await Promise.all(
            links.map(async ([name, digest]) => {
                const manifest = await machine.builds.get(digest);

                return [name, PackageManifest.package(manifest)] as const;
            }),
        );

        // read the template's files
        const reader = await machine.builds.contents(opened.root);
        const files = await Promise.all(
            (await reader.files()).map(
                async (file) => [file.path, await reader.load(file.path)] as const,
            ),
        );
        const template = new Template(new Map(files));

        // make the package, selecting the template's own dependencies at their released versions
        const selected = linked.filter(([name]) => template.dependencies.includes(name));
        const parameters = {
            id: Identifier.create("package", machine.environment),
            name: source.name,
            dependencies: Object.fromEntries(selected),
            versions: Object.fromEntries(linked.map(([name, each]) => [name, each.version])),
        };
        await template.write(directory, parameters, await compilers.toolchain(), host.files);

        // add the universe's npm address for the scopes of the package and its Destack dependencies
        const { universe } = await machine.enrolled("making a package");
        const names = [source.name, ...template.workspace, ...linked.map(([name]) => name)];
        const scopes = [...new Set(names.map((name) => `@${PackageName.split(name).handle}`))];
        const path = join(directory, BUNFIG_FILE);
        const existing = (await host.files.exists(path)) ? await host.files.readTextFile(path) : "";
        const registries = withScopes(existing, scopes.toSorted(), `${universe.origin}/.npm/`);
        await host.files.writeFile(path, new TextEncoder().encode(registries));
    }

    /** List the Destack packages of a working tree's workspace, by path below its root. */
    static async #members(root: string, files: FileSystem): Promise<string[]> {
        // read the enclosing workspace
        const workspace = await check.Workspace.read(root, files);
        if (workspace === undefined) {
            return [];
        }

        // take the members below the working tree's root that declare a Destack package
        const members = await Promise.all(
            workspace.members.map(async (member) => {
                // take a member with a destack.json, by its path below the root
                const path = join(workspace.root, member.directory);
                const below = WorkingTree.below(root, path);
                const isMember =
                    below !== undefined && (await files.exists(join(path, "destack.json")));

                return isMember ? [below] : [];
            }),
        );

        return members.flat().toSorted();
    }
}

/** Write scopes' registry into a bunfig's text, keeping the scopes it sets and its other tables. */
function withScopes(bunfig: string, scopes: readonly string[], url: string): string {
    // leave out the scopes the text sets already
    const lines = bunfig.split("\n");
    const added = scopes
        .filter((scope) => !lines.some((line) => line.trimStart().startsWith(`"${scope}"`)))
        .map((scope) => `"${scope}" = { url = "${url}" }`);

    // add them below the text's scopes table, or in a table of their own at its end
    const header = lines.findIndex((line) => line.trim() === SCOPES_TABLE);
    if (header === -1) {
        return [...lines, SCOPES_TABLE, ...added, ""].join("\n");
    }

    return lines.toSpliced(header + 1, 0, ...added).join("\n");
}
