import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const corpusDirectory = path.resolve(scriptDirectory, "..", "destack", "test", "corpus");

function readLines(filePath) {
    return fs.readFileSync(filePath, "utf8").split(/\r?\n/);
}

function corpusFiles(directory) {
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(directory, entry.name);

        if (entry.isDirectory()) {
            files.push(...corpusFiles(entryPath));
        } else if (entry.name.endsWith(".txt")) {
            files.push(entryPath);
        }
    }

    return files.sort();
}

function checkCorpusFile(filePath, errors) {
    const lines = readLines(filePath);
    let sectionCount = 0;

    for (let index = 0; index < lines.length - 3; index += 1) {
        const opening = lines[index].trim();
        const title = lines[index + 1]?.trim() ?? "";
        const directive = lines[index + 2]?.trim() ?? "";
        const closing = lines[index + 3]?.trim() ?? "";

        if (!/^=+$/.test(opening) || title === "") {
            continue;
        }

        if (!directive.startsWith(":language(")) {
            if (/^=+$/.test(directive)) {
                errors.push(`${filePath}:${index + 1}: missing :language(destack) after section title`);
            }
            continue;
        }

        if (directive !== ":language(destack)") {
            errors.push(`${filePath}:${index + 3}: expected :language(destack)`);
        }

        if (!/^=+$/.test(closing)) {
            errors.push(`${filePath}:${index + 1}: malformed section header block`);
            continue;
        }

        sectionCount += 1;
        index += 3;
    }

    return sectionCount;
}

function main() {
    const errors = [];
    const files = corpusFiles(corpusDirectory);

    if (files.length === 0) {
        errors.push(`no corpus files found in ${corpusDirectory}`);
    }

    let sectionCount = 0;
    for (const filePath of files) {
        sectionCount += checkCorpusFile(filePath, errors);
    }

    if (errors.length > 0) {
        console.error("corpus routing check failed:");
        for (const error of errors) {
            console.error(`- ${error}`);
        }
        process.exit(1);
    }

    console.log(`corpus routing: ok (${files.length} files, ${sectionCount} sections)`);
}

main();
