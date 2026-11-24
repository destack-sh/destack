import type { Parser } from "@lezer/common";
import * as css from "@lezer/css";
import * as json from "@lezer/json";
import { GFM, parseCode, parser } from "@lezer/markdown";
import * as yaml from "@lezer/yaml";
import { Language, type LanguageName } from "../../language/index.ts";
import { htmlParser } from "../html.ts";
import { javascriptParser } from "../javascript/javascript.ts";
import { LezerGrammar } from "../lezer.ts";

/** The parsers of the code blocks colored inside a page, the rest staying plain. */
const EMBEDDED: Readonly<Partial<Record<LanguageName, Parser>>> = {
    css: css.parser,
    html: htmlParser,
    javascript: javascriptParser.jsx,
    json: json.parser,
    tsx: javascriptParser.tsx,
    typescript: javascriptParser.ts,
    yaml: yaml.parser,
};

/** The grammar of Markdown, with the code of its blocks. */
export default new LezerGrammar(
    Language.of("markdown"),
    parser.configure([
        GFM,
        parseCode({
            codeParser: (tag) => {
                const language = Language.ofTag(tag);

                return (language === undefined ? undefined : EMBEDDED[language.name]) ?? null;
            },
            htmlParser,
        }),
    ]),
);
