import { sep } from "node:path";
import { Compiler, CompilerCommand } from "./compiler.ts";

/** The warm compilers a machine keeps at once. */
const MAX_WARM_COMPILERS = 4;

/** How long an unused compiler stays warm: fifteen minutes, since a cold start costs seconds. */
const IDLE_MILLISECONDS = 15 * 60_000;

/** A package directory's warm compiler and when it was last used. */
interface Warm {
    /** The compiler, started once. */
    readonly compiler: Promise<Compiler>;
    /** The last use, in Unix milliseconds, which orders eviction at the cap. */
    usedAt: number;
    /** The timer closing the compiler once it idles. */
    idle: ReturnType<typeof setTimeout>;
}

/** Warm compilers, one per package directory, closed when idle or beyond the cap. */
export class CompilerPool implements AsyncDisposable {
    /** The command the pool starts each compiler with. */
    readonly #command: CompilerCommand;
    /** The warm compilers, by package directory. */
    readonly #warm = new Map<string, Warm>();
    /** Report a compiler failing to close after it idled or was evicted. */
    readonly #report: (error: unknown) => void;

    /** Keep compilers started with a command, reporting failures no request answers. */
    private constructor(command: CompilerCommand, report: (error: unknown) => void) {
        this.#command = command;
        this.#report = report;
    }

    /** Open a pool starting the current compiler. */
    static async open(report: (error: unknown) => void): Promise<CompilerPool> {
        return new CompilerPool(await CompilerCommand.current(), report);
    }

    /** Use a directory's warm compiler, starting it once and closing the oldest beyond the cap. */
    compiler(directory: string): Promise<Compiler> {
        // keep a warm compiler warm for another idle period
        const now = Date.now();
        const warm = this.#warm.get(directory);
        if (warm !== undefined) {
            warm.usedAt = now;
            warm.idle.refresh();

            return warm.compiler;
        }

        // start the compiler, forgetting one that fails to start
        const compiler = Compiler.start(directory, this.#command);
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

    /** Close the warm compilers of the packages at or below a directory, such as a working tree. */
    async release(root: string): Promise<void> {
        await this.#close(
            [...this.#warm.keys()].filter(
                (directory) => directory === root || directory.startsWith(`${root}${sep}`),
            ),
        );
    }

    /** Close every warm compiler. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#close([...this.#warm.keys()]);
    }

    /** Forget the warm compilers of some directories, closing each after its queued requests. */
    async #close(directories: readonly string[]): Promise<void> {
        // forget each warm compiler of the directories
        const closing = directories.flatMap((directory) => {
            // skip a directory without a warm compiler
            const warm = this.#warm.get(directory);
            if (warm === undefined) {
                return [];
            }

            // stop its idle timer and drop it from the pool
            clearTimeout(warm.idle);
            this.#warm.delete(directory);

            return [warm.compiler];
        });

        // close each after its queued requests, ignoring one that failed to start
        await Promise.all(
            closing.map((compiler) =>
                compiler.then(
                    (started) => started[Symbol.asyncDispose](),
                    () => undefined,
                ),
            ),
        );
    }
}
