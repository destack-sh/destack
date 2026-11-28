import { checkout } from "./checkout/index.ts";
import { Git } from "./checkout/server/index.ts";
import { eq } from "@destack/db";
import { type LocalGitEnvironment, GitLease } from "@destack/git/local";
import { join } from "node:path";
import { type CompilerPool, outputsOf, Workspace, type WorkspacePackage } from "@destack/build";
import type { PackageVersion } from "@destack/package";
import type { BuildReader, BuildImage } from "@destack/package/manifest";
import { Digest, type Identifier, Version } from "@destack/schema";
import stackPackage, { PUSH_PATH } from "@destack/stack/package";
import { RemoteBuildStore } from "@destack/build/store";
import { build, type CompileOutput } from "@destack/stack/object";
import { MethodContext } from "@destack/object";
import type { Select } from "@destack/db";
import type { CommitSource } from "../installation/build.ts";
import type { RunnerContext } from "./runner.ts";
import { WorkingTree } from "./checkout/index.ts";
import type { Machine } from "../machine/index.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** A build's row as the run's actor reads it. */
type BuildRow = Select<typeof build.table>;

/** A build's package directory on this machine, removed once disposed. */
type SourceDirectory = { readonly directory: string } & AsyncDisposable;

/** The machine's builder: it compiles builds placed on it and pushes each to its space. */
export class Builder {
    /** The machine whose compilers build, and whose store keeps the builds. */
    readonly #machine: Machine;
    /** The warm compilers compiling the builds into the machine's local store. */
    readonly #compilers: CompilerPool;
    /** The linked builtins' images, pulled once with all their files. */
    #linked: Promise<readonly BuildImage[]> | undefined;
    /** The host the working trees are checked out on, by git. */
    readonly #environment: LocalGitEnvironment;
    /** The end of the build last opened in each checkout's mirror, which builds one at a time. */
    readonly #mirrors = new Map<string, Promise<void>>();

    /** Build on a machine, compiling with warm compilers on a host. */
    constructor(machine: Machine, compilers: CompilerPool, environment: LocalGitEnvironment) {
        // set the machine, its compilers and the host
        this.#machine = machine;
        this.#compilers = compilers;
        this.#environment = environment;
    }

    /** Compile a build placed on this machine and push it to its space, answering its image. */
    async build(
        id: Identifier<"build">,
        context: Pick<RunnerContext, "scope" | "call" | "signal" | "directory">,
    ): Promise<CompileOutput> {
        // read the build as the run's actor
        const { scope, signal } = context;
        const read = build.call("get", build.target({ scope, id }));
        const [input] = MethodContext.results([build], [read], [await context.call(read)]);

        // open the source and its workspace, compiling only its publisher's members as a published build does
        await using opened = await this.#open(input, context.directory, signal);
        const workspacePackage = await this.#package(opened.directory);
        const implied = outputsOf(workspacePackage.root.packageJson);
        const outputs = input.outputs ?? Object.keys(implied);

        // compile the outputs with a warm compiler
        const compiled = await this.#compile(
            workspacePackage,
            opened,
            { outputs, version: input.version },
            signal,
        );
        const root = await compiled.digest();

        // bundle a release's browser output, its builtins inlined to what its entries reach
        const built = await workspacePackage.image(root);
        const isRelease = input.version !== null && Version.isRelease(input.version);
        const image = isRelease ? await workspacePackage.release(built) : built;

        // push the build and the graphs the image attaches to the space
        const endpoint = await this.#machine.options(stackPackage, input.scope);
        const pushing = new RemoteBuildStore(`${endpoint.url}${PUSH_PATH}/`, endpoint.fetch);
        await pushing.push(this.#compilers.store, [image], signal);

        return { image, outputs: [...outputs] };
    }

    /** Compile every output of a package directory as unreleased code, as an edit checks it. */
    async compile(directory: string, signal: AbortSignal): Promise<void> {
        const workspacePackage = await this.#package(directory);
        await workspacePackage.build({
            outputs: outputsOf(workspacePackage.root.packageJson),
            isUnreleased: true,
            signal,
        });
    }

