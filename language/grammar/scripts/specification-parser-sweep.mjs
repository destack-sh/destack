import path from "node:path";
import process from "node:process";
import fs from "node:fs";
import os from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const specificationRoot = path.resolve(
    scriptDirectory,
    "..",
    "..",
    "test",
    "fixtures",
    "specification"
);
const grammarRoot = path.resolve(scriptDirectory, "..", "destack");
const parserDirectoriesRoot = path.resolve(scriptDirectory, "..");

function parseArgs(argv) {
    const options = {
        limit: 40,
        json: "",
        failOnAny: false,
        failOnPositive: false,
    };

    for (let index = 0; index < argv.length; index += 1) {
        const token = argv[index];

        if ((token === "--limit" || token === "-n") && argv[index + 1]) {
            options.limit = Number.parseInt(argv[index + 1], 10);
            index += 1;
            continue;
        }

        if (token === "--json" && argv[index + 1]) {
            options.json = argv[index + 1];
            index += 1;
            continue;
        }

        if (token === "--fail-on-any") {
            options.failOnAny = true;
            continue;
        }

        if (token === "--fail-on-positive") {
            options.failOnPositive = true;
            continue;
        }

        if (token === "--help" || token === "-h") {
            printHelp();
            process.exit(0);
        }
    }

    if (!Number.isFinite(options.limit) || options.limit < 0) {
        throw new Error(`invalid --limit value: ${options.limit}`);
    }

    return options;
}

function printHelp() {
    console.log("Run a tree-sitter parser sweep across specification markdown fixtures.");
    console.log("");
    console.log("Usage:");
    console.log("  node language/grammar/scripts/specification-parser-sweep.mjs [options]");
    console.log("");
    console.log("Options:");
    console.log("  --limit, -n <N>         number of failing snippets to print (default: 40)");
    console.log("  --json <path>           write full machine-readable report");
    console.log("  --fail-on-any           exit non-zero if any parsed snippet has ERROR/MISSING");
    console.log("  --fail-on-positive      exit non-zero if any positive case has ERROR/MISSING");
}

function markdownFiles(directory) {
    const files = [];

    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
        const fullPath = path.join(directory, entry.name);

        if (entry.isDirectory()) {
            files.push(...markdownFiles(fullPath));
            continue;
        }

        if (entry.isFile() && fullPath.endsWith(".md")) {
            files.push(fullPath);
        }
    }

    return files.sort();
}

function parseCases(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const lines = source.split(/\r?\n/u);
    const cases = [];

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
        if (!/^#{3,4}\s+/u.test(lines[lineIndex])) {
            continue;
        }

        const caseStart = lineIndex;
        let caseEnd = lines.length;

        for (let lookahead = lineIndex + 1; lookahead < lines.length; lookahead += 1) {
            if (/^#{2,4}\s+/u.test(lines[lookahead])) {
                caseEnd = lookahead;
                break;
            }
        }

        const caseLines = lines.slice(caseStart, caseEnd);
        const caseTitle = (caseLines[0] ?? "").replace(/^#{3,4}\s+/u, "").trim();
        const hasExpectedErrors = caseLines.some((line) => /^\s*-\s+/u.test(line));
        const fences = [];

        let inFence = false;
        let fenceInfo = "";
        let fenceStartLine = 0;
        let buffer = [];

        for (let localIndex = 0; localIndex < caseLines.length; localIndex += 1) {
            const line = caseLines[localIndex];
            const fenceMatch = line.match(/^```([^`]*)$/u);

            if (!inFence) {
                if (fenceMatch) {
                    inFence = true;
                    fenceInfo = (fenceMatch[1] ?? "").trim();
                    fenceStartLine = caseStart + localIndex + 2;
                    buffer = [];
                }
                continue;
            }

            if (fenceMatch) {
                fences.push({
                    info: fenceInfo,
                    source: buffer.join("\n"),
                    startLine: fenceStartLine,
                });
                inFence = false;
                fenceInfo = "";
                buffer = [];
                continue;
            }

            buffer.push(line);
        }

        cases.push({
            title: caseTitle,
            hasExpectedErrors,
            fences,
        });

        lineIndex = caseEnd - 1;
    }

    return cases;
}

function parseTarget(info) {
    const token = info.trim().split(/\s+/u)[0]?.toLowerCase() ?? "";
    if (!token) {
        return null;
    }

    const colonIndex = token.indexOf(":");
    if (colonIndex >= 0) {
        return parseTargetFromFilename(token.slice(colonIndex + 1));
    }

    return parseTargetFromLanguage(token);
}

