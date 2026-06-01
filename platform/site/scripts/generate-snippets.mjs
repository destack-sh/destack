import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { highlightDestackFile } from "./highlight.mjs";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const snippetDirectory = join(siteDirectory, "src/snippets");
const generatedSnippetFile = join(siteDirectory, "src/generated/snippets.ts");
const isCheck = process.argv.includes("--check");

const snippetOrder = [
    "types/primitives",
    "types/intervals",
    "types/newtypes",
    "types/extensions",
    "types/enums",
    "types/structs",
    "types/tuples",
    "types/slices",
    "types/arrays",
    "types/readonly",
    "types/generics",
    "types/constraints",
    "types/associated",
    "types/reflection",
    "types/tagged-unions",
    "types/static",
    "types/any",
    "expressions/blocks",
    "expressions/patterns",
    "expressions/let-else",
    "expressions/match",
    "expressions/is",
    "expressions/loops",
    "expressions/using",
    "expressions/ranges",
    "expressions/overloads",
    "expressions/errors",
    "expressions/tsx",
    "expressions/decorators",
    "expressions/module",
    "expressions/comptime",
    "expressions/macros",
    "expressions/captures",
    "memory/space",
    "memory/ownership",
    "memory/borrowing",
    "memory/lifetimes",
    "memory/drop",
    "memory/dispose",
    "memory/unsafe",
    "memory/polymorphism",
    "runtime/modules",
    "runtime/conditions",
    "runtime/import-meta",
    "runtime/data-modules",
    "runtime/policy",
    "runtime/test",
    "runtime/fuzz",
    "runtime/bench",
    "runtime/simulation",
];

const entries = snippetOrder.map((name) => {
    const file = join(snippetDirectory, `${name}.ds`);
    const source = readFileSync(file, "utf8");
    const html = highlightDestackFile(file, source);

    return [name, { html, source }];
});

const generatedSnippetSource = `export const snippets = ${JSON.stringify(Object.fromEntries(entries), null, 4)} as const;\n`;

if (isCheck) {
    if (!existsSync(generatedSnippetFile)) {
        throw new Error(`missing generated snippet file: ${generatedSnippetFile}`);
    }

    const currentSnippetSource = readFileSync(generatedSnippetFile, "utf8");
    if (currentSnippetSource !== generatedSnippetSource) {
        throw new Error("generated snippets are out of date, run `just platform/site/format`");
    }
} else {
    mkdirSync(dirname(generatedSnippetFile), { recursive: true });
    writeGeneratedFile(generatedSnippetFile, generatedSnippetSource);
}

const snippetFiles = listSnippetFiles(snippetDirectory, snippetDirectory);
const expectedFiles = new Set(snippetOrder.map((name) => `${name}.ds`));

for (const file of snippetFiles) {
    if (!expectedFiles.has(file)) {
        throw new Error(`unexpected snippet file: ${file}`);
    }
}

for (const file of expectedFiles) {
    if (!snippetFiles.has(file)) {
        throw new Error(`missing snippet file: ${file}`);
    }
}

function writeGeneratedFile(file, source) {
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}

function listSnippetFiles(rootDirectory, directory) {
    const files = new Set();

    for (const entry of readdirSync(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            for (const file of listSnippetFiles(rootDirectory, path)) {
                files.add(file);
            }
        } else if (entry.isFile() && entry.name.endsWith(".ds")) {
            const file = relative(rootDirectory, path).replaceAll("\\", "/");
            files.add(file);
        } else {
            const file = relative(rootDirectory, path).replaceAll("\\", "/");
            throw new Error(`unexpected snippet file: ${file}`);
        }
    }

    return files;
}
