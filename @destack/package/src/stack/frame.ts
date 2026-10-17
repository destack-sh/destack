import { schema } from "@destack/schema";
import { Declaration } from "../graph/declaration.ts";
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
