import { readPackageJson } from "@destack/build/local";
import { type FileSystem, Telemetry } from "@destack/environment";
import { relative, sep } from "node:path";
import type { PackageJson } from "@destack/package";
import { PackageName } from "@destack/package";
import { InstallationTarget } from "../catalog/installation.ts";
import { reference } from "@destack/registry/object";
import type { GitAuthor } from "@destack/git";
import { type Commit, type Identifier, present } from "@destack/schema";
import { type ObjectReference, RequestId } from "@destack/sync";
import { TelemetryScope } from "@destack/telemetry";
import type { CommandContext } from "../command/context.ts";
import { CliError } from "../error/index.ts";

/** The path of a repository at the universe's forge: `/@handle/name.git` below its origin. */
const REPOSITORY_PATH = /^\/(@[^/]+\/[^/]+)\.git$/u;

/** How long a package's files stay quiet before a watch pushes them: a save of several files. */
const SETTLE_MILLISECONDS = 200;

/** The directories whose changes no build reads: Git's files and installed dependencies. */
const IGNORED_DIRECTORIES = new Set([".git", "node_modules"]);

/** A directory's checkout on this computer, which pushes its working tree to its repository. */
export class LocalCheckout {
    /** The checkout registered on this computer's machine. */
    readonly id: Identifier<"checkout">;
    /** The machine administering the checkout. */
    readonly machine: Identifier<"machine">;
    /** The directory's path below the checkout's root, `.` for the root. */
    readonly path: string;
    /** The directory's canonical absolute path on this computer. */
    readonly directory: string;
    /** The root of the checkout's working tree on this computer. */
    readonly root: string;
    /** The package the directory declares. */
    readonly package: PackageJson["package"];
    /** The forge repository the working tree's origin points at. */
    readonly repository: Identifier<"repository">;
    /** The name of the repository, such as `@ada/apps`. */
    readonly name: string;
    /** The author of the commits the checkout's pushes make. */
    readonly author: GitAuthor;

    /** Take a checkout's directory and repository. */
    private constructor(
        fields: Pick<
            LocalCheckout,
            | "id"
            | "machine"
            | "path"
            | "directory"
            | "root"
            | "package"
            | "repository"
            | "name"
            | "author"
        >,
    ) {
        // set the checkout, its directory, its package and its repository
        this.id = fields.id;
        this.machine = fields.machine;
        this.path = fields.path;
        this.directory = fields.directory;
        this.root = fields.root;
        this.package = fields.package;
        this.repository = fields.repository;
        this.name = fields.name;
        this.author = fields.author;
    }

    /** Open the checkout of a package directory, registering its working tree once. */
    static async open(directory: string, context: CommandContext): Promise<LocalCheckout> {
        // find the checkout of the directory, registering its working tree when none has it
        const client = await context.native();
        const canonical = await context.environment.files.realPath(directory);
        const { machineId } = await client.loopback.computer();
        const opened = await client.machine().checkout.create({
            machineId,
            requestId: RequestId.create(client.environment),
            directory: canonical,
            source: { kind: "existing" },
        });
        const below = relative(opened.directory, canonical);

        // read the package and the forge repository of the working tree's origin
        const packageJson = await readPackageJson(canonical, context.environment.files);
        const name = await LocalCheckout.#origin(opened.remote, canonical, context);
        const { client: registry, home, accountId } = await context.registry(name);
        const [found] = (
            await registry.repository.list({
                spaceId: home,
                where: { accountId, name: PackageName.split(name).local },
                limit: 1,
            })
        ).items;
        if (found === undefined) {
            throw CliError.of("REPOSITORY_NOT_FOUND", `repository '${name}' not found`);
        }

        return new LocalCheckout({
            id: opened.id,
            machine: machineId,
            path: below === "" ? "." : below,
            directory: canonical,
            root: opened.directory,
            package: packageJson.package,
            repository: found.id,
            name,
            author: await (await context.launcher()).author(),
        });
    }

