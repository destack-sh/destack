import { originalPositionFor, TraceMap } from "@jridgewell/trace-mapping";
import type { graph } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import type { StackFrame } from "./frame.ts";

/** Symbolicates stack frames against one build: maps them through its source maps and resolves them to its graph's symbols and declarations. */
export class Symbolicator {
    /** The build the frames ran: its source maps, graph and files. */
    readonly #reader: Pick<BuildReader, "sourceMaps" | "graph" | "module" | "load">;
    /** The loaded source maps by generated path. */
    readonly #maps = new Map<string, Promise<TraceMap>>();
    /** The build's generated files with their maps, read once. */
    #references: Promise<readonly { generated: string; map: string }[]> | undefined;

    /** Each module's graph file and source line starts, by package path. */
    readonly #modules = new Map<
        string,
        Promise<{ module: graph.Module; lines: number[] } | undefined>
    >();

    /** Symbolicate frames against a build. */
    constructor(reader: Pick<BuildReader, "sourceMaps" | "graph" | "module" | "load">) {
        this.#reader = reader;
    }

    /** Map each frame to its source, marking the package's own code as in app, and resolve in-app frames to their symbol and declaration. */
    async frames(frames: readonly StackFrame[]): Promise<StackFrame[]> {
        return Promise.all(frames.map(async (frame) => this.#attribute(await this.#map(frame))));
    }

    /** Map one frame through the source map of the generated file it ran. */
    async #map(frame: StackFrame): Promise<StackFrame> {
        // find the generated file the frame's file ends with
        this.#references ??= this.#reader.sourceMaps();
        const file = pathOf(frame.file);
        const reference = (await this.#references).find((candidate) =>
            file.endsWith(candidate.generated),
        );
        if (reference === undefined) {
            return { ...frame, isInApp: false };
        }

        // map the position to its original source
        let map = this.#maps.get(reference.generated);
        if (map === undefined) {
            map = this.#reader
                .load(reference.map)
                .then((bytes) => new TraceMap(new TextDecoder().decode(bytes)));
            this.#maps.set(reference.generated, map);
        }
        const original = originalPositionFor(await map, {
            line: frame.line,
            column: frame.column - 1,
        });
        if (original.source === null || original.line === null || original.column === null) {
            return { ...frame, isInApp: false };
        }

        // keep the package's own sources as in app, leaving dependencies out
        const path = sourcePath(original.source);

        return {
            ...(original.name === null ? frame : { ...frame, function: original.name }),
            file: frame.file,
            line: original.line,
            column: original.column + 1,
            path,
            isInApp: !path.includes("node_modules/"),
        };
    }

    /** Resolve one frame through its module's symbols and declarations. */
    async #attribute(frame: StackFrame): Promise<StackFrame> {
        // read the module of an in-app frame
        if (!frame.isInApp || frame.path === undefined) {
            return frame;
        }
        const found = await this.#module(frame.path);
        if (found === undefined) {
            return frame;
        }

        // find the innermost symbol around the frame's offset
        const offset = (found.lines[frame.line - 1] ?? 0) + frame.column - 1;
        const symbol = found.module.symbols
            .filter(
                (candidate) => candidate.source.start <= offset && offset < candidate.source.end,
            )
            .toSorted(
                (left, right) =>
                    left.source.end - left.source.start - (right.source.end - right.source.start),
            )[0];
        if (symbol === undefined) {
            return frame;
        }

        // find the declaration at the symbol, else the one its enclosing symbol declares, the most specific first
        const specificity = (candidate: graph.Declaration) =>
            candidate.moniker.startsWith(`${symbol.moniker}:`)
                ? 2
                : candidate.symbol === symbol.moniker
                  ? 1
                  : 0;
        const declaration = found.module.declarations
            .filter(
                (candidate) =>
                    candidate.moniker.startsWith(`${symbol.moniker}:`) ||
                    symbol.moniker === candidate.symbol ||
                    symbol.moniker.startsWith(`${candidate.symbol}.`),
            )
            .toSorted(
                (left, right) =>
                    specificity(right) - specificity(left) ||
                    right.symbol.length - left.symbol.length,
            )[0];

        return {
            ...frame,
            moniker: symbol.moniker,
            ...(declaration === undefined
                ? {}
                : {
                      declaration: {
                          moniker: declaration.moniker,
                          kind: declaration.kind,
                          name: declaration.name,
                      },
                  }),
        };
    }

    /** Read a module's graph file and the UTF-16 offsets its source lines start at, once per path. */
    #module(path: string): Promise<{ module: graph.Module; lines: number[] } | undefined> {
        let found = this.#modules.get(path);
        if (found === undefined) {
            found = (async () => {
                // find the module's graph file by its path
                const digest = (await this.#reader.graph()).modules[path];
                if (digest === undefined) {
                    return undefined;
                }

                // measure its source's line starts
                const source = new TextDecoder().decode(await this.#reader.load(path));
                const lines = [0];
                for (let index = 0; index < source.length; index++) {
                    if (source[index] === "\n") {
                        lines.push(index + 1);
                    }
                }

                return { module: await this.#reader.module(digest), lines };
            })();
            this.#modules.set(path, found);
        }

        return found;
    }
}

/** Read a frame file's path, a URL's path name or a plain path. */
function pathOf(file: string): string {
    return URL.canParse(file) ? new URL(file).pathname : file;
}

/** Read a source map source as a package-relative path. */
function sourcePath(source: string): string {
    return source.replace(/^(?:\.\.\/)+/u, "").replace(/^\.\//u, "");
}
