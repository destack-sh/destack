import { Once } from "@destack/environment";
import type { BuildReader } from "../manifest/reader.ts";
import type { Module } from "../graph/module.ts";
import { type FrameModule, StackFrame } from "./frame.ts";

/** Attributes symbolicated frames to the symbols and declarations of one build's graph. */
export class Attribution {
    /** The build the frames ran, whose sources the frames point into. */
    readonly #reader: Pick<BuildReader, "load">;
    /** The graph file of each module of the build, by package-relative path. */
    readonly #graph: ReadonlyMap<string, Module>;
    /** The located modules by path, each read once, absent for a path outside the index. */
    readonly #modules = new Map<string, Once<FrameModule | undefined>>();

    /** Attribute frames through a build and its modules' graph files. */
    constructor(reader: Pick<BuildReader, "load">, modules: readonly Module[]) {
        // index the modules by path
        this.#reader = reader;
        this.#graph = new Map(modules.map((module) => [module.path, module]));
    }

    /** Attribute each in-app frame to its innermost symbol and that symbol's declaration. */
    async frames(frames: readonly StackFrame[]): Promise<StackFrame[]> {
        return Promise.all(
            frames.map(async (frame) => {
                // leave a frame outside the package's own modules as it is
                const located =
                    frame.path === undefined ? undefined : await this.#module(frame.path);

                return located === undefined ? frame : StackFrame.attribute(frame, located);
            }),
        );
    }

    /** Locate a module of the graph: its symbols, its declarations and its lines. */
    #module(path: string): Promise<FrameModule | undefined> {
        // read each path once
        const once = this.#modules.get(path) ?? new Once<FrameModule | undefined>();
        this.#modules.set(path, once);

        return once.get(async () => {
            // leave a path the graph lacks, else read its source's lines
            const module = this.#graph.get(path);
            if (module === undefined) {
                return undefined;
            }
            const source = new TextDecoder().decode(await this.#reader.load(path));

            return { module, lines: StackFrame.lineStarts(source) };
        });
    }
}
