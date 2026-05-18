import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, "..", "..", "..");
const vscodeFixtureDirectory = path.join(repoRoot, "bridge", "vscode", "src", "tests-grammar");
const destackCorpusDirectory = path.join(repoRoot, "language", "grammar", "destack", "test", "corpus");
const mirCorpusDirectory = path.join(repoRoot, "language", "grammar", "mir", "test", "corpus");
const mirVscodeFixtureDirectory = path.join(repoRoot, "bridge", "vscode", "src", "tests-grammar-mir");
const textmateAssertionPattern = /^\/\/\s*(?:\^|<-)/m;
const staleCorpusPatterns = [
    /\bCoverage\b/,
    /\bCommonJS\b/,
    /\bFlow\b/,
    /\bmodule\.exports\b/,
    /\bimport typeof\b/,
    /\bexport =\b/,
    /\?Y\b/,
    /<A>b/,
    /\bextension\s+.*\s+for\b/,
];

function walkFiles(directory, predicate = () => true) {
    const entries = fs.readdirSync(directory, { withFileTypes: true });
    const files = [];

    for (const entry of entries) {
        const entryPath = path.join(directory, entry.name);

        if (entry.isDirectory()) {
            files.push(...walkFiles(entryPath, predicate));
        } else if (predicate(entryPath)) {
            files.push(entryPath);
        }
    }

    return files.sort();
}

function relativeFiles(directory, extension) {
    return walkFiles(directory, (filePath) => filePath.endsWith(extension))
        .map((filePath) => path.relative(directory, filePath))
        .sort();
}

function sectionHeaders(source) {
    const lines = source.split(/\r?\n/);
    const sections = [];

    for (let index = 0; index < lines.length - 3; index += 1) {
        const opening = lines[index].trim();
        const title = lines[index + 1].trim();
        const directive = lines[index + 2].trim();
        const closing = lines[index + 3].trim();

        if (/^=+$/.test(opening) && title && directive.startsWith(":language(") && /^=+$/.test(closing)) {
            sections.push({ line: index + 2, title, directive });
            index += 3;
        }
    }

    return sections;
}

function checkVscodeFixtures(errors) {
    const vscodeFixtures = relativeFiles(vscodeFixtureDirectory, ".txt");
    const missingAssertions = [];

    if (vscodeFixtures.length === 0) {
        errors.push("Destack VSCode grammar fixtures are empty");
        return;
    }

    for (const fixture of vscodeFixtures) {
        const filePath = path.join(vscodeFixtureDirectory, fixture);
        const source = fs.readFileSync(filePath, "utf8");

        if (!source.startsWith("// SYNTAX TEST ")) {
            errors.push(`${path.relative(repoRoot, filePath)}: missing TextMate syntax test header`);
        }

        if (!textmateAssertionPattern.test(source)) {
            missingAssertions.push(path.relative(repoRoot, filePath));
        }

        for (const pattern of staleCorpusPatterns) {
            if (pattern.test(source)) {
                errors.push(`${path.relative(repoRoot, filePath)}: stale fixture content matched ${pattern}`);
            }
        }
    }

    if (missingAssertions.length > 0) {
        errors.push(`${missingAssertions.length} Destack VSCode grammar fixtures have no TextMate assertions`);

        for (const fixture of missingAssertions.slice(0, 40)) {
            errors.push(`${fixture}: missing TextMate assertions`);
        }

        if (missingAssertions.length > 40) {
            errors.push(`${missingAssertions.length - 40} more Destack VSCode fixtures have no TextMate assertions`);
        }
    }
}

function checkDestackCorpus(errors) {
    const files = walkFiles(destackCorpusDirectory, (filePath) => filePath.endsWith(".txt"));

    if (files.length === 0) {
        errors.push("destack tree-sitter corpus is empty");
        return;
    }

    for (const filePath of files) {
        const source = fs.readFileSync(filePath, "utf8");
        const relativePath = path.relative(repoRoot, filePath);
        const sections = sectionHeaders(source);

        if (sections.length === 0) {
            errors.push(`${relativePath}: no corpus sections found`);
        }

        for (const section of sections) {
            if (section.directive !== ":language(destack)") {
                errors.push(`${relativePath}:${section.line}: expected :language(destack)`);
            }
        }

        for (const pattern of staleCorpusPatterns) {
            if (pattern.test(source)) {
                errors.push(`${relativePath}: stale corpus content matched ${pattern}`);
            }
        }
    }
}

function checkMirFixtures(errors) {
    const corpusFiles = relativeFiles(mirCorpusDirectory, ".txt");
    const vscodeFiles = relativeFiles(mirVscodeFixtureDirectory, ".txt");

    if (corpusFiles.length === 0) {
        errors.push("MIR tree-sitter corpus is empty");
    }

    if (vscodeFiles.length === 0) {
        errors.push("MIR VSCode grammar fixtures are empty");
    }
}

function main() {
    const errors = [];

    checkVscodeFixtures(errors);
    checkDestackCorpus(errors);
    checkMirFixtures(errors);

    if (errors.length > 0) {
        console.error("grammar fixture check failed:");
        for (const error of errors) {
            console.error(`- ${error}`);
        }
        process.exit(1);
    }

    console.log("grammar fixtures: ok");
}

main();
