import type { Environment, FileSystem } from "@destack/environment";
import type { MethodOutput, Package } from "@destack/object";
import { PackageName, type PackageVersion } from "@destack/package";
import { preview, type Preview } from "../object/index.ts";
import type { CommandHost } from "../../process/command.ts";
import { join } from "node:path";
import * as check from "@destack/check/workspace";
import { and, eq } from "@destack/db";
import { nth, schema } from "@destack/schema";
import type { Select } from "@destack/db";
import { type Digest, Identifier } from "@destack/schema";
import {
    checkout,
    type CheckoutStack,
    WorkingTree,
    type WorkingTreeSource,
} from "../object/index.ts";
import { Git } from "./git.ts";
import { type CompilerPool } from "@destack/build";
import { Template } from "@destack/build/template";
import { ObjectReference, RequestId } from "@destack/sync";
import { build, Build } from "@destack/stack/object";
import type { CheckoutSource } from "../../../installation/build.ts";
import type { StackClient } from "@destack/stack/client";
import stackPackage from "@destack/stack/package";
import type { ClientOf } from "@destack/object/client";
import type { Machine } from "../../../machine/index.ts";
import { BuildImage, PackageManifest, ViewDescription } from "@destack/package/manifest";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** The file Bun reads a package's registries from. */
const BUNFIG_FILE = "bunfig.toml";

/** The bunfig table giving each scope its registry. */
const SCOPES_TABLE = "[install.scopes]";

/** How long a checkout's build caller waits. */
const BUILD_WAIT_MILLISECONDS = 5 * 60_000;

/** The machine's development workflow: its checkouts' working trees, builds and previews. */
export class CheckoutServer {
    /** The machine registering the checkouts, building them and storing their builds. */
    readonly machine: Machine;
    /** Connect to a package of the machine serving a space as a person signed in on the machine. */
    readonly connect: <Handle extends Package>(
        handle: Handle,
        login: Identifier<"login">,
        spaceId: Identifier<"space">,
    ) => Promise<ClientOf<Handle>>;
    /** The machine's warm compilers, building into its local store of every build. */
    readonly compilers: CompilerPool;
    /** The host the working trees live on: its files, Git and platform. */
    readonly host: CommandHost;

    /** Serve the development workflow of a machine. */
    constructor(fields: Pick<CheckoutServer, "machine" | "connect" | "compilers" | "host">) {
        // set the machine, its stack connection, its compilers and its host
        this.machine = fields.machine;
        this.connect = fields.connect;
        this.compilers = fields.compilers;
        this.host = fields.host;
    }

    /** Resolve a package directory in a checkout a machine registers. */
    static async locate(
        machine: Pick<Machine, "database" | "id">,
        source: { readonly checkout: string; readonly directory: string },
    ): Promise<{ readonly root: string; readonly path: string; readonly directory: string }> {
        const [registered] = await machine.database
            .select({ root: checkout.table.root })
            .from(checkout.table)
            .where(
                and(
                    eq(checkout.table.scope, machine.id),
                    eq(checkout.table.id, schema.identifier("checkout").parse(source.checkout)),
                ),
            );
        if (registered === undefined) {
            throw MachineError.of("CHECKOUT_NOT_FOUND", `checkout '${source.checkout}' not found`);
        }

        return { root: registered.root, ...WorkingTree.package(registered.root, source.directory) };
    }

