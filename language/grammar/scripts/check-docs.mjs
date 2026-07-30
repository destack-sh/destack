import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, "..", "..", "..");
const docsRoot = path.join(repoRoot, "docs");
const mirReadmePath = path.join(repoRoot, "language", "mir", "README.md");
const bridgeVscodeRoot = path.join(repoRoot, "bridge", "vscode");
const require = createRequire(import.meta.url);
const languages = {
    destack: {
        name: "destack",
        extension: "ds",
        grammarRoot: path.join(repoRoot, "language", "grammar", "destack", "destack"),
        textmateGrammarPath: path.join(bridgeVscodeRoot, "destack-ds.tmLanguage.json"),
        textmateScope: "source.ds",
        matches: (tag) => tag === "ds" || tag.startsWith("ds:"),
    },
    mir: {
        name: "mir",
        extension: "dsm",
        grammarRoot: path.join(repoRoot, "language", "grammar", "mir"),
        textmateGrammarPath: path.join(bridgeVscodeRoot, "destack-dsm.tmLanguage.json"),
        textmateScope: "source.dsm",
        matches: (tag) => tag === "mir" || tag.startsWith("mir:") || tag === "dsm" || tag.startsWith("dsm:"),
    },
    bytecode: {
        name: "bytecode",
        extension: "dsa",
        grammarRoot: path.join(repoRoot, "language", "grammar", "bytecode"),
        textmateGrammarPath: path.join(bridgeVscodeRoot, "destack-dsa.tmLanguage.json"),
        textmateScope: "source.dsa",
        matches: (tag) => tag === "dsa" || tag.startsWith("dsa:"),
    },
};
const languageList = Object.values(languages);

function parseArgs(argv) {
    const options = {
        fail: false,
        limit: 100,
    };

    for (let index = 0; index < argv.length; index += 1) {
        const token = argv[index];

        if (token === "--fail") {
            options.fail = true;
            continue;
        }

        if (token === "--limit" && argv[index + 1]) {
            options.limit = Number.parseInt(argv[index + 1], 10);
            index += 1;
            continue;
        }

        if (token === "--help" || token === "-h") {
            printHelp();
            process.exit(0);
        }

        throw new Error(`unknown argument: ${token}`);
    }

    if (!Number.isFinite(options.limit) || options.limit < 0) {
        throw new Error(`invalid --limit value: ${options.limit}`);
    }

    return options;
}

function printHelp() {
    console.log("Report whether language examples in docs parse and tokenize.");
    console.log("");
    console.log("Usage:");
    console.log("  node language/grammar/scripts/check-docs.mjs [options]");
    console.log("");
    console.log("Options:");
    console.log("  --fail       exit non-zero when any unexpected example fails");
    console.log("  --limit <n>  number of failed example samples to print (default: 100)");
}

function collectMarkdownFiles(sourcePath) {
    const stat = fs.statSync(sourcePath);

    if (stat.isFile()) {
        return sourcePath.endsWith(".md") ? [sourcePath] : [];
    }

    const entries = fs.readdirSync(sourcePath, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(sourcePath, entry.name);

        if (entry.isDirectory()) {
            files.push(...collectMarkdownFiles(entryPath));
        } else if (entry.name.endsWith(".md")) {
            files.push(entryPath);
        }
    }

    return files.sort();
}

function exampleLanguage(info) {
    const tag = info.trim().split(/\s+/)[0].toLowerCase();
    if (tag.endsWith(":unchecked")) {
        return null;
    }

    for (const language of languageList) {
        if (language.matches(tag)) {
            return language;
        }
    }

    return null;
}

function collectLanguageExamples(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const relativePath = path.relative(repoRoot, filePath);
    const examples = [];
    const expression = /^```([^\n]*)\n([\s\S]*?)^```/gm;
    let match;

    while ((match = expression.exec(source)) !== null) {
        const language = exampleLanguage(match[1]);
        if (!language) {
            continue;
        }

        const line = source.slice(0, match.index).split("\n").length;
        examples.push({
            filePath,
            relativePath,
            line,
            language: language.name,
            info: match[1].trim(),
            source: match[2],
        });
    }

    return examples;
}

function writeExample(example, language, tempDirectory) {
    const safeName = [
        example.index.toString().padStart(4, "0"),
        example.relativePath.replaceAll(path.sep, "-").replaceAll(/[^A-Za-z0-9_.-]/g, "-"),
    ].join("-");
    const tempPath = path.join(tempDirectory, `${safeName}.${language.extension}`);

    fs.writeFileSync(tempPath, example.source);

    return { ...example, tempPath };
}

function parseBatch(examples, language) {
    const result = spawnSync(
        "bunx",
        ["tree-sitter-cli@0.24.4", "parse", "--quiet", "--stat", ...examples.map((example) => example.tempPath)],
        {
            cwd: language.grammarRoot,
            encoding: "utf8",
        },
    );

    const failedPaths = new Set();
    const output = `${result.stdout}\n${result.stderr}`;
    const expression = new RegExp(`^(\\S+\\.${language.extension})\\s+.*\\((?:ERROR|MISSING)[^)]*\\)`, "gm");
    let match;

    while ((match = expression.exec(output)) !== null) {
        failedPaths.add(match[1]);
    }

    return {
        status: result.status ?? 1,
        failedPaths,
    };
}

