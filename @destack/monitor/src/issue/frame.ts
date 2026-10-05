import type { graph } from "@destack/package";

/** One frame of a stack trace, after Sentry's frames: where it ran, and once symbolicated the source and graph symbol it ran. */
export interface StackFrame {
    /** The function's name, absent for an anonymous one. */
    readonly function?: string;
    /** The file or URL the code ran from. */
    readonly file: string;
    /** The one-based line, in the source once symbolicated. */
    readonly line: number;
    /** The one-based column, in the source once symbolicated. */
    readonly column: number;
    /** Whether the frame runs the package's own code rather than a dependency's or the runtime's. */
    readonly isInApp: boolean;
    /** The package-relative source path, absent until a source map covers the frame. */
    readonly path?: string;
    /** The innermost graph symbol the frame ran, absent outside the package's graph. */
    readonly moniker?: graph.Moniker;
    /** The declaration the symbol is or belongs to, such as an object method, absent for undeclared code. */
    readonly declaration?: Pick<graph.Declaration, "moniker" | "kind" | "name">;
}

/** A V8 frame: `at name (file:line:column)` or `at file:line:column`. */
const V8_FRAME = /^\s*at (?:(?:async )?(.+?) \()?(.+?):(\d+):(\d+)\)?$/u;

/** A SpiderMonkey or JavaScriptCore frame: `name@file:line:column`. */
const GECKO_FRAME = /^\s*(?:(.*?)@)?(.+?):(\d+):(\d+)$/u;

/** Read a stack trace's frames, innermost first, in V8, SpiderMonkey and JavaScriptCore formats, taking dependency code as not in app. */
export function parseStack(stack: string): StackFrame[] {
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
}
