import type { LocalGitEnvironment } from "@destack/git/local";
import { join } from "node:path";
import type { ClientOf } from "@destack/object/client";
import { type CompilerPool, outputsOf, Workspace } from "@destack/build";
import type { PackageVersion } from "@destack/package";
import type { BuildReader, Lock } from "@destack/package/manifest";
import { type Controller, eq, type Select, Reconciliation } from "@destack/db";
import { GitLease } from "@destack/git/local";
import { Digest, type Identifier, present, schema, Uuid } from "@destack/schema";
import { PRIMARY_RUNTIMES } from "@destack/package/runtime";
import { CallError } from "@destack/object";
import { RequestId } from "@destack/sync";
import stackPackage, { PUSH_PATH } from "@destack/stack/package";
import { BuildObject, PackageClient } from "@destack/build/store";
import { ACTIVE_BUILD_STATES, build } from "@destack/stack/object";
import type { CheckoutSource, CommitSource } from "@destack/stack/server";
import { Failure } from "@destack/error";
import { WorkingTree } from "./checkout/index.ts";
import { CheckoutServer, Git } from "./checkout/server/index.ts";
import type { Machine } from "../machine/index.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** The builds this machine runs at once, two beside a laptop's other work. */
const RUN_CONCURRENCY = 2;

/** A build's package directory on this machine with the commit its files match. */
type SourceDirectory = {
    readonly directory: string;
    readonly commit?: string;
} & AsyncDisposable;

/** Compile the builds assigned to this machine and push each to its space. */
export class BuildRunner {
    /** The machine whose replicas list the builds, and whose compilers build them. */
    readonly #machine: Machine;
    /** The warm compilers compiling the builds into the machine's local store. */
    readonly #compilers: CompilerPool;
    /** The directory of the working trees of the commits being built. */
    readonly #commits: string;
    /** The host the working trees are checked out on, by git. */
    readonly #environment: LocalGitEnvironment;

    /** Run the builds a machine replicates on a host, compiling them with warm compilers. */
    constructor(
        machine: Machine,
        compilers: CompilerPool,
        commits: string,
        environment: LocalGitEnvironment,
    ) {
        // set the machine, its compilers, the commits' directory and the host
        this.#machine = machine;
        this.#compilers = compilers;
        this.#commits = commits;
        this.#environment = environment;
    }

    /** Build the controller running the unfinished replicated builds, two at once. */
    controller(): Controller<typeof build.table> {
        return {
            name: "build",
            table: build.table,
            pending: { state: { in: [...ACTIVE_BUILD_STATES] } },
            concurrency: RUN_CONCURRENCY,
            reconcile: (rows, reconciliation) => this.reconcile(rows, reconciliation),
        };
    }

