import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, "..", "..", "..");
const specificationRoot = path.join(repoRoot, "language", "test", "fixtures", "specification");
const grammarRoot = path.join(repoRoot, "language", "grammar", "destack", "destack");
const attempts = [
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
        name: "statement",
        wrap: (source) => `function __destackGrammarStatement() {\n${source}\n}\n`,
    },
];

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
    console.log("Report whether ds fences in specification fixtures parse as Destack.");
    console.log("");
    console.log("Usage:");
    console.log("  node language/grammar/scripts/check-specification-fences.mjs [options]");
    console.log("");
    console.log("Options:");
    console.log("  --fail       exit non-zero when any fence fails");
    console.log("  --limit <n>  number of failed fence samples to print (default: 100)");
}

function collectMarkdownFiles(directory) {
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(directory, entry.name);

        if (entry.isDirectory()) {
            files.push(...collectMarkdownFiles(entryPath));
        } else if (entry.name.endsWith(".md")) {
            files.push(entryPath);
        }
    }

    return files.sort();
}

function collectDestackFences(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const relativePath = path.relative(specificationRoot, filePath);
    const fences = [];
    const expression = /^```ds\n([\s\S]*?)^```/gm;
    let match;

    while ((match = expression.exec(source)) !== null) {
        const line = source.slice(0, match.index).split("\n").length;
        fences.push({
            filePath,
            relativePath,
            line,
            source: match[1],
        });
    }

    return fences;
}

function writeFence(fence, attempt, tempDirectory) {
    const safeName = [
        fence.index.toString().padStart(4, "0"),
        attempt.name,
        fence.relativePath.replaceAll(path.sep, "-"),
    ].join("-");
    const tempPath = path.join(tempDirectory, `${safeName}.ds`);

    fs.writeFileSync(tempPath, attempt.wrap(fence.source));

    return { ...fence, tempPath, attempt: attempt.name };
}

function parseBatch(fences) {
    const result = spawnSync(
        "bunx",
        ["tree-sitter-cli@0.24.4", "parse", "--quiet", "--stat", ...fences.map((fence) => fence.tempPath)],
        {
            cwd: grammarRoot,
            encoding: "utf8",
        },
    );

    const failedPaths = new Set();
    const output = `${result.stdout}\n${result.stderr}`;
    const expression = /^(\S+\.ds)\s+.*\((?:ERROR|MISSING)[^)]*\)/gm;
    let match;

    while ((match = expression.exec(output)) !== null) {
        failedPaths.add(match[1]);
    }

    return {
        status: result.status ?? 1,
        failedPaths,
    };
}

function parseFences(fences, tempDirectory) {
    let remaining = fences;
    const passedByAttempt = new Map();
    const batchSize = 100;

    for (const attempt of attempts) {
        const tempFences = remaining.map((fence) => writeFence(fence, attempt, tempDirectory));
        const failedFences = [];

        for (let index = 0; index < tempFences.length; index += batchSize) {
            const batch = tempFences.slice(index, index + batchSize);
            const result = parseBatch(batch);

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

        remaining = failedFences.map((fence) => ({
            filePath: fence.filePath,
            relativePath: fence.relativePath,
            line: fence.line,
            source: fence.source,
            index: fence.index,
        }));

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

function printBreakdown(fences, failures, passedByAttempt, limit) {
    const passed = attempts.map((attempt) => {
        const count = [...passedByAttempt.values()].filter((name) => name === attempt.name).length;
        return `${attempt.name}=${count}`;
    });

    console.log(`specification fence parse: ${fences.length - failures.length}/${fences.length} ds fences ok`);
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
        console.log(`${failure.relativePath}:${failure.line}: ${firstCodeLine(failure.source)}`);
    }
}

function main() {
    const options = parseArgs(process.argv.slice(2));
    const markdownFiles = collectMarkdownFiles(specificationRoot);
    const fences = markdownFiles.flatMap(collectDestackFences).map((fence, index) => ({ ...fence, index }));
    const tempDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "destack-spec-fences-"));

    try {
        const result = parseFences(fences, tempDirectory);
        printBreakdown(fences, result.failures, result.passedByAttempt, options.limit);

        if (options.fail && result.failures.length > 0) {
            process.exit(1);
        }
    } finally {
        fs.rmSync(tempDirectory, { recursive: true, force: true });
    }
}

main();