function parseExamples(examples, language, tempDirectory) {
    const tempExamples = examples.map((example) => writeExample(example, language, tempDirectory));
    const failures = [];
    const batchSize = 100;

    for (let index = 0; index < tempExamples.length; index += batchSize) {
        const batch = tempExamples.slice(index, index + batchSize);
        const result = parseBatch(batch, language);

        if (result.status !== 0 && result.failedPaths.size === 0) {
            failures.push(...batch);
        } else {
            failures.push(...batch.filter((example) => result.failedPaths.has(example.tempPath)));
        }
    }

    return failures.map(({ tempPath, ...example }) => example);
}

function countBy(values, callback) {
    const counts = new Map();

    for (const value of values) {
        const key = callback(value);
        counts.set(key, (counts.get(key) ?? 0) + 1);
    }

    return [...counts.entries()].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
}

function firstCodeLine(source) {
    return source.split(/\r?\n/).map((line) => line.trim()).find(Boolean) ?? "";
}

function printParseBreakdown(language, examples, failures, limit) {
    console.log(`${language.name}: ${examples.length - failures.length}/${examples.length} examples parsed`);

    if (failures.length === 0) {
        return;
    }

    console.log(`unparsed: ${failures.length}`);
    console.log("");
    console.log("by top directory:");
    for (const [name, count] of countBy(failures, (failure) => failure.relativePath.split(path.sep)[0])) {
        console.log(`${count.toString().padStart(3, " ")} ${name}`);
    }

    console.log("");
    console.log("by file:");
    for (const [name, count] of countBy(failures, (failure) => failure.relativePath)) {
        console.log(`${count.toString().padStart(3, " ")} ${name}`);
    }

    if (limit === 0) {
        return;
    }

    console.log("");
    console.log("samples:");
    for (const failure of failures.slice(0, limit)) {
        console.log(`${failure.relativePath}:${failure.line}: ${failure.info}: ${firstCodeLine(failure.source)}`);
    }
}

async function loadTextMateRegistry() {
    const textmate = require("vscode-textmate");
    const oniguruma = require("vscode-oniguruma");
    const onigurumaPath = require.resolve("vscode-oniguruma");
    const wasmPath = path.join(
        path.dirname(onigurumaPath),
        "..",
        "release",
        "onig.wasm",
    );
    const wasm = fs.readFileSync(wasmPath);
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));

    return new textmate.Registry({
        onigLib: Promise.resolve({
            createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
            createOnigString: (source) => new oniguruma.OnigString(source),
        }),
        loadGrammar: (scopeName) => {
            const language = languageList.find((candidate) => candidate.textmateScope === scopeName);
            if (!language) {
                return null;
            }

            return JSON.parse(fs.readFileSync(language.textmateGrammarPath, "utf8"));
        },
    });
}

async function tokenizeTextMateExamples(examplesByLanguage) {
    const registry = await loadTextMateRegistry();
    const failuresByLanguage = new Map();

    for (const language of languageList) {
        const examples = examplesByLanguage.get(language.name) ?? [];
        const failures = [];

        if (examples.length === 0) {
            failuresByLanguage.set(language.name, failures);
            continue;
        }

        const grammar = await registry.loadGrammar(language.textmateScope);

        for (const example of examples) {
            try {
                let ruleStack = null;

                for (const line of example.source.split(/\r?\n/)) {
                    const result = grammar.tokenizeLine(line, ruleStack);
                    ruleStack = result.ruleStack;
                }
            } catch (error) {
                failures.push({ ...example, error });
            }
        }

        failuresByLanguage.set(language.name, failures);
    }

    return failuresByLanguage;
}

function printTextMateBreakdown(language, examples, failures, limit) {
    console.log(`${language.name}: ${examples.length - failures.length}/${examples.length} examples tokenized`);

    if (failures.length === 0 || limit === 0) {
        return;
    }

    console.log("");
    console.log("samples:");
    for (const failure of failures.slice(0, limit)) {
        console.log(`${failure.relativePath}:${failure.line}: ${failure.info}: ${failure.error.message}`);
    }
}

async function main() {
    const options = parseArgs(process.argv.slice(2));
    const markdownFiles = [
        ...collectMarkdownFiles(docsRoot),
        ...collectMarkdownFiles(mirReadmePath),
    ];
    const examples = markdownFiles.flatMap(collectLanguageExamples).map((example, index) => ({ ...example, index }));
    const examplesByLanguage = new Map(languageList.map((language) => [
        language.name,
        examples.filter((example) => example.language === language.name),
    ]));
    const tempDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "destack-docs-"));
    let hasFailures = false;

    try {
        console.log("language docs parse:");
        for (const language of languageList) {
            const languageExamples = examplesByLanguage.get(language.name) ?? [];
            const failures = parseExamples(languageExamples, language, tempDirectory);
            printParseBreakdown(language, languageExamples, failures, options.limit);
            hasFailures ||= failures.length > 0;
        }

        console.log("");
        console.log("language TextMate tokenization:");
        const textMateFailures = await tokenizeTextMateExamples(examplesByLanguage);
        for (const language of languageList) {
            const languageExamples = examplesByLanguage.get(language.name) ?? [];
            const failures = textMateFailures.get(language.name) ?? [];
            printTextMateBreakdown(language, languageExamples, failures, options.limit);
            hasFailures ||= failures.length > 0;
        }

        if (options.fail && hasFailures) {
            process.exit(1);
        }
    } finally {
        fs.rmSync(tempDirectory, { recursive: true, force: true });
    }
}

main();
