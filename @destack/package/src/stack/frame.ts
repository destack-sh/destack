import { schema } from "@destack/schema";
import { Declaration } from "../graph/declaration.ts";
import type { Document } from "../graph/document.ts";
import type { Module } from "../graph/module.ts";
import { Moniker } from "../graph/moniker.ts";

/** One frame of a stack trace. */
const stackFrame = schema.object({
    /** The function's name, absent for an anonymous one. */
    function: schema.string().exactOptional(),
    /** The file or URL the code ran from. */
    file: schema.string(),
    /** The one-based line, in the source once symbolicated. */
    line: schema.number().int(),
    /** The one-based column, in the source once symbolicated. */
    column: schema.number().int(),
    /** Whether the frame runs the package's own code, not a dependency's or the runtime's. */
    isInApp: schema.boolean(),
    /** The package-relative source path, absent until a source map covers the frame. */
    path: schema.string().exactOptional(),
    /** The innermost graph symbol the frame ran, absent outside the package's graph. */
    moniker: Moniker.exactOptional(),
    /** The declaration the symbol is or belongs to, such as an object method. */
    declaration: Declaration.pick({ moniker: true, kind: true, name: true }).exactOptional(),
});
/** One frame of a stack trace. */
export type StackFrame = schema.Infer<typeof stackFrame>;

/** A V8 frame: `at name (file:line:column)` or `at file:line:column`. */
const V8_FRAME = /^\s*at (?:(?:async )?(.+?) \()?(.+?):(\d+):(\d+)\)?$/u;

/** A SpiderMonkey or JavaScriptCore frame: `name@file:line:column`. */
const GECKO_FRAME = /^\s*(?:(.*?)@)?(.+?):(\d+):(\d+)$/u;

/** The frames of stack traces, and their schema. */
export const StackFrame = Object.assign(stackFrame, {
    /** Attribute an in-app frame to its innermost symbol and that symbol's declaration. */
    attribute(frame: StackFrame, located: FrameModule): StackFrame {
        // find the innermost symbol around the frame's offset, leaving a frame outside the module unresolved
        const lineStart = located.lines[frame.line - 1];
        if (!frame.isInApp || frame.path !== located.code.path || lineStart === undefined) {
            return frame;
        }
        const offset = lineStart + frame.column - 1;
        const symbol = located.code.symbols
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
        const specificity = (candidate: Declaration) =>
            candidate.moniker.startsWith(`${symbol.moniker}:`)
                ? 2
                : candidate.symbol === symbol.moniker
                  ? 1
                  : 0;
        const declaration = located.module.declarations
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
    },

    /** Read the UTF-16 offsets a source's lines start at, as frames count lines. */
    lineStarts(source: string): number[] {
        // start a line after each newline
        const lines = [0];
        for (let index = 0; index < source.length; index++) {
            if (source[index] === "\n") {
                lines.push(index + 1);
            }
        }

        return lines;
    },

    /** Read a stack trace's frames, innermost first. */
    read(stack: string): StackFrame[] {
        return stack.split("\n").flatMap((line) => {
            // read a V8 frame, else a Gecko or WebKit one, skipping the message and native frames
            const match = line.startsWith("    at ") ? V8_FRAME.exec(line) : GECKO_FRAME.exec(line);
            if (match === null) {
                return [];
            }
            const [, name, file = "", row = "0", column = "0"] = match;
            if (file === "native" || file.startsWith("node:")) {
                return [];
            }

            return [
                {
                    ...(name === undefined || name === "" ? {} : { function: name }),
                    file,
                    line: Number(row),
                    column: Number(column),
                    isInApp: !file.includes("node_modules/"),
                },
            ];
        });
    },
});

/** The module a frame ran: its code, its declarations and its source's line starts. */
export interface FrameModule {
    /** The module's code from the code index. */
    readonly code: Document;
    /** The module's graph file from the build. */
    readonly module: Pick<Module, "declarations">;
    /** The UTF-16 offsets its source's lines start at. */
    readonly lines: readonly number[];
}