    /** Implement checkouts: their Git working tree registration and the stacks they submit. */
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
                        }

                        return tree;
                    },
                    handle: (context, next) =>
                        next(
                            context.with({
                                input: {
                                    ...context.input,
                                    directory: context.prepared.directory,
                                    root: context.prepared.root,
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
                    handle: (context) => context.update({ ...context.prepared }),
                },
                delete: {
                    prepare: async (context) => context.target.root,
                    // close the warm compilers of the unregistered working tree
                    confirm: async (_call, prepared) => {
                        await compilers.release(prepared);
                    },
                },
                members: {
                    prepare: (context) =>
                        CheckoutServer.#members(
                            context.target.root,
                            context.target.directory,
                            host.files,
                        ),
                    handle: (context) => Promise.resolve(context.prepared),
                },
                submit: {
                    prepare: (context) => this.#submission(context.target, context.input),
                    handle: (context) => Promise.resolve(context.prepared),
                },
            },
        });
    }

    /** Implement previews: build each checkout package and submit the build to its installation. */
    preview() {
        return preview.implement({
            controllers: [
                {
                    name: "build",
                    // run each preview at a generation not synced yet, and each deleted one
                    when: {
                        OR: [
                            { generation: { gt: { field: "syncedGeneration" } } },
                            { deletedAt: { isNotNull: true } },
                        ],
                    },
                    reconcile: async (reconciliation) => {
                        // finalize a deleted preview
                        const row = nth(reconciliation.rows, 0);
                        if (row.deletedAt !== null) {
                            await reconciliation.mutate([
                                preview.call("finalize", preview.target(row)),
                            ]);

                            return;
                        }

                        // run the generation, recording it ready with its views, else its failure
                        try {
                            const views = await this.#runPreview(row);
                            await reconciliation.observe(preview, row, {
                                conditions: {
                                    Ready: { status: "true", reason: "Running", message: "" },
                                },
                                fields: { views },
                            });
                        } catch (error) {
                            const message = error instanceof Error ? error.message : String(error);
                            await reconciliation.observe(preview, row, {
                                conditions: {
                                    Ready: { status: "false", reason: "Failed", message },
                                },
                                fields: {},
                            });
                        }
                    },
                },
            ],
        });
    }

    /** Build a checkout's package directory in a space as a person, and wait until it succeeds. */
    async build(
        client: StackClient,
        spaceId: Identifier<"space">,
        source: CheckoutSource,
    ): Promise<{ readonly id: Identifier<"build">; readonly image: Digest }> {
        // note the machine's log position before the build, whose replica it follows as its runner
        const { database } = this.machine;
        const { sequence } = await database.log.position();

        // create the build for the machine to run
        const created = await client.build.create({
            spaceId,
            requestId: RequestId.create(this.machine.environment),
            source,
        });

        // read the build again at each of its changes until it finished
        let current = created;
        const changes = database.log.follow(
            { tables: [build.table], after: sequence },
            this.machine.clock.timeout(BUILD_WAIT_MILLISECONDS),
        );
        for await (const page of changes) {
            if (page.changes.some((change) => change.key.id === created.id)) {
                current = await client.build.get({ spaceId, id: created.id });
            }
            if (Build.isFinished(current)) {
                break;
            }
        }

        // refuse a build still running after the wait or one that did not succeed
        if (!Build.isFinished(current)) {
            throw MachineError.of(
                "BUILD_UNREACHABLE",
                `could not finish build '${created.id}' within ${BUILD_WAIT_MILLISECONDS} ms`,
            );
        }

        return { id: created.id, image: Build.image(current) };
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
        const registries = withScopes(existing, scopes.toSorted(), `${universe.origin}/npm/`);
        await host.files.writeFile(path, new TextEncoder().encode(registries));
    }

    /** Build a checkout's stack package in its space. */
    async #submission(
        registered: Select<typeof checkout.table>,
        source: CheckoutStack,
    ): Promise<MethodOutput<typeof checkout, "submit">> {
        // build every output the package implies in the space as the person
        const { selection, built, reader } = await this.#open(
            { checkout: registered.id, root: registered.root, directory: source.directory },
            source.login,
            source.space,
        );
        const packageId = PackageManifest.package(reader.manifest).id;

        return {
            packageId,
            submission: {
                version: checkout.reference(selection.machine, selection.checkout),
                directory: selection.directory,
                build: built.id,
                stack: { export: source.export, parameters: { ...source.parameters } },
            },
        };
    }

    /** Build a preview's package as its person, submit the build, and list its views. */
    async #runPreview(row: Preview): Promise<Record<string, ViewDescription>> {
        // build every output the package implies in the space as the preview's person
        const located = await CheckoutServer.locate(this.machine, {
            checkout: row.checkoutId,
            directory: row.package,
        });
        const spaceId = schema.identifier("space").parse(row.space);
        const { selection, built, client, reader } = await this.#open(
            { checkout: row.checkoutId, root: located.root, directory: row.package },
            row.login,
            spaceId,
        );
        const objects = await this.connect(stackPackage, row.login, spaceId);
        await CheckoutServer.#submitPreview(
            client,
            objects,
            spaceId,
            selection,
            PackageManifest.package(reader.manifest),
            {
                build: built.id,
                environment: this.machine.environment,
            },
        );

        return ViewDescription.list(reader.manifest);
    }

    /** Build a package directory of a checkout in a space as a person, and read the build. */
    async #open(
        source: {
            readonly checkout: Identifier<"checkout">;
            readonly root: string;
            readonly directory: string;
        },
        login: Identifier<"login">,
        spaceId: Identifier<"space">,
    ) {
        // require a package directory inside the working tree, followed from this machine
        const located = WorkingTree.package(source.root, source.directory);
        const selection: CheckoutSource = {
            kind: "checkout",
            machine: this.machine.id,
            checkout: source.checkout,
            directory: located.directory,
        };

        // build every output the package implies, then read the build
        const client = await this.connect(stackPackage, login, spaceId);
        const built = await this.build(client, spaceId, selection);
        const { builds } = this.machine;
        const reader = await builds.root(built.image);

        return { selection, built, client, builds, reader };
    }

    /** List the Destack packages of a directory's workspace at or below the directory. */
    static async #members(root: string, directory: string, files: FileSystem): Promise<string[]> {
        // read the enclosing workspace
        const workspace = await check.Workspace.read(directory, files);
        if (workspace === undefined) {
            return [];
        }

        // take the members at or below the directory that declare a Destack package
        const members = await Promise.all(
            workspace.members.map(async (member) => {
                // take a member below the working tree's root and the directory with a destack.json
                const path = join(workspace.root, member.directory);
                const below = WorkingTree.below(root, path);
                const isMember =
                    below !== undefined &&
                    WorkingTree.below(directory, path) !== undefined &&
                    (await files.exists(join(path, "destack.json")));

                return isMember ? [below] : [];
            }),
        );

        return members.flat().toSorted();
    }

    /** Submit a preview's build to its installation as the preview's person, installing it once. */
    static async #submitPreview(
        spaces: StackClient,
        objects: ClientOf<typeof stackPackage>,
        spaceId: Identifier<"space">,
        selection: CheckoutSource,
        built: PackageVersion,
        submitted: { readonly build: Identifier<"build">; readonly environment: Environment },
    ): Promise<void> {
        // read the package's installation, of which the space has one
        const { items } = await objects.installation.list({
            scope: spaceId,
            where: { packageId: built.id },
            limit: 1,
        });
        const [existing] = items;
        const version = {
            ...ObjectReference.columns(
                "follows",
                checkout.reference(selection.machine, selection.checkout),
            ),
            directory: selection.directory,
        };

        // take a request for the write
        const requestId = RequestId.create(submitted.environment);
        let installed: typeof existing;

        // install the package at the checkout's package directory
        if (existing === undefined) {
            installed = await objects.installation.create({
                scope: spaceId,
                requestId,
                packageId: built.id,
                ...version,
                name: PackageName.split(built.name).local,
            });
        }
        // keep an installation following it already
        else if (CheckoutServer.#isFollowing(existing, selection)) {
            installed = existing;
        }
        // have the package's installation follow it
        else {
            installed = await objects.installation.update({
                scope: spaceId,
                id: existing.id,
                requestId,
                revision: existing.revision,
                ...version,
            });
        }

        // apply the installation at the build
        await spaces.installation.reconcile({
            scope: spaceId,
            id: installed.id,
            requestId: RequestId.create(submitted.environment),
            build: submitted.build,
        });
    }

    /** Report whether an installation follows the same checkout's package directory. */
    static #isFollowing(
        installed: {
            readonly versionType: string | null;
            readonly versionId: string | null;
            readonly directory: string | null;
        },
        selection: { readonly checkout: string; readonly directory: string },
    ): boolean {
        return (
            installed.versionType === checkout.name &&
            installed.versionId === selection.checkout &&
            installed.directory === selection.directory
        );
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
