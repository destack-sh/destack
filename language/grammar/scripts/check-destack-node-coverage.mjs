import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const grammarDirectory = path.resolve(scriptDirectory, "..", "destack");
const destackNodeTypesPath = path.resolve(grammarDirectory, "destack", "src", "node-types.json");
const tsxNodeTypesPath = path.resolve(grammarDirectory, "tsx", "src", "node-types.json");
const corpusDirectory = path.resolve(grammarDirectory, "test", "corpus");

function readNamedNodeTypes(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const nodeTypes = JSON.parse(source);
    return new Set(
        nodeTypes
            .filter((entry) => entry?.named === true && typeof entry?.type === "string")
            .map((entry) => entry.type)
    );
}

function escapePattern(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function expectedTrees(filePath) {
    const source = fs.readFileSync(filePath, "utf8");
    const blocks = [];
    const sectionPattern = /\n---\n\n([\s\S]*?)(?=\n=+\n[^\n]+\n=+\n|\s*$)/g;

    for (const match of source.matchAll(sectionPattern)) {
        const block = match[1].trim();
        if (block.length > 0) {
            blocks.push(block);
        }
    }

    return blocks;
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

function main() {
    const destackNamedNodes = readNamedNodeTypes(destackNodeTypesPath);
    const tsxNamedNodes = readNamedNodeTypes(tsxNodeTypesPath);
    const destackOnlyNodes = [...destackNamedNodes]
        .filter((node) => !tsxNamedNodes.has(node))
        .sort();

    const files = corpusFiles(corpusDirectory);
    if (files.length === 0) {
        console.error(`no corpus files found in ${corpusDirectory}`);
        process.exit(1);
    }

    const allExpectedTrees = files.flatMap((filePath) => expectedTrees(filePath));
    const expectedSource = allExpectedTrees.join("\n");
    const missingNodes = destackOnlyNodes.filter((node) => {
        const pattern = new RegExp(`\\(${escapePattern(node)}(?:\\s|\\)|$)`, "m");
        return !pattern.test(expectedSource);
    });

    if (missingNodes.length > 0) {
        console.error("destack node coverage check failed:");
        console.error(`- destack-only node types: ${destackOnlyNodes.length}`);
        console.error(`- covered node types: ${destackOnlyNodes.length - missingNodes.length}`);
        console.error(`- missing node types: ${missingNodes.length}`);
        for (const node of missingNodes) {
            console.error(`  - ${node}`);
        }
        process.exit(1);
    }

    console.log(
        `destack node coverage: ok (${destackOnlyNodes.length}/${destackOnlyNodes.length} destack-only named nodes covered)`
    );
}

main();