    /** Push the working tree as it is, or HEAD with no uncommitted change, to the reference. */
    async push(
        context: CommandContext,
        of: "tree" | "head",
    ): Promise<{ readonly commit: Commit; readonly reference: ObjectReference }> {
        // lease the repository to write as the person, and push with the lease
        const client = await context.native();
        const { client: registry, home } = await context.registry(this.name);
        const lease = await registry.repository.push({ spaceId: home, id: this.repository });
        const pushed = await client.machine().checkout.push({
            machineId: this.machine,
            id: this.id,
            requestId: RequestId.create(client.environment),
            lease,
            of,
            author: this.author,
        });

        // observe the repository's references again, then read the pushed one
        await registry.repository.refresh({
            spaceId: home,
            id: this.repository,
            requestId: RequestId.create(client.environment),
        });
        const [row] = (
            await registry.reference.list({
                spaceId: home,
                where: { parentId: this.repository, name: pushed.reference },
                limit: 1,
            })
        ).items;
        const recorded = present(row, `the reference '${pushed.reference}' of '${this.name}'`);

        return {
            commit: pushed.commit,
            reference: reference.reference(recorded.scope, recorded.id),
        };
    }

    /** Push the working tree, and describe the installation following its reference. */
    async target(context: CommandContext): Promise<InstallationTarget> {
        const { commit, reference: version } = await this.push(context, "tree");
        const { id, name } = this.package;

        return new InstallationTarget({
            packageId: id,
            version,
            directory: this.path,
            fallback: PackageName.split(name).local,
            output: { package: name, repository: this.name, commit, directory: this.path },
            detail: `${name} from checkout '${this.path}' · updates on each push`,
        });
    }

    /** Push the working tree again once each burst of changes settles, until stopped. */
    async watch(
        context: CommandContext,
        signal: AbortSignal,
        pushed: (commit: Commit) => Promise<void>,
    ): Promise<void> {
        // stop the watcher and the settling with the watch, however it ends
        const watching = new AbortController();
        let settling = new AbortController();
        try {
            // wake once a burst of changes outside Git's files and dependencies settles, failing with the watcher
            const { files, clock } = context.environment;
            let wake = awake();
            const changed = (path: string) => {
                if (path.split(sep).some((segment) => IGNORED_DIRECTORIES.has(segment))) {
                    return;
                }
                settling.abort();
                const period = new AbortController();
                settling = period;
                clock.sleep(SETTLE_MILLISECONDS, period.signal).then(
                    () => wake.resolve(),
                    () => undefined,
                );
            };
            const stopping = AbortSignal.any([signal, watching.signal]);
            LocalCheckout.#changes(this.root, files, stopping, changed).catch((error: unknown) =>
                wake.reject(error),
            );

            // push after each settled burst until stopped
            const { telemetry } = context.environment;
            const { span } = TelemetryScope.of(import.meta.destack.package, telemetry);
            const save = async () => {
                const { commit } = await this.push(context, "tree");
                await pushed(commit);
            };
            const stopped = aborted(signal);
            while (!signal.aborted) {
                await Promise.race([wake.promise, stopped]);
                if (!signal.aborted) {
                    wake = awake();

                    // save in a trace of its own, linked to the command's
                    const attributes = { "destack.checkout.path": this.path };
                    await Telemetry.detach(telemetry, (links) =>
                        span("destack.checkout.save", attributes, save, { links }),
                    );
                }
            }
        } finally {
            settling.abort();
            watching.abort();
        }
    }

    /** Report each change below a directory until stopped. */
    static async #changes(
        directory: string,
        files: FileSystem,
        signal: AbortSignal,
        changed: (path: string) => void,
    ): Promise<void> {
        for await (const change of await files.watch(directory, { recursive: true, signal })) {
            changed(change.path);
        }
    }

    /** Read the forge repository a checkout's origin points at, as `@handle/name`. */
    static async #origin(
        remote: string | null,
        directory: string,
        context: CommandContext,
    ): Promise<string> {
        // take a repository below the universe's origin, refusing a working tree without one
        const origin = remote === null ? null : URL.parse(remote);
        const universe = new URL((await context.launcher()).session.universe);
        const name = origin === null ? undefined : REPOSITORY_PATH.exec(origin.pathname)?.[1];
        if (origin?.origin !== universe.origin || name === undefined) {
            throw CliError.of(
                "REPOSITORY_NOT_FOUND",
                `repository of '${directory}' not found: its origin is no repository of '${universe.origin}'`,
            );
        }

        return name;
    }
}

/** Open a wake of a watch, whose failure counts as handled until awaited. */
function awake(): PromiseWithResolvers<void> {
    const wake = Promise.withResolvers<void>();
    wake.promise.catch(() => undefined);

    return wake;
}

/** Resolve once a signal aborts, at once for an aborted one. */
function aborted(signal: AbortSignal): Promise<void> {
    return new Promise((resolve) => {
        if (signal.aborted) {
            resolve();
        }
        signal.addEventListener("abort", () => resolve(), { once: true });
    });
}