function parseTargetFromFilename(fileName) {
    const lower = fileName.toLowerCase();

    if (lower.endsWith(".d.tsx")) {
        return "tsx";
    }

    if (lower.endsWith(".d.ts")) {
        return "ts";
    }

    const extension = path.extname(lower);

    if (extension === ".ds") {
        return "ds";
    }

    if (extension === ".ts" || extension === ".mts" || extension === ".cts") {
        return "ts";
    }

    if (extension === ".tsx" || extension === ".jsx") {
        return "tsx";
    }

    if (extension === ".js" || extension === ".mjs" || extension === ".cjs") {
        return "ts";
    }

    return null;
}

function parseTargetFromLanguage(language) {
    if (language === "ds") {
        return "ds";
    }

    if (language === "ts") {
        return "ts";
    }

    if (language === "tsx" || language === "jsx") {
        return "tsx";
    }

    if (language === "js" || language === "mjs" || language === "cjs") {
        return "ts";
    }

    return null;
}

function extensionForTarget(target) {
    if (target === "ds") {
        return ".ds";
    }

    if (target === "tsx") {
        return ".tsx";
    }

    return ".ts";
}

function parseCliIssue(line) {
    const match = line.match(
        /^(.+?)\t[^\t]*\t[^\t]*\t\((ERROR|MISSING(?: [^\[]+)?) \[(\d+),\s*(\d+)\] - \[(\d+),\s*(\d+)\]\)$/u
    );

    if (!match) {
        return null;
    }

    const kind = match[2].startsWith("MISSING") ? "MISSING" : "ERROR";
    const text = match[2];

    return {
        file: path.resolve(match[1]),
        kind,
        text,
        row: Number.parseInt(match[3], 10) + 1,
        column: Number.parseInt(match[4], 10) + 1,
    };
}

function topCounts(values, limit) {
    return [...values.entries()]
        .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
        .slice(0, limit)
        .map(([name, count]) => ({ name, count }));
}

function sortedObjectFromMap(values) {
    return Object.fromEntries(
        [...values.entries()].sort((left, right) => left[0].localeCompare(right[0]))
    );
}

function maybeWriteJson(filePath, data) {
    if (!filePath) {
        return;
    }

    const outputPath = path.resolve(process.cwd(), filePath);
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, `${JSON.stringify(data, null, 2)}\n`, "utf8");
    console.log(`json report written: ${outputPath}`);
}

