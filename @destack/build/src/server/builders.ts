import { access } from "node:fs/promises";
import { join, sep } from "node:path";
import { Workspace } from "@destack/check/workspace";
import type { PackageInspection } from "@destack/package/code";
import { Commit } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Checkout } from "../checkout/index.ts";
import { readDependencies } from "../source/dependency.ts";
import type { PackageStore } from "../store/index.ts";
import { type PackageBuild, readOutputs } from "../build/build.ts";
import { Compiler, PackageBuilder } from "../build/builder.ts";

/** The warm compilers a machine keeps at once: a compiler holds 0.5 to 1 GB, so four stay within a quarter of a 16 GB laptop. */
const MAX_WARM_COMPILERS = 4;

/** How long a compiler stays warm unused: fifteen minutes outlast a pause between edits, and a cold start costs seconds. */
const IDLE_MILLISECONDS = 15 * 60_000;

/** A package directory's warm compiler and when it was last used. */
interface Warm {
    /** The compiler, started once. */
    readonly compiler: Promise<PackageBuilder>;
    /** The last use, in Unix milliseconds, which orders eviction at the cap. */
    usedAt: number;
    /** The timer closing the compiler once it idles. */
    idle: ReturnType<typeof setTimeout>;
}

/** The warm package builders of a machine, one per package directory, each taking one request at a time. */
export class PackageBuilders implements AsyncDisposable {
    /** The compiler the builders start, with the toolchain directory it reads. */
    readonly #compiler: Compiler;
    /** The warm compilers, by package directory, at most MAX_WARM_COMPILERS. */
    readonly #warm = new Map<string, Warm>();
    /** The last work of each package directory's compiler, settled, since a compiler takes one request at a time. */
    readonly #queues = new Map<string, Promise<void>>();
    /** Report a compiler failing to close after it idled or was evicted. */
    readonly #report: (error: unknown) => void;

    /** Start builders with a compiler, reporting failures no request answers. */
    private constructor(compiler: Compiler, report: (error: unknown) => void) {
        this.#compiler = compiler;
        this.#report = report;
    }

    /** Open the builders with the current compiler. */
    static async open(report: (error: unknown) => void): Promise<PackageBuilders> {
        return new PackageBuilders(await Compiler.current(), report);
    }

    /** List the outputs a package directory's exports imply, by name. */
    async outputs(directory: string): Promise<readonly string[]> {
        return Object.keys(await readOutputs(directory));
    }

