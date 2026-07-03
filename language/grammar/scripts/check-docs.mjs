import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, "..", "..", "..");
const specificationRoot = path.join(repoRoot, "language", "test", "fixtures", "specification");
const designPath = path.join(repoRoot, "language", "DESIGN.md");
const mirReadmePath = path.join(repoRoot, "language", "mir", "README.md");
const bridgeVscodeRoot = path.join(repoRoot, "bridge", "vscode");
const require = createRequire(import.meta.url);
const languages = {
    destack: {
        name: "destack",
        extension: "ds",
        grammarRoot: path.join(repoRoot, "language", "grammar", "destack", "destack"),
        textmateGrammarPath: path.join(bridgeVscodeRoot, "destack.tmLanguage.json"),
        textmateScope: "source.ds",
        matches: (tag) => tag === "ds" || tag.startsWith("ds:"),
        attempts: [
            {
                name: "source",
                wrap: (source) => source,
            },
            {
                name: "expression",
                wrap: (source) => `const __destackGrammarExpression = (${source.trim()});\n`,
            },
            {
                name: "type",
                wrap: (source) => `type __DestackGrammarType = ${source.trim()};\n`,
            },
            {
                name: "type-lines",
                wrap: (source) => source
                    .split(/\r?\n/u)
                    .map((line) => line.replace(/\/\/.*$/u, "").trim())
                    .filter(Boolean)
                    .map((line, index) => `type __DestackGrammarType${index} = ${line};`)
                    .join("\n"),
            },
            {
                name: "statement",
                wrap: (source) => `function __destackGrammarStatement() {\n${source}\n}\n`,
            },
        ],
    },
    mir: {
        name: "mir",
        extension: "mir",
        grammarRoot: path.join(repoRoot, "language", "grammar", "mir"),
        textmateGrammarPath: path.join(bridgeVscodeRoot, "destack-mir.tmLanguage.json"),
        textmateScope: "source.dsmir",
        matches: (tag) => tag === "mir" || tag.startsWith("mir:") || tag === "dsmir" || tag.startsWith("dsmir:"),
        attempts: [
            {
                name: "source",
                wrap: (source) => source,
            },
        ],
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

function writeExample(example, language, attempt, tempDirectory) {
    const safeName = [
        example.index.toString().padStart(4, "0"),
        attempt.name,
        example.relativePath.replaceAll(path.sep, "-").replaceAll(/[^A-Za-z0-9_.-]/g, "-"),
    ].join("-");
    const tempPath = path.join(tempDirectory, `${safeName}.${language.extension}`);

    fs.writeFileSync(tempPath, attempt.wrap(example.source));

    return { ...example, tempPath, attempt: attempt.name };
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
    let remaining = examples;
    const passedByAttempt = new Map();
    const batchSize = 100;

    for (const attempt of language.attempts) {
        const tempExamples = remaining.map((example) => writeExample(example, language, attempt, tempDirectory));
        const failedExamples = [];

        for (let index = 0; index < tempExamples.length; index += batchSize) {
            const batch = tempExamples.slice(index, index + batchSize);
            const result = parseBatch(batch, language);

            if (result.status !== 0 && result.failedPaths.size === 0) {
                failedExamples.push(...batch);
            } else {
                failedExamples.push(...batch.filter((example) => result.failedPaths.has(example.tempPath)));
            }
        }

        const failedIndexes = new Set(failedExamples.map((example) => example.index));

        for (const example of remaining) {
            if (!failedIndexes.has(example.index)) {
                passedByAttempt.set(example.index, attempt.name);
            }
        }

        remaining = failedExamples.map(({ tempPath, attempt, ...example }) => example);

        if (remaining.length === 0) {
            break;
        }
    }

    return {
        failures: remaining,
        passedByAttempt,
    };
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

function printParseBreakdown(language, examples, result, limit) {
    const passed = language.attempts.map((attempt) => {
        const count = [...result.passedByAttempt.values()].filter((name) => name === attempt.name).length;
        return `${attempt.name}=${count}`;
    });

    console.log(`${language.name}: ${examples.length - result.failures.length}/${examples.length} examples parsed`);
    console.log(`passed by attempt: ${passed.join(" ")}`);

    if (result.failures.length === 0) {
        return;
    }

    console.log(`failed all attempts: ${result.failures.length}`);
    console.log("");
    console.log("by top directory:");
    for (const [name, count] of countBy(result.failures, (failure) => failure.relativePath.split(path.sep)[0])) {
        console.log(`${count.toString().padStart(3, " ")} ${name}`);
    }

    console.log("");
    console.log("by file:");
    for (const [name, count] of countBy(result.failures, (failure) => failure.relativePath)) {
        console.log(`${count.toString().padStart(3, " ")} ${name}`);
    }

    if (limit === 0) {
        return;
    }

    console.log("");
    console.log("samples:");
    for (const failure of result.failures.slice(0, limit)) {
        console.log(`${failure.relativePath}:${failure.line}: ${failure.info}: ${firstCodeLine(failure.source)}`);
    }
}

async function loadTextMateRegistry() {
    const textmate = require(path.join(bridgeVscodeRoot, "node_modules", "vscode-textmate"));
    const oniguruma = require(path.join(bridgeVscodeRoot, "node_modules", "vscode-oniguruma"));
    const wasmPath = path.join(bridgeVscodeRoot, "node_modules", "vscode-oniguruma", "release", "onig.wasm");
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

async function smokeTextMateExamples(examplesByLanguage) {
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
        ...collectMarkdownFiles(designPath),
        ...collectMarkdownFiles(specificationRoot),
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
            const result = parseExamples(languageExamples, language, tempDirectory);
            printParseBreakdown(language, languageExamples, result, options.limit);
            hasFailures ||= result.failures.length > 0;
        }

        console.log("");
        console.log("language TextMate smoke:");
        const textMateFailures = await smokeTextMateExamples(examplesByLanguage);
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
