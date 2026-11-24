import { type LocalGitEnvironment, GitLease } from "@destack/git/local";
import { join } from "node:path";
import { type CompilerPool, outputsOf, Workspace, type WorkspacePackage } from "@destack/build";
import type { PackageVersion } from "@destack/package";
import type { BuildReader, BuildImage } from "@destack/package/manifest";
import { Commit, Digest, type Identifier, schema, Uuid, type Version } from "@destack/schema";
import stackPackage, { PUSH_PATH } from "@destack/stack/package";
import { RemoteBuildStore } from "@destack/build/store";
import { build, type CheckoutSource, type CompileOutput } from "@destack/stack/object";
import { MethodContext } from "@destack/object";
import { ObjectReference } from "@destack/sync";
import type { Select } from "@destack/db";
import type { CommitSource } from "../installation/build.ts";
import type { RunnerContext } from "./runner.ts";
import { principal } from "@destack/access";
import { WorkingTree } from "./checkout/index.ts";
import { CheckoutServer, Git } from "./checkout/server/index.ts";
import type { Machine } from "../machine/index.ts";
import type { Device } from "./device/device.ts";
import type { NativeClient } from "@destack/account/native";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** A build's row as the run's actor reads it. */
type BuildRow = Select<typeof build.table>;

/** A build's package directory on this machine with the commit its files match. */
type SourceDirectory = {
    readonly directory: string;
    readonly commit?: Commit;
} & AsyncDisposable;

/** The machine's builder: it compiles builds placed on it and pushes each to its space. */
export class Builder {
    /** The machine whose compilers build, and whose store keeps the builds. */
    readonly #machine: Machine;
    /** The computer's device, whose signed-in people alone build its checkouts. */
    readonly #device: Pick<Device<NativeClient>, "isSignedIn">;
    /** The warm compilers compiling the builds into the machine's local store. */
    readonly #compilers: CompilerPool;
    /** The linked builtins' images, pulled once with all their files. */
    #linked: Promise<readonly BuildImage[]> | undefined;
    /** The directory of the working trees of the commits being built. */
    readonly #commits: string;
    /** The host the working trees are checked out on, by git. */
    readonly #environment: LocalGitEnvironment;

    /** Build on a machine and its device's people, compiling with warm compilers on a host. */
    constructor(
        machine: Machine,
        device: Pick<Device<NativeClient>, "isSignedIn">,
        compilers: CompilerPool,
        commits: string,
        environment: LocalGitEnvironment,
    ) {
        // set the machine, its device, its compilers, the commits' directory and the host
        this.#machine = machine;
        this.#device = device;
        this.#compilers = compilers;
        this.#commits = commits;
        this.#environment = environment;
    }

    /** Compile a build placed on this machine and push it to its space, answering its image. */
    async build(
        id: Identifier<"build">,
        context: Pick<RunnerContext, "scope" | "call" | "signal">,
    ): Promise<CompileOutput> {
        // read the build as the run's actor
        const { scope, signal } = context;
        const read = build.call("get", build.target({ scope, id }));
        const [input] = MethodContext.results([build], [read], [await context.call(read)]);

        // open the source and its workspace, compiling only its publisher's members as a published build does
        await using opened = await this.#open(input, signal);
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

        // push the build and the graphs the image attaches to the space
        const endpoint = await this.#machine.options(stackPackage, input.scope);
        const pushing = new RemoteBuildStore(`${endpoint.url}${PUSH_PATH}/`, endpoint.fetch);
        const image = await workspacePackage.image(root);
        await pushing.push(this.#compilers.store, [image], signal);

        return {
            image,
            outputs: [...outputs],
            ...(opened.commit === undefined ? {} : { commit: opened.commit }),
        };
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

        // pull every build each image lists with all its files, which the shipped layer lacks
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

    /** Open a build's source here: a checkout for its creator alone, else a fetched commit. */
    async #open(input: BuildRow, signal: AbortSignal): Promise<SourceDirectory> {
        const { source } = input;

        // read a checkout in place, for this computer's own person alone
        if (source.kind === "checkout") {
            await this.#requireSignedIn(input);

            return this.#checkout(source);
        }
        // fetch a commit from its repository
        else if (source.kind === "commit") {
            return this.#fetch(source, input.scope, signal);
        }
        // refuse a release, which is born built and leases no runner
        else {
            throw MachineError.of(
                "RELEASE_INVALID",
                `build '${input.id}' is invalid: it is a release and compiles nothing`,
            );
        }
    }

    /** Refuse a build of a checkout whose creator is not signed in on this computer. */
    async #requireSignedIn(input: BuildRow): Promise<void> {
        // take a build its creator made while signed in on this computer
        const creator = ObjectReference.at("createdBy", input);
        const isSignedIn =
            principal.user.is(creator) &&
            (await this.#device.isSignedIn(schema.identifier("user").parse(creator.id)));
        if (!isSignedIn) {
            throw MachineError.of(
                "BUILD_DENIED",
                `build '${input.id}' of a checkout is forbidden: its creator is not signed in on this computer`,
            );
        }
    }

    /** Resolve a checkout's package directory to its path and clean commit. */
    async #checkout(source: CheckoutSource): Promise<SourceDirectory> {
        const located = await CheckoutServer.locate(this.#machine, source);
        const commit = await new Git(this.#environment).commit(located.root);

        return {
            directory: located.path,
            ...(commit === undefined ? {} : { commit: Commit.parse(commit) }),
            [Symbol.asyncDispose]: async () => {},
        };
    }

    /** Fetch a repository commit into a new working tree with its locked dependencies installed. */
    async #fetch(
        source: CommitSource,
        spaceId: Identifier<"space">,
        signal: AbortSignal,
    ): Promise<SourceDirectory> {
        // lease the repository as the space and check the commit out into a new working tree
        const lease = await (await this.#machine.space(spaceId)).lease(source.repository);
        const environment = this.#environment;
        const root = join(this.#commits, `${source.commit}-${Uuid.v4(environment.random)}`);
        await environment.files.mkdir(root, { recursive: true });
        const remove = async () => {
            await this.#compilers.release(root);
            await environment.files.remove(root, { recursive: true, force: true });
        };
        try {
            await GitLease.checkout(lease, source.commit, root, environment, signal);
            await Workspace.install(root, environment, { isFrozen: true, signal });
        } catch (error) {
            await remove();
            throw error;
        }

        return {
            directory: WorkingTree.package(root, source.directory).path,
            commit: source.commit,
            [Symbol.asyncDispose]: remove,
        };
    }

    /** Compile a package directory's outputs on its workspace units into the machine's store. */
    async #compile(
        workspacePackage: WorkspacePackage,
        opened: SourceDirectory,
        built: { readonly outputs: readonly string[]; readonly version: Version | null },
        signal: AbortSignal,
    ): Promise<BuildReader> {
        // release the package at the build's version, marking uncommitted source a dev build of it
        const { name, packageJson } = workspacePackage.root;
        const request = {
            outputs: outputsOf(packageJson, built.outputs),
            ...(built.version === null ? {} : { versions: { [name]: built.version } }),
            isUnreleased: opened.commit === undefined,
            signal,
        };

        return workspacePackage.build(request);
    }
}
