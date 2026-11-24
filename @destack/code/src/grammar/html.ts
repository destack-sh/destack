import * as css from "@lezer/css";
import { configureNesting, parser } from "@lezer/html";
import { javascriptParser } from "./javascript/javascript.ts";

/** The parser of HTML, with the styles and scripts a page embeds. */
export const htmlParser = parser.configure({
    wrap: configureNesting([
        { tag: "script", parser: javascriptParser.jsx },
        { tag: "style", parser: css.parser },
    ]),
});
