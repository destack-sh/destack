import { parser } from "./parser.ts";

/** The parser of JavaScript, of TypeScript, and of either with elements. */
export const javascriptParser = {
    jsx: parser.configure({ dialect: "jsx" }),
    tsx: parser.configure({ dialect: "ts jsx" }),
    ts: parser.configure({ dialect: "ts" }),
};
