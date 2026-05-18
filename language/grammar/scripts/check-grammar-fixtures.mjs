import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, "..", "..", "..");
const grammarFixtureDirectory = path.join(repoRoot, "language", "test", "fixtures", "grammar");
const specificationFixtureDirectory = path.join(repoRoot, "language", "test", "fixtures", "specification");
const vscodeFixtureDirectory = path.join(grammarFixtureDirectory, "destack");
const destackCorpusDirectory = path.join(repoRoot, "language", "grammar", "destack", "test", "corpus");
const destackGrammarDirectory = path.join(repoRoot, "language", "grammar", "destack", "destack");
const mirCorpusDirectory = path.join(repoRoot, "language", "grammar", "mir", "test", "corpus");
const mirVscodeFixtureDirectory = path.join(grammarFixtureDirectory, "mir");
const textmateAssertionPattern = /^\/\/\s*(?:\^|<-)/m;
const vscodeSpecBlockHeaderPattern = /^\/\/\s+[a-z][\w-]*\s+\/\s+.+$/;
const staleCorpusPatterns = [
    /\bCoverage\b/,
    /\bCommonJS\b/,
    /\bFlow\b/,
    /\bmodule\.exports\b/,
    /\bimport typeof\b/,
    /\bexport =\b/,
    /\b(?:break|continue)\s+:/,
    /\?Y\b/,
    /<A>b/,
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

function fixtureFamily(filePath) {
    const parts = filePath.replace(/\.[^.]+$/, "").split(path.sep);

    if (parts.length >= 2) {
        return `${parts[0]}/${parts[1]}`;
    }

    return parts[0];
}

function checkSpecificationFamilyCoverage(errors) {
    const specificationFamilies = new Set(
        relativeFiles(specificationFixtureDirectory, ".md")
            .filter((filePath) => filePath !== "known-failures.txt")
            .map((filePath) => fixtureFamily(filePath)),
    );
    const grammarFamilies = new Set(
        relativeFiles(vscodeFixtureDirectory, ".txt")
            .map((filePath) => fixtureFamily(filePath)),
    );
    const missingFamilies = [...specificationFamilies]
        .filter((family) => !grammarFamilies.has(family))
        .sort();

    if (missingFamilies.length > 0) {
        errors.push(`${missingFamilies.length} specification fixture families have no shared grammar fixture`);

        for (const family of missingFamilies) {
            errors.push(`${family}: missing shared grammar fixture`);
        }
    }
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

function vscodeSpecBlocks(source) {
    const lines = source.split(/\r?\n/);
    const starts = [];

    for (let index = 0; index < lines.length; index += 1) {
        if (vscodeSpecBlockHeaderPattern.test(lines[index])) {
            starts.push(index);
        }
    }

    return starts.map((start, index) => {
        const end = index + 1 < starts.length ? starts[index + 1] : lines.length;
        const body = lines.slice(start, end).join("\n");
        return { line: start + 1, title: lines[start].slice(3), body };
    });
}

function checkVscodeFixtures(errors) {
    const vscodeFixtures = relativeFiles(vscodeFixtureDirectory, ".txt");
    const missingAssertions = [];
    const missingBlockAssertions = [];

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

        for (const block of vscodeSpecBlocks(source)) {
            if (!textmateAssertionPattern.test(block.body)) {
                missingBlockAssertions.push({
                    filePath,
                    line: block.line,
                    title: block.title,
                });
            }
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

    if (missingBlockAssertions.length > 0) {
        errors.push(`${missingBlockAssertions.length} Destack VSCode fixture blocks have no TextMate assertions`);

        for (const block of missingBlockAssertions.slice(0, 80)) {
            const relativePath = path.relative(repoRoot, block.filePath);
            errors.push(`${relativePath}:${block.line}: missing TextMate assertions for ${block.title}`);
        }

        if (missingBlockAssertions.length > 80) {
            errors.push(`${missingBlockAssertions.length - 80} more Destack VSCode fixture blocks have no TextMate assertions`);
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

function copyDestackFixturesToTemp(files, tempDirectory) {
    return files.map((filePath, index) => {
        const source = fs.readFileSync(filePath, "utf8");
        const tempPath = path.join(tempDirectory, `${index.toString().padStart(4, "0")}.ds`);
        fs.writeFileSync(tempPath, source);
        return { filePath, tempPath };
    });
}

function checkDestackFixtureParse(errors) {
    const files = walkFiles(vscodeFixtureDirectory, (filePath) => filePath.endsWith(".txt"));

    if (files.length === 0) {
        return;
    }

    const tempDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "destack-grammar-fixtures-"));

    try {
        const tempFiles = copyDestackFixturesToTemp(files, tempDirectory);
        const result = spawnSync(
            "bunx",
            [
                "tree-sitter-cli@0.24.4",
                "parse",
                "--quiet",
                "--stat",
                ...tempFiles.map((file) => file.tempPath),
            ],
            {
                cwd: destackGrammarDirectory,
                encoding: "utf8",
            },
        );

        const output = `${result.stdout}\n${result.stderr}`;
        const failedPaths = new Set();
        const failedPathPattern = /^(\S+\.ds)\s+.*\((?:ERROR|MISSING)[^)]*\)/gm;
        let match;

        while ((match = failedPathPattern.exec(output)) !== null) {
            failedPaths.add(match[1]);
        }

        if ((result.status ?? 1) !== 0 && failedPaths.size === 0) {
            errors.push("Destack grammar fixtures failed tree-sitter parse smoke");
            errors.push(output.trim());
            return;
        }

        if (failedPaths.size > 0) {
            errors.push(`${failedPaths.size} Destack grammar fixtures failed tree-sitter parse smoke`);

            for (const file of tempFiles.filter((item) => failedPaths.has(item.tempPath)).slice(0, 40)) {
                errors.push(`${path.relative(repoRoot, file.filePath)}: tree-sitter parse error`);
            }

            if (failedPaths.size > 40) {
                errors.push(`${failedPaths.size - 40} more Destack grammar fixtures failed tree-sitter parse smoke`);
            }
        }
    } finally {
        fs.rmSync(tempDirectory, { recursive: true, force: true });
    }
}

function main() {
    const errors = [];

    checkVscodeFixtures(errors);
    checkSpecificationFamilyCoverage(errors);
    checkDestackCorpus(errors);
    checkMirFixtures(errors);
    checkDestackFixtureParse(errors);

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