    /** Open a package directory's workspace, compiling only its publisher's members. */
    async #package(directory: string): Promise<WorkspacePackage> {
        const workspace = await Workspace.open(directory, this.#compilers, {
            resolver: (dependency) => this.#release(dependency),
            selection: "account",
            host: await this.#builtins(),
        });

        return workspace.package();
    }

    /** Pull the linked builtins' images once with all their files, which a compile reads. */
    #builtins(): Promise<readonly BuildImage[]> {
        // take the images pulled or pulling already
        if (this.#linked !== undefined) {
            return this.#linked;
        }

        // pull every build each image lists with all its files, which the bundled layer lacks
        const { linked } = this.#machine;
        const pulling = Promise.all(
            [...linked.values()].map(async (release) => {
                // pull the image whole, then read it
                const root = { kind: "image", digest: release.image } as const;
                await this.#machine.pull(release.package.id, root, "all");

                return await this.#machine.builds.image(release.image);
            }),
        );
        this.#linked = pulling;
        pulling.catch(() => (this.#linked = undefined));

        return pulling;
    }

    /** Pull a required release's image from its publisher's registry, with every build it lists. */
    async #release(dependency: PackageVersion): Promise<Digest> {
        // pull the image of its version whole
        const digest = await this.#machine.resolve(dependency.id, dependency.version);
        await this.#machine.pull(dependency.id, { kind: "image", digest }, "all");

        return digest;
    }

    /** Open a build's source here: a checkout's mirror at the commit, else a fetched commit. */
    async #open(input: BuildRow, directory: string, signal: AbortSignal): Promise<SourceDirectory> {
        const { source } = input;

        // build a commit in the mirror of a checkout of this machine that has it, else fetch it on a runner
        if (source.kind === "commit") {
            const local = await this.#local(source, signal);
            if (local === undefined && !this.#machine.isRunner) {
                throw MachineError.of(
                    "BUILD_BLOCKED",
                    `cannot build commit '${source.commit}': no checkout of this machine has it, and it runs no space's work`,
                );
            }

            return local ?? (await this.#fetch(source, input.scope, directory, signal));
        }
        // refuse a release, which is born built and leases no runner
        else {
            throw MachineError.of(
                "RELEASE_INVALID",
                `build '${input.id}' is invalid: it is a release and compiles nothing`,
            );
        }
    }

    /** Move the mirror of a checkout of this machine that has a commit to it, absent for none. */
    async #local(source: CommitSource, signal: AbortSignal): Promise<SourceDirectory | undefined> {
        // find the first checkout whose repository has the commit
        const git = new Git(this.#environment);
        const checkouts = await this.#machine.database
            .select({ root: checkout.table.directory })
            .from(checkout.table)
            .where(eq(checkout.table.scope, this.#machine.id));
        let found: string | undefined;
        for (const { root } of checkouts) {
            if (found === undefined && (await git.has(root, source.commit))) {
                found = root;
            }
        }
        if (found === undefined) {
            return undefined;
        }

        // wait for the build open in the checkout's mirror, then take the mirror
        const previous = this.#mirrors.get(found) ?? Promise.resolve();
        const ended = Promise.withResolvers<void>();
        this.#mirrors.set(
            found,
            previous.then(() => ended.promise),
        );
        await previous;

        // move the mirror to the commit, installing its dependencies again when its lockfiles changed
        try {
            const { path, isLockfileChanged } = await git.mirror(found, source.commit);
            if (isLockfileChanged) {
                const registry = await this.#machine.registry();
                await Workspace.install(path, this.#environment, {
                    isFrozen: true,
                    signal,
                    registry,
                });
            }

            return {
                directory: WorkingTree.package(path, source.directory).path,
                [Symbol.asyncDispose]: async () => ended.resolve(),
            };
        } catch (error) {
            ended.resolve();
            throw error;
        }
    }

    /** Fetch a repository commit into a step's directory with its locked dependencies installed. */
    async #fetch(
        source: CommitSource,
        spaceId: Identifier<"space">,
        directory: string,
        signal: AbortSignal,
    ): Promise<SourceDirectory> {
        // lease the repository as the space and check the commit out in the step's directory
        const lease = await (await this.#machine.space(spaceId)).lease(source.repository);
        const environment = this.#environment;
        const root = join(directory, source.commit);
        await environment.files.mkdir(root, { recursive: true });
        const release = async () => this.#compilers.release(root);
        try {
            await GitLease.checkout(lease, source.commit, root, environment, signal);
            const registry = await this.#machine.registry();
            await Workspace.install(root, environment, { isFrozen: true, signal, registry });
        } catch (error) {
            await release();
            throw error;
        }

        return {
            directory: WorkingTree.package(root, source.directory).path,
            [Symbol.asyncDispose]: release,
        };
    }

    /** Compile a package directory's outputs on its workspace units into the machine's store. */
    async #compile(
        workspacePackage: WorkspacePackage,
        opened: SourceDirectory,
        built: { readonly outputs: readonly string[]; readonly version: Version | null },
        signal: AbortSignal,
    ): Promise<BuildReader> {
        // release the package at the build's version, else stamp it a development build
        const { name, packageJson } = workspacePackage.root;
        const request = {
            outputs: outputsOf(packageJson, built.outputs),
            ...(built.version === null ? {} : { versions: { [name]: built.version } }),
            isUnreleased: built.version === null,
            signal,
        };

        return workspacePackage.build(request);
    }
}