    /** Compile a package directory's named outputs at the commit it holds with its warm compiler through a cache, one request at a time. */
    compile(
        store: PackageStore,
        directory: string,
        outputs: readonly string[],
        commit: string | undefined,
        signal: AbortSignal,
    ): Promise<PackageBuild> {
        return this.#serially(directory, async () => {
            // refuse an output the package's exports do not imply
            const implied = await readOutputs(directory);
            const selected: Record<string, (typeof implied)[string]> = {};
            const unknown: string[] = [];
            for (const name of outputs) {
                const output = Object.hasOwn(implied, name) ? implied[name] : undefined;
                if (output === undefined) {
                    unknown.push(name);
                } else {
                    selected[name] = output;
                }
            }
            if (unknown.length > 0) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `the package builds no output ${unknown.join(", ")}`,
                });
            }

            // compile the named outputs with the package's dependencies
            const builder = await this.#builder(directory);

            return builder.build({
                dependencies: await readDependencies(directory),
                outputs: selected,
                ...(commit === undefined ? {} : { commit: Commit.parse(commit) }),
                signal,
                store,
            });
        });
    }

    /** Inspect a package directory for one of its outputs' runtimes with its warm compiler. */
    inspect(directory: string, output: string, signal?: AbortSignal): Promise<PackageInspection> {
        return this.#serially(directory, async () => {
            // require a module output the package's exports imply
            const implied = (await readOutputs(directory))[output];
            if (implied === undefined) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `the package builds no output ${output}`,
                });
            }
            const { runtime } = implied;

            return (await this.#builder(directory)).inspect({ runtime }, signal);
        });
    }

    /** Close the warm compilers of the packages below a directory, such as a working tree. */
    async release(root: string): Promise<void> {
        await this.#close(
            [...this.#warm.keys()].filter(
                (directory) => directory === root || directory.startsWith(`${root}${sep}`),
            ),
        );
    }

    /** List the directories whose files a package directory's build reads: its own and those of the workspace packages it depends on. */
    async sources(directory: string): Promise<string[]> {
        // read the package's workspace
        const workspace = await Workspace.read(directory);
        const member = workspace?.members.find(
            (candidate) => join(workspace.root, candidate.directory) === directory,
        );
        if (workspace === undefined || member === undefined) {
            return [directory];
        }

        return workspace
            .dependencies(member.directory)
            .map((dependency) => join(workspace.root, dependency))
            .toSorted();
    }

    /** List the Destack packages of a directory's workspace at or below the directory, relative to a working tree's root. */
    async members(root: string, directory: string): Promise<string[]> {
        // read the enclosing workspace
        const workspace = await Workspace.read(directory);
        if (workspace === undefined) {
            return [];
        }

        // keep the members at or below the directory that declare a Destack package
        const members = await Promise.all(
            workspace.members.map(async (member) => {
                // register a member at or below the directory inside the working tree with a destack.json
                const path = join(workspace.root, member.directory);
                const below = Checkout.below(root, path);
                const isRegistered =
                    below !== undefined &&
                    Checkout.below(directory, path) !== undefined &&
                    (await exists(join(path, "destack.json")));

                return isRegistered ? [below] : [];
            }),
        );

        return members.flat().toSorted();
    }

    /** Close the warm compilers. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#close([...this.#warm.keys()]);
    }

    /** Use a package directory's warm compiler, starting it once and evicting the least recently used beyond the cap. */
    #builder(directory: string): Promise<PackageBuilder> {
        // keep a warm compiler warm for another idle period
        const now = Date.now();
        const warm = this.#warm.get(directory);
        if (warm !== undefined) {
            warm.usedAt = now;
            warm.idle.refresh();

            return warm.compiler;
        }

        // start the compiler
        const compiler = PackageBuilder.start(directory, this.#compiler);
        const idle = setTimeout(
            () => void this.#close([directory]).catch(this.#report),
            IDLE_MILLISECONDS,
        );
        idle.unref();
        this.#warm.set(directory, { compiler, usedAt: now, idle });
        compiler.catch(() => {
            if (this.#warm.get(directory)?.compiler === compiler) {
                clearTimeout(idle);
                this.#warm.delete(directory);
            }
        });

        // close the least recently used compilers beyond the cap
        const evicted = [...this.#warm]
            .toSorted(([, left], [, right]) => right.usedAt - left.usedAt)
            .slice(MAX_WARM_COMPILERS)
            .map(([evictee]) => evictee);
        void this.#close(evicted).catch(this.#report);

        return compiler;
    }

    /** Run work on a package directory's compiler after the work before it settled. */
    #serially<Result>(directory: string, work: () => Promise<Result>): Promise<Result> {
        // queue the work behind the directory's last
        const previous = this.#queues.get(directory) ?? Promise.resolve();
        const running = previous.then(work);
        const settled = running.then(
            () => {},
            () => {},
        );
        this.#queues.set(directory, settled);

        // forget a settled queue nothing waits behind
        void settled.then(() => {
            if (this.#queues.get(directory) === settled) {
                this.#queues.delete(directory);
            }
        });

        return running;
    }

    /** Close the warm compilers of some package directories once their work settled. */
    async #close(directories: readonly string[]): Promise<void> {
        const closing = directories.flatMap((directory) => {
            const warm = this.#warm.get(directory);

            return warm === undefined ? [] : [{ directory, warm }];
        });
        await Promise.all(
            closing.map(({ directory, warm }) => {
                // forget the compiler at once
                clearTimeout(warm.idle);
                this.#warm.delete(directory);

                // close it behind the work queued before
                return this.#serially(directory, () =>
                    warm.compiler.then(
                        (compiler) => compiler[Symbol.asyncDispose](),
                        () => undefined,
                    ),
                );
            }),
        );
    }
}

/** Report whether a path names a file or directory that exists. */
async function exists(path: string): Promise<boolean> {
    return access(path).then(
        () => true,
        (error: unknown) => {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }

            return false;
        },
    );
}
