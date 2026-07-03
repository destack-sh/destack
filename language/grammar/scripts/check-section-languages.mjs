import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const sectionsDirectory = path.resolve(scriptDirectory, "..", "destack", "test", "corpus");

function readLines(filePath) {
    return fs.readFileSync(filePath, "utf8").split(/\r?\n/);
}

function sectionFiles(directory) {
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(directory, entry.name);

        if (entry.isDirectory()) {
            files.push(...sectionFiles(entryPath));
        } else if (entry.name.endsWith(".txt")) {
            files.push(entryPath);
        }
    }

    return files.sort();
}

function checkSectionFile(filePath, errors) {
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
    const files = sectionFiles(sectionsDirectory);

    if (files.length === 0) {
        errors.push(`no section files found in ${sectionsDirectory}`);
    }

    let sectionCount = 0;
    for (const filePath of files) {
        sectionCount += checkSectionFile(filePath, errors);
    }

    if (errors.length > 0) {
        console.error("section language check failed:");
        for (const error of errors) {
            console.error(`- ${error}`);
        }
        process.exit(1);
    }

    console.log(`section languages: ok (${files.length} files, ${sectionCount} sections)`);
}

main();
