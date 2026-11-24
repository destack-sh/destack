import { dirname, join } from "node:path";
import { BunEnvironment } from "@destack/environment/bun";
import { buildParserFile } from "@lezer/generator";
import { JAVASCRIPT_CHANGES } from "./changes.ts";

/** Where Lezer publishes the grammar of JavaScript. */
const GRAMMAR = new URL(import.meta.resolve("@lezer/javascript/package.json"));

/** The files of the process. */
const { files } = BunEnvironment.system;

/** The terms the tokenizers read. */
const TERMS = [
    "BlockComment",
    "Dialect_jsx",
    "incdec",
    "incdecPrefix",
    "insertSemi",
    "JSXStartTag",
    "LineComment",
    "newline",
    "noSemi",
    "noSemiType",
    "questionDot",
    "spaces",
];

/** The line generated files start with. */
const HEADER = "// generate with `bun run generate` from Lezer's grammar of JavaScript";

/** Change each rule of the published grammar once. */
async function readGrammar(): Promise<string> {
    let grammar = await files.readTextFile(new URL("src/javascript.grammar", GRAMMAR).pathname);
    for (const { from, to } of JAVASCRIPT_CHANGES) {
        const pieces = grammar.split(from);
        if (pieces.length !== 2) {
            throw new TypeError(`grammar change is invalid: '${from}' is not there once`);
        }
        grammar = pieces.join(to);
    }

    return grammar;
}

/** Build the parser, and the terms its tokenizers read. */
async function generate(): Promise<void> {
    // build both from the grammar
    const { parser, terms } = buildParserFile(await readGrammar(), { typeScript: true });
    const directory = dirname(new URL(import.meta.url).pathname);
    const written = parser
        .replace(/^\/\/ .*\n/u, `${HEADER}\n\n`)
        .replaceAll(/from "\.\/(tokens|highlight)(?:\.js)?"/gu, 'from "./$1.ts"')
        .replaceAll("|| -1}", "?? -1}")
        .replaceAll(/^const (spec_\w+)/gmu, "/** The words of the grammar. */\nconst $1")
        .replace("export const parser", "/** The parser of JavaScript. */\nexport const parser");
    const numbers = TERMS.map((term) => {
        const number = new RegExp(`\\b${term} = (\\d+)`, "u").exec(terms)?.[1];
        if (number === undefined) {
            throw new TypeError(`grammar is invalid: it has no term '${term}'`);
        }

        return `/** The term \`${term}\`. */\nexport const ${term} = ${number};\n`;
    });

    // write them beside this module
    await files.writeTextFile(join(directory, "parser.ts"), written);
    await files.writeTextFile(join(directory, "terms.ts"), `${HEADER}\n\n${numbers.join("\n")}`);
}

await generate();