function main() {
    const options = parseArgs(process.argv.slice(2));
    const files = markdownFiles(specificationRoot);
    const parseEntries = [];
    let totalFenceBlocks = 0;
    let skippedFenceBlocks = 0;
    let positiveBlocks = 0;
    let negativeBlocks = 0;
    const parsedBlocksByParser = new Map();
    const positiveBlocksByParser = new Map();
    const negativeBlocksByParser = new Map();

    for (const filePath of files) {
        const cases = parseCases(filePath);

        for (const testCase of cases) {
            for (const fence of testCase.fences) {
                totalFenceBlocks += 1;

                const target = parseTarget(fence.info);
                if (!target || !fence.source.trim()) {
                    skippedFenceBlocks += 1;
                    continue;
                }

                if (testCase.hasExpectedErrors) {
                    negativeBlocks += 1;
                    negativeBlocksByParser.set(
                        target,
                        (negativeBlocksByParser.get(target) ?? 0) + 1
                    );
                } else {
                    positiveBlocks += 1;
                    positiveBlocksByParser.set(
                        target,
                        (positiveBlocksByParser.get(target) ?? 0) + 1
                    );
                }

                parsedBlocksByParser.set(target, (parsedBlocksByParser.get(target) ?? 0) + 1);

                parseEntries.push({
                    filePath,
                    case: testCase.title,
                    expectation: testCase.hasExpectedErrors ? "negative" : "positive",
                    fenceInfo: fence.info,
                    source: fence.source,
                    startLine: fence.startLine,
                    target,
                });
            }
        }
    }

    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "destack-spec-parse-"));
    const snippetDirectory = path.join(tempRoot, "snippets");
    const pathListFile = path.join(tempRoot, "paths.txt");
    const configFile = path.join(tempRoot, "tree-sitter-config.json");
    fs.mkdirSync(snippetDirectory, { recursive: true });

    const metadataBySnippetPath = new Map();
    const snippetPaths = [];

    for (let index = 0; index < parseEntries.length; index += 1) {
        const entry = parseEntries[index];
        const extension = extensionForTarget(entry.target);
        const fileName = `${String(index).padStart(5, "0")}${extension}`;
        const snippetPath = path.join(snippetDirectory, fileName);
        fs.writeFileSync(snippetPath, `${entry.source}\n`, "utf8");
        snippetPaths.push(snippetPath);
        metadataBySnippetPath.set(path.resolve(snippetPath), entry);
    }

    fs.writeFileSync(pathListFile, `${snippetPaths.join("\n")}\n`, "utf8");
    fs.writeFileSync(
        configFile,
        `${JSON.stringify({ "parser-directories": [parserDirectoriesRoot] })}\n`,
        "utf8"
    );

    const parseResult = spawnSync(
        "npx",
        [
            "tree-sitter",
            "parse",
            "-q",
            "--paths",
            pathListFile,
            "--config-path",
            configFile,
        ],
        {
            cwd: grammarRoot,
            encoding: "utf8",
        }
    );

    if (parseResult.error) {
        throw parseResult.error;
    }

    if (parseResult.status !== 0 && parseResult.status !== 1) {
        throw new Error(
            `tree-sitter parse failed with status ${parseResult.status}\n${parseResult.stdout}\n${parseResult.stderr}`
        );
    }

    const combinedOutput = `${parseResult.stdout}\n${parseResult.stderr}`;
    const failures = [];
    const failuresByFile = new Map();
    const failuresByCase = new Map();
    let positiveFailures = 0;
    let negativeFailures = 0;

    for (const line of combinedOutput.split(/\r?\n/u)) {
        const issue = parseCliIssue(line.trimEnd());
        if (!issue) {
            continue;
        }

        const metadata = metadataBySnippetPath.get(issue.file);
        if (!metadata) {
            continue;
        }

        const relativePath = path.relative(path.resolve(process.cwd(), "language"), metadata.filePath);
        const absoluteLine = metadata.startLine + issue.row - 1;
        const failure = {
            file: relativePath,
            case: metadata.case,
            expectation: metadata.expectation,
            fenceInfo: metadata.fenceInfo,
            parser: metadata.target,
            fileLine: absoluteLine,
            snippetLine: issue.row,
            snippetColumn: issue.column,
            issueKind: issue.kind,
            issueText: issue.text,
        };

        failures.push(failure);
        failuresByFile.set(relativePath, (failuresByFile.get(relativePath) ?? 0) + 1);
        failuresByCase.set(metadata.case, (failuresByCase.get(metadata.case) ?? 0) + 1);

        if (metadata.expectation === "positive") {
            positiveFailures += 1;
        } else {
            negativeFailures += 1;
        }
    }

    fs.rmSync(tempRoot, { recursive: true, force: true });

    const report = {
        files: files.length,
        totalFenceBlocks,
        parsedFenceBlocks: parseEntries.length,
        skippedFenceBlocks,
        positiveBlocks,
        negativeBlocks,
        parsedBlocksByParser: sortedObjectFromMap(parsedBlocksByParser),
        positiveBlocksByParser: sortedObjectFromMap(positiveBlocksByParser),
        negativeBlocksByParser: sortedObjectFromMap(negativeBlocksByParser),
        totalFailures: failures.length,
        positiveFailures,
        negativeFailures,
        topFailingFiles: topCounts(failuresByFile, 20),
        topFailingCases: topCounts(failuresByCase, 20),
        failures,
    };

    console.log(`# specification parser sweep`);
    console.log(`files=${report.files}`);
    console.log(`total_fence_blocks=${report.totalFenceBlocks}`);
    console.log(`parsed_fence_blocks=${report.parsedFenceBlocks}`);
    console.log(`skipped_fence_blocks=${report.skippedFenceBlocks}`);
    console.log(`positive_blocks=${report.positiveBlocks}`);
    console.log(`negative_blocks=${report.negativeBlocks}`);
    console.log(`parsed_blocks_by_parser=${JSON.stringify(report.parsedBlocksByParser)}`);
    console.log(`positive_blocks_by_parser=${JSON.stringify(report.positiveBlocksByParser)}`);
    console.log(`negative_blocks_by_parser=${JSON.stringify(report.negativeBlocksByParser)}`);
    console.log(`total_failures=${report.totalFailures}`);
    console.log(`positive_failures=${report.positiveFailures}`);
    console.log(`negative_failures=${report.negativeFailures}`);

    if (report.topFailingFiles.length > 0) {
        console.log(`## top failing files`);
        for (const entry of report.topFailingFiles) {
            console.log(`${entry.name}: ${entry.count}`);
        }
    }

    if (failures.length > 0 && options.limit > 0) {
        console.log(`## failure samples`);
        for (const failure of failures.slice(0, options.limit)) {
            console.log(
                `${failure.file}:${failure.fileLine} [${failure.expectation}] [${failure.parser}] ${failure.fenceInfo}`
            );
            console.log(`case: ${failure.case}`);
            console.log(
                `issue: ${failure.issueKind} at snippet ${failure.snippetLine}:${failure.snippetColumn}`
            );
            console.log(`node: ${failure.issueText}`);
            console.log("");
        }
    }

    maybeWriteJson(options.json, report);

    const failOnAny = options.failOnAny && report.totalFailures > 0;
    const failOnPositive = options.failOnPositive && report.positiveFailures > 0;

    if (failOnAny || failOnPositive) {
        process.exit(1);
    }
}

main();
