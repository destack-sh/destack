import type { Environment, FileSystem } from "@destack/environment";
import { ListPage, type MethodOutput, Observation, type PackageHandle } from "@destack/object";
import type { Package } from "@destack/package";
import { preview, type Preview } from "../object/index.ts";
import type { CommandHost } from "../../process/command.ts";
import { join } from "node:path";
import * as check from "@destack/check/workspace";
import { and, eq } from "@destack/db";
import { nth, present, schema } from "@destack/schema";
import type { Select } from "@destack/db";
import type { Digest, Identifier } from "@destack/schema";
import { checkout, type CheckoutStack, WorkingTree } from "../object/index.ts";
import { Git } from "./git.ts";
import { type CompilerPool, outputsOf, Workspace } from "@destack/build";
import { PackageInspection } from "@destack/package/code";
import { RequestId } from "@destack/sync";
import { build, Build } from "@destack/stack/object";
import type { CheckoutSource } from "@destack/stack/server";
import type { StackClient } from "@destack/stack/client";
import stackPackage from "@destack/stack/package";
import objectPackage from "@destack/object/package";
import type { ClientOf } from "@destack/object/client";
import type { Machine } from "../../../machine/index.ts";
import { PackageManifest, ViewDescription } from "@destack/package/manifest";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** How long a checkout's build caller waits. */
const BUILD_WAIT_MILLISECONDS = 5 * 60_000;

/** The machine's development workflow: its checkouts' working trees, builds and previews. */
export class CheckoutServer {
    /** The machine registering the checkouts, building them and storing their builds. */
    readonly machine: Machine;
    /** Connect to a package of the machine serving a space as a person signed in on the machine. */
    readonly connect: <Handle extends PackageHandle>(
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
            throw MachineError.of(
                "CHECKOUT_NOT_FOUND",
                `this machine has no registered checkout ${source.checkout}`,
            );
        }

        return { root: registered.root, ...WorkingTree.package(registered.root, source.directory) };
    }

    /** Implement checkouts: their Git working tree registration and the stacks they submit. */
    checkout() {
        const { compilers, host } = this;

        return checkout.implement({
            methods: {
                create: {
                    prepare: (context) =>
                        new Git(host).open(context.input.directory, context.input.source),
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
                                "relocate requires the directory the working tree moved to",
                            );
                        }

                        return new Git(host).relocated(context.target.directory, directory);
                    },
                    handle: (context) => context.update({ ...context.prepared }),
                },
                delete: {
                    prepare: async (context) => context.target.root,
                    settle: async (_call, settled) => {
                        // close the warm compilers of the unregistered working tree
                        if (settled.status === "committed") {
                            await compilers.release(settled.prepared);
                        }
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
                inspect: {
                    prepare: (context) => this.#inspect(context.target, context.input),
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
                    pending: {},
                    reconcile: async (reconciliation) => {
                        // finalize a deleted preview
                        const row = nth(reconciliation.rows, 0);
                        if (row.deletedAt !== null) {
                            await reconciliation.mutate([
                                preview.call("finalize", preview.target(row)),
                            ]);

                            return;
                        }
                        // leave one observed at its generation
                        else if (Observation.isCurrent(row, "Ready")) {
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
    ): Promise<{ readonly id: Identifier<"build">; readonly manifest: Digest }> {
        // require the space here, noting its log position before the build
        const { spaces } = this.machine;
        if (!(await spaces.list()).includes(spaceId)) {
            throw MachineError.of(
                "CHECKOUT_SPACE_ELSEWHERE",
                `${spaceId} is not served on this machine, which has the working tree`,
            );
        }
        const { database } = await this.machine.space(spaceId);
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
                "BUILD_TIMED_OUT",
                `build ${created.id} did not finish in ${BUILD_WAIT_MILLISECONDS} ms`,
            );
        }

        return { id: created.id, manifest: Build.manifest(current) };
    }

    /** Inspect a checkout's package directory for one output, on its dependencies' builds. */
    async #inspect(
        registered: Select<typeof checkout.table>,
        input: { readonly directory: string; readonly output: string },
    ): Promise<PackageInspection> {
        // read the output's runtime, refusing an output the package does not build
        const located = WorkingTree.package(registered.root, input.directory);
        const workspace = await Workspace.open(located.path, this.compilers);
        const [selected] = Object.values(outputsOf(workspace.root.packageJson, [input.output]));
        const { runtime } = present(selected, `the output ${input.output}`);

        // inspect with the package's warm compiler on its workspace units' stored builds
        const inspection = await workspace.inspect({ runtime });

        return PackageInspection.parse(inspection);
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
                selection,
                build: { kind: "build", build: built.id },
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
        const objects = await this.connect(objectPackage, row.login, spaceId);
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
        const { builds } = await this.machine.space(spaceId);
        const reader = await builds.contents(built.manifest);

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
        objects: ClientOf<typeof objectPackage>,
        spaceId: Identifier<"space">,
        selection: CheckoutSource,
        built: Package,
        submitted: { readonly build: Identifier<"build">; readonly environment: Environment },
    ): Promise<void> {
        // install the package following the checkout once, its checkout directory naming the package
        const items = await Array.fromAsync(
            ListPage.all(objects.installation.list, { scope: spaceId }),
        );
        const installed =
            items.find((item) => CheckoutServer.#isFollowing(item.selection, selection)) ??
            (await objects.installation.create({
                scope: spaceId,
                requestId: RequestId.create(submitted.environment),
                packageId: built.id,
                selection,
                name: built.name.slice(built.name.lastIndexOf("/") + 1),
            }));

        // submit the build for the installation to follow
        await spaces.rollout.submit({
            spaceId,
            id: installed.id,
            requestId: RequestId.create(submitted.environment),
            selection,
            build: { kind: "build", build: submitted.build },
        });
    }

    /** Report whether an installation follows the same checkout's package directory. */
    static #isFollowing(
        followed: {
            readonly kind: string;
            readonly checkout?: string;
            readonly directory?: string;
        },
        selection: { readonly checkout: string; readonly directory: string },
    ): boolean {
        return (
            followed.kind === "checkout" &&
            followed.checkout === selection.checkout &&
            followed.directory === selection.directory
        );
    }
}
