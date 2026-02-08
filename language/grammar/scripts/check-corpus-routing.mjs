import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const rootCorpusDirectory = path.resolve(scriptDirectory, "..", "destack", "test", "corpus");
const destackCorpusDirectory = path.resolve(
    scriptDirectory,
    "..",
    "destack",
    "destack",
    "test",
    "corpus"
);
const allowedRootLanguages = new Set(["typescript", "tsx"]);

function readLines(filePath) {
    return fs.readFileSync(filePath, "utf8").split(/\r?\n/);
}

function checkRootCorpusFile(filePath, errors) {
    const lines = readLines(filePath);
    const languagePattern = /^:language\(([^)]+)\)$/;
    let sectionCount = 0;

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
        const line = lines[lineIndex];
        const match = line.match(languagePattern);
        if (!match) {
            continue;
        }

        const language = match[1];
        if (!allowedRootLanguages.has(language)) {
            errors.push(`${filePath}:${lineIndex + 1}: unsupported language directive ${language}`);
        }
    }

    for (let lineIndex = 0; lineIndex < lines.length - 3; lineIndex += 1) {
        const openingLine = lines[lineIndex].trim();
        if (!/^=+$/.test(openingLine)) {
            continue;
        }

        const titleLine = lines[lineIndex + 1] ?? "";
        const directiveLine = (lines[lineIndex + 2] ?? "").trim();
        const closingLine = (lines[lineIndex + 3] ?? "").trim();

        if (titleLine.trim() === "") {
            continue;
        }

        if (!languagePattern.test(directiveLine)) {
            if (/^=+$/.test(directiveLine)) {
                errors.push(`${filePath}:${lineIndex + 1}: missing :language(...) after section title`);
            }
            continue;
        }

        if (!/^=+$/.test(closingLine)) {
            errors.push(`${filePath}:${lineIndex + 1}: malformed section header block`);
            continue;
        }

        sectionCount += 1;

        const languageMatch = directiveLine.match(languagePattern);
        const language = languageMatch[1];
        if (!allowedRootLanguages.has(language)) {
            errors.push(`${filePath}:${lineIndex + 3}: unsupported language directive ${language}`);
        }

        lineIndex += 3;
    }

    return sectionCount;
}

function checkDestackCorpusFile(filePath, errors) {
    const lines = readLines(filePath);
    const languagePattern = /^:language\(/;

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
        if (languagePattern.test(lines[lineIndex])) {
            errors.push(`${filePath}:${lineIndex + 1}: unexpected :language(...) in destack corpus`);
        }
    }
}

function textFiles(directory) {
    return fs
        .readdirSync(directory)
        .filter((entry) => entry.endsWith(".txt"))
        .sort()
        .map((entry) => path.join(directory, entry));
}

function main() {
    const errors = [];

    const rootCorpusFiles = textFiles(rootCorpusDirectory);
    const destackCorpusFiles = textFiles(destackCorpusDirectory);

    if (rootCorpusFiles.length === 0) {
        errors.push(`no root corpus files found in ${rootCorpusDirectory}`);
    }

    if (destackCorpusFiles.length === 0) {
        errors.push(`no destack corpus files found in ${destackCorpusDirectory}`);
    }

    let rootSectionCount = 0;
    for (const filePath of rootCorpusFiles) {
        rootSectionCount += checkRootCorpusFile(filePath, errors);
    }

    for (const filePath of destackCorpusFiles) {
        checkDestackCorpusFile(filePath, errors);
    }

    if (errors.length > 0) {
        console.error("corpus routing check failed:");
        for (const error of errors) {
            console.error(`- ${error}`);
        }
        process.exit(1);
    }

    console.log(
        `corpus routing: ok (${rootCorpusFiles.length} root files, ${rootSectionCount} sections, ${destackCorpusFiles.length} destack files)`
    );
}

main();
