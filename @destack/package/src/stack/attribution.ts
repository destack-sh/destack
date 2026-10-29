import type { Document } from "../graph/document.ts";
import { type BuildReader, Once } from "../manifest/reader.ts";
import type { Module } from "../graph/module.ts";
import { type FrameModule, StackFrame } from "./frame.ts";

/** Attributes symbolicated frames to the symbols and declarations of one build's code index. */
export class Attribution {
    /** The build the frames ran, whose sources the frames point into. */
    readonly #reader: Pick<BuildReader, "load">;
    /** The module graph of the build's code index, by package-relative path. */
    readonly #graph: ReadonlyMap<string, Module>;
    /** The code of each module of the build's code index, by package-relative path. */
    readonly #code: ReadonlyMap<string, Document>;
    /** The located modules by path, each read once, absent for a path outside the index. */
    readonly #modules = new Map<string, Once<FrameModule | undefined>>();

    /** Attribute frames through a build and its code index: each module's graph and code. */
    constructor(
        reader: Pick<BuildReader, "load">,
        index: { readonly code: readonly Document[]; readonly modules: readonly Module[] },
    ) {
        // index the modules and their code by path
        this.#reader = reader;
        this.#graph = new Map(index.modules.map((module) => [module.path, module]));
        const { code } = index;
        this.#code = new Map(code.map((document) => [document.path, document]));
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

    /** Locate a module of the code index: its code, its graph's declarations and its lines. */
    #module(path: string): Promise<FrameModule | undefined> {
        // read each path once
        const once = this.#modules.get(path) ?? new Once<FrameModule | undefined>();
        this.#modules.set(path, once);

        return once.get(async () => {
            // leave a path the code index lacks, else read its source's lines
            const code = this.#code.get(path);
            const module = this.#graph.get(path);
            if (code === undefined || module === undefined) {
                return undefined;
            }
            const source = new TextDecoder().decode(await this.#reader.load(path));

            return { code, module, lines: StackFrame.lineStarts(source) };
        });
    }
}