    /** Run an unfinished build once, aborting once its replica shows it ended elsewhere. */
    async reconcile(
        rows: readonly Select<typeof build.table>[],
        reconciliation: Reconciliation,
    ): Promise<void> {
        // leave a finished build
        const [row] = rows;
        const replicated = row === undefined ? undefined : await this.#replicated(row.id);
        if (
            replicated === undefined ||
            !ACTIVE_BUILD_STATES.some((state) => state === replicated.state)
        ) {
            return;
        }
        const spaceId = schema.identifier("space").parse(replicated.scope);
        const space = await this.#machine.connect(stackPackage, spaceId);
        const target = { spaceId, id: replicated.id };

        // start a pending build at its space
        if (replicated.state === "pending") {
            await space.build.start({
                ...target,
                requestId: RequestId.create(this.#machine.environment),
            });
        }

        // run it until its replica shows it ended elsewhere
        await Reconciliation.runUntil(
            reconciliation,
            async () => {
                const current = await this.#replicated(replicated.id);

                return current === undefined || current.state === "running"
                    ? undefined
                    : MachineError.of(
                          "BUILD_BLOCKED",
                          `cannot end build '${replicated.id}': it already ended`,
                      );
            },
            async (signal) => {
                try {
                    await this.#run(space, target, replicated, signal);
                } catch (error) {
                    // leave a build its space refuses for now to the loop's retry
                    if (CallError.of(error)?.status === 503) {
                        throw error;
                    }
                    // finish a build still running here with its failure
                    else if (!signal.aborted) {
                        await space.build.finish({
                            ...target,
                            requestId: RequestId.create(this.#machine.environment),
                            end: { kind: "failed", failure: Failure.of(error) },
                        });
                    }
                }
            },
        );
    }

    /** Open a build's source, compile it, push it to its space and finish it with its manifest. */
    async #run(
        space: ClientOf<typeof stackPackage>,
        target: { readonly spaceId: Identifier<"space">; readonly id: Identifier<"build"> },
        replicated: Select<typeof build.table>,
        signal: AbortSignal,
    ): Promise<void> {
        // open the source and its workspace, compiling only its publisher's members as a published build does
        await using opened = await this.#open(target.spaceId, replicated, signal);
        const workspace = await Workspace.open(opened.directory, this.#compilers, {
            resolver: (dependency) => this.#release(dependency),
            selection: "account",
            host: await this.#builtins(),
        });
        const implied = outputsOf(workspace.root.packageJson);
        const outputs = replicated.outputs ?? Object.keys(implied);
        await space.build.progress({
            ...target,
            requestId: RequestId.create(this.#machine.environment),
            progress: {
                phase: "building",
                outputs: [...outputs],
                ...(opened.commit === undefined ? {} : { commit: opened.commit }),
            },
        });

        // compile the outputs with a warm compiler, then index the code for its primary runtime
        const compiled = await this.#compile(workspace, opened, outputs, signal);
        const runtime = present(
            PRIMARY_RUNTIMES.find((each) =>
                Object.values(implied).some((output) => output.runtime === each),
            ),
            `a runtime ${workspace.root.name} ships`,
        );
        const code = new TextEncoder().encode(JSON.stringify(await workspace.index({ runtime })));
        const indexed = {
            digest: await Digest.of(code, this.#environment.crypto),
            size: code.byteLength,
        };

        // report storing, then push the build and its code index to the space's builds
        await space.build.progress({
            ...target,
            requestId: RequestId.create(this.#machine.environment),
            progress: { phase: "storing" },
        });
        const endpoint = await this.#machine.options(stackPackage, target.spaceId);
        const pushing = new PackageClient(
            `${endpoint.url}${PUSH_PATH}/${target.spaceId}/${target.id}/`,
            endpoint.fetch,
        );
        const digest = await workspace.lock(await compiled.digest());
        await pushing.push(this.#compilers.store, [digest], signal);
        await pushing.write(BuildObject.files([{ digest: indexed.digest, bytes: code }]), signal);
        await space.build.finish({
            ...target,
            requestId: RequestId.create(this.#machine.environment),
            end: {
                kind: "stored",
                lock: digest,
                code: { digest: indexed.digest, size: indexed.size },
            },
        });
    }

    /** Read the locks of the builtins the machine links. */
    async #builtins(): Promise<readonly Lock[]> {
        const { cache, releases } = this.#machine;

        return [...(await cache.linkedLocks(releases)).values()];
    }

    /** Pull a required release's lock from its publisher's forge, with every build it lists. */
    async #release(dependency: PackageVersion): Promise<Digest> {
        // pull the lock of its version whole
        const { cache, releases } = this.#machine;
        const digest = await releases.find(dependency.id, dependency.version);
        await releases.pull(cache.store, dependency.id, digest, "all");

        return digest;
    }

    /** Open a build's source on this machine. */
    async #open(
        spaceId: Identifier<"space">,
        replicated: Select<typeof build.table>,
        signal: AbortSignal,
    ): Promise<SourceDirectory> {
        const { source } = replicated;

        // read a checkout in place
        if (source.kind === "checkout") {
            return this.#checkout(source);
        }
        // fetch a commit from its repository
        else if (source.kind === "commit") {
            return this.#fetch(source, spaceId, signal);
        }
        // refuse a release, which is born built and leases no runner
        else {
            throw MachineError.of(
                "RELEASE_INVALID",
                `build '${replicated.id}' is invalid: it is a release and compiles nothing`,
            );
        }
    }

    /** Resolve a checkout's package directory to its path and clean commit. */
    async #checkout(source: CheckoutSource): Promise<SourceDirectory> {
        const located = await CheckoutServer.locate(this.#machine, source);
        const commit = await new Git(this.#environment).commit(located.root);

        return {
            directory: located.path,
            ...(commit === undefined ? {} : { commit }),
            [Symbol.asyncDispose]: async () => {},
        };
    }

    /** Fetch a repository commit into a new working tree, through the space's read lease. */
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
        workspace: Workspace,
        opened: SourceDirectory,
        outputs: readonly string[],
        signal: AbortSignal,
    ): Promise<BuildReader> {
        // stamp a build of uncommitted source as a prerelease
        const request = {
            outputs: outputsOf(workspace.root.packageJson, outputs),
            isPrerelease: opened.commit === undefined,
            signal,
        };

        return workspace.build(request);
    }

    /** Read a build's replica. */
    async #replicated(id: Identifier<"build">): Promise<Select<typeof build.table> | undefined> {
        const [row] = await this.#machine.database
            .select()
            .from(build.table)
            .where(eq(build.table.id, id));

        return row;
    }
}
