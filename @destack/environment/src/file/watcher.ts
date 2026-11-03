import type { FileEvent, WatchOptions } from "./file.ts";

/** A watch of a path: where it looks, how deep, and how it takes each change. */
interface FileWatch {
    /** The watched absolute path. */
    readonly path: string;
    /** Whether it reports changes inside subdirectories. */
    readonly isRecursive: boolean;
    /** Take a change. */
    readonly take: (event: FileEvent) => void;
}

/** The watcher of a world's files, reporting the changes its verbs make to each watch. */
export class FileWatcher {
    /** The watches of paths. */
    readonly #watches = new Set<FileWatch>();

    /** Report whether a watch sees the changes at an absolute path. */
    sees(path: string): boolean {
        return [...this.#watches].some((watch) => FileWatcher.#relative(watch, path) !== undefined);
    }

    /** Report a change at an absolute path to each watch that sees it, relative to its path. */
    emit(kind: FileEvent["kind"], path: string): void {
        for (const watch of this.#watches) {
            const relative = FileWatcher.#relative(watch, path);
            if (relative !== undefined) {
                watch.take({ kind, path: relative });
            }
        }
    }

    /** Report the changes at an absolute path until the signal aborts or the reader stops. */
    watch(path: string, options: WatchOptions = {}): AsyncIterable<FileEvent> {
        // buffer each change until read, waking a waiting reader
        const watches = this.#watches;
        const events: FileEvent[] = [];
        let wake: (() => void) | undefined;
        const watch: FileWatch = {
            path,
            isRecursive: options.recursive ?? false,
            take: (event) => {
                events.push(event);
                wake?.();
            },
        };
        watches.add(watch);

        // stop watching once the signal aborts
        const { signal } = options;
        const stop = () => {
            watches.delete(watch);
            wake?.();
        };
        signal?.addEventListener("abort", stop, { once: true });
        if (signal?.aborted === true) {
            stop();
        }

        // wait for the next change, as the reader does once the buffer runs dry
        const changed = () =>
            new Promise<void>((resolve) => {
                wake = resolve;
            });

        return (async function* () {
            try {
                // yield each buffered change, then end once stopped, or wait for the next
                for (;;) {
                    const event = events.shift();
                    if (event !== undefined) {
                        yield event;
                    } else if (signal?.aborted === true) {
                        return;
                    } else {
                        await changed();
                    }
                }
            } finally {
                // stop watching once the reader stops early
                signal?.removeEventListener("abort", stop);
                watches.delete(watch);
            }
        })();
    }

    /** Read a path as a watch sees it: the watched path by name, a path below relative to it. */
    static #relative(watch: FileWatch, path: string): string | undefined {
        // report the watched path by its name
        const prefix = watch.path === "/" ? "/" : `${watch.path}/`;
        const below = path.slice(prefix.length);
        if (path === watch.path) {
            return path.slice(path.lastIndexOf("/") + 1);
        }
        // report a path below it, deeper ones only to a recursive watch
        else if (path.startsWith(prefix) && (watch.isRecursive || !below.includes("/"))) {
            return below;
        }
        // leave a path it does not see
        else {
            return undefined;
        }
    }
}
