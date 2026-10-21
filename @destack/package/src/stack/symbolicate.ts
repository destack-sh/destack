import { originalPositionFor, TraceMap } from "@jridgewell/trace-mapping";
import { type BuildReader, Once } from "../manifest/reader.ts";
import type { StackFrame } from "./frame.ts";

/** Maps stack frames through one build's source maps to the sources they ran. */
export class Symbolicator {
    /** The build the frames ran: its source maps and files. */
    readonly #reader: Pick<BuildReader, "sourceMaps" | "load">;
    /** The source maps by generated path, each read once. */
    readonly #maps = new Map<string, Once<TraceMap>>();

    /** Map frames through a build's source maps. */
    constructor(reader: Pick<BuildReader, "sourceMaps" | "load">) {
        this.#reader = reader;
    }

    /** Map each frame to its source, marking the package's own code as in app. */
    async frames(frames: readonly StackFrame[]): Promise<StackFrame[]> {
        return Promise.all(frames.map((frame) => this.#map(frame)));
    }

    /** Map one frame through the source map of the generated file it ran. */
    async #map(frame: StackFrame): Promise<StackFrame> {
        // find the generated file the frame's file ends with
        const file = pathOf(frame.file);
        const reference = (await this.#reader.sourceMaps()).find((candidate) =>
            file.endsWith(candidate.generated),
        );
        if (reference === undefined) {
            return { ...frame, isInApp: false };
        }

        // map the position to its original source
        const once = this.#maps.get(reference.generated) ?? new Once<TraceMap>();
        this.#maps.set(reference.generated, once);
        const map = await once.get(async () => {
            const bytes = await this.#reader.load(reference.map);

            return new TraceMap(new TextDecoder().decode(bytes));
        });
        const original = originalPositionFor(map, {
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
}

/** Read a frame file's path, a URL's path name or a plain path. */
function pathOf(file: string): string {
    return URL.canParse(file) ? new URL(file).pathname : file;
}

/** Read a source map source as a package-relative path. */
function sourcePath(source: string): string {
    return source.replace(/^(?:\.\.\/)+/u, "").replace(/^\.\//u, "");
}
