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
    console.log("Report whether language fences in specification fixtures parse and tokenize.");
    console.log("");
    console.log("Usage:");
    console.log("  node language/grammar/scripts/check-specification-fences.mjs [options]");
    console.log("");
    console.log("Options:");
    console.log("  --fail       exit non-zero when any fence fails");
    console.log("  --limit <n>  number of failed fence samples to print (default: 100)");
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

function fenceLanguage(info) {
    const tag = info.trim().split(/\s+/)[0].toLowerCase();

    for (const language of languageList) {
        if (language.matches(tag)) {
            return language;
        }
    }

    return null;
}

function collectLanguageFences(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const relativePath = path.relative(repoRoot, filePath);
    const fences = [];
    const expression = /^```([^\n]*)\n([\s\S]*?)^```/gm;
    let match;

    while ((match = expression.exec(source)) !== null) {
        const language = fenceLanguage(match[1]);
        if (!language) {
            continue;
        }

        const line = source.slice(0, match.index).split("\n").length;
        fences.push({
            filePath,
            relativePath,
            line,
            language: language.name,
            info: match[1].trim(),
            source: match[2],
        });
    }

    return fences;
}

function writeFence(fence, language, attempt, tempDirectory) {
    const safeName = [
        fence.index.toString().padStart(4, "0"),
        attempt.name,
        fence.relativePath.replaceAll(path.sep, "-").replaceAll(/[^A-Za-z0-9_.-]/g, "-"),
    ].join("-");
    const tempPath = path.join(tempDirectory, `${safeName}.${language.extension}`);

    fs.writeFileSync(tempPath, attempt.wrap(fence.source));

    return { ...fence, tempPath, attempt: attempt.name };
}

function parseBatch(fences, language) {
    const result = spawnSync(
        "bunx",
        ["tree-sitter-cli@0.24.4", "parse", "--quiet", "--stat", ...fences.map((fence) => fence.tempPath)],
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

// constructs the real parser accepts but the editor grammar deliberately
// rejects: supporting them would destabilize core expression disambiguation
const knownUnparsed = new Set([
    // static if guards in expression operand positions
    "language/test/fixtures/specification/expressions/static-if/validation.md:88",
]);

function parseFences(fences, language, tempDirectory) {
    let remaining = fences;
    const passedByAttempt = new Map();
    const batchSize = 100;

    for (const attempt of language.attempts) {
        const tempFences = remaining.map((fence) => writeFence(fence, language, attempt, tempDirectory));
        const failedFences = [];

        for (let index = 0; index < tempFences.length; index += batchSize) {
            const batch = tempFences.slice(index, index + batchSize);
            const result = parseBatch(batch, language);

            if (result.status !== 0 && result.failedPaths.size === 0) {
                failedFences.push(...batch);
            } else {
                failedFences.push(...batch.filter((fence) => result.failedPaths.has(fence.tempPath)));
            }
        }

        const failedIndexes = new Set(failedFences.map((fence) => fence.index));

        for (const fence of remaining) {
            if (!failedIndexes.has(fence.index)) {
                passedByAttempt.set(fence.index, attempt.name);
            }
        }

        remaining = failedFences.map(({ tempPath, attempt, ...fence }) => fence);

        if (remaining.length === 0) {
            break;
        }
    }

    return {
        failures: remaining.filter((fence) => !knownUnparsed.has(`${fence.relativePath}:${fence.line}`)),
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

function printParseBreakdown(language, fences, failures, passedByAttempt, limit) {
    const passed = language.attempts.map((attempt) => {
        const count = [...passedByAttempt.values()].filter((name) => name === attempt.name).length;
        return `${attempt.name}=${count}`;
    });

    console.log(`${language.name}: ${fences.length - failures.length}/${fences.length} fences parsed`);
    console.log(`passed by attempt: ${passed.join(" ")}`);

    if (failures.length === 0) {
        return;
    }

    console.log(`failed all attempts: ${failures.length}`);
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

async function smokeTextMateFences(fencesByLanguage) {
    const registry = await loadTextMateRegistry();
    const failuresByLanguage = new Map();

    for (const language of languageList) {
        const fences = fencesByLanguage.get(language.name) ?? [];
        const failures = [];

        if (fences.length === 0) {
            failuresByLanguage.set(language.name, failures);
            continue;
        }

        const grammar = await registry.loadGrammar(language.textmateScope);

        for (const fence of fences) {
            try {
                let ruleStack = null;

                for (const line of fence.source.split(/\r?\n/)) {
                    const result = grammar.tokenizeLine(line, ruleStack);
                    ruleStack = result.ruleStack;
                }
            } catch (error) {
                failures.push({ ...fence, error });
            }
        }

        failuresByLanguage.set(language.name, failures);
    }

    return failuresByLanguage;
}

function printTextMateBreakdown(language, fences, failures, limit) {
    console.log(`${language.name}: ${fences.length - failures.length}/${fences.length} fences tokenized`);

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
    const fences = markdownFiles.flatMap(collectLanguageFences).map((fence, index) => ({ ...fence, index }));
    const fencesByLanguage = new Map(languageList.map((language) => [
        language.name,
        fences.filter((fence) => fence.language === language.name),
    ]));
    const tempDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "destack-spec-fences-"));
    let hasFailures = false;

    try {
        console.log("language fence parse:");
        for (const language of languageList) {
            const languageFences = fencesByLanguage.get(language.name) ?? [];
            const result = parseFences(languageFences, language, tempDirectory);
            printParseBreakdown(language, languageFences, result.failures, result.passedByAttempt, options.limit);
            hasFailures ||= result.failures.length > 0;
        }

        console.log("");
        console.log("language TextMate smoke:");
        const textMateFailures = await smokeTextMateFences(fencesByLanguage);
        for (const language of languageList) {
            const languageFences = fencesByLanguage.get(language.name) ?? [];
            const failures = textMateFailures.get(language.name) ?? [];
            printTextMateBreakdown(language, languageFences, failures, options.limit);
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
