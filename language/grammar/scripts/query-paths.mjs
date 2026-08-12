import { readFileSync } from "node:fs";

const configPath = process.argv[2];
if (configPath == undefined) {
    throw new Error("usage: query-paths.mjs <tree-sitter.json>");
}

// collect every configured query path once, preserving declaration order
const config = JSON.parse(readFileSync(configPath, "utf8"));
const paths = new Set();
for (const grammar of config.grammars) {
    for (const kind of ["highlights", "injections", "locals", "tags"]) {
        const queries = grammar[kind];
        if (queries == undefined) {
            continue;
        }

        const entries = Array.isArray(queries) ? queries : [queries];
        for (const entry of entries) {
            paths.add(entry);
        }
    }
}

process.stdout.write(`${[...paths].join("\n")}\n`);
