import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const keywordPath = path.resolve(scriptDir, "..", "..", "dir", "src", "source", "keyword.rs");
const overlayPath = path.resolve(scriptDir, "..", "queries", "destack_overlay.scm");
const forkTreeSitterPath = path.resolve(scriptDir, "..", "destack", "tree-sitter.json");
const forkDestackGrammarPath = path.resolve(scriptDir, "..", "destack", "destack", "grammar.js");
const zedHighlightsPath = path.resolve(
    scriptDir,
    "..",
    "..",
    "..",
    "bridge",
    "zed",
    "languages",
    "destack",
    "highlights.scm"
);

function readText(filePath) {
    if (!fs.existsSync(filePath)) {
        throw new Error(`missing file: ${filePath}`);
    }

    return fs.readFileSync(filePath, "utf8");
}

function rustKeywords(source) {
    const matches = [...source.matchAll(/Keyword::[A-Za-z0-9_]+\s*=>\s*"([^"]+)"/g)];
    return [...new Set(matches.map((match) => match[1]))].sort();
}

function quotedValues(source) {
    const values = [...source.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
    return new Set(values);
}

function main() {
    const keywordSource = readText(keywordPath);
    const overlaySource = readText(overlayPath);
    const forkTreeSitterSource = readText(forkTreeSitterPath);
    const forkDestackGrammarSource = readText(forkDestackGrammarPath);
    const zedHighlightsSource = readText(zedHighlightsPath);

    const keywords = rustKeywords(keywordSource);
    const overlayValues = quotedValues(overlaySource);

    const missingKeywords = keywords.filter((keyword) => !overlayValues.has(keyword));
    if (missingKeywords.length > 0) {
        console.error("overlay is missing keyword captures:");
        for (const keyword of missingKeywords) {
            console.error(`- ${keyword}`);
        }
        process.exit(1);
    }

    const numericPattern = "int(?:\\\\d+)?|uint(?:\\\\d+)?|float(?:\\\\d+)?";
    if (!overlaySource.includes(numericPattern)) {
        console.error("overlay is missing variable-width numeric type matching");
        process.exit(1);
    }

    const forkTreeSitter = JSON.parse(forkTreeSitterSource);
    const hasDestackGrammar = forkTreeSitter.grammars?.some((grammar) => grammar?.name === "destack");
    if (!hasDestackGrammar) {
        console.error("fork tree-sitter.json is missing the destack grammar entry");
        process.exit(1);
    }

    if (!/\bname:\s*['"]destack['"]/.test(forkDestackGrammarSource)) {
        console.error("fork destack grammar.js is not configured as the destack dialect");
        process.exit(1);
    }

    const zedOverlayMarker = "; destack overlay";
    const zedOverlayIndex = zedHighlightsSource.indexOf(zedOverlayMarker);
    if (zedOverlayIndex < 0) {
        console.error("zed highlights file is missing the destack overlay marker");
        process.exit(1);
    }

    const zedOverlaySource = zedHighlightsSource
        .slice(zedOverlayIndex + zedOverlayMarker.length)
        .trim();
    if (zedOverlaySource !== overlaySource.trim()) {
        console.error("zed highlights overlay is out of sync with grammar overlay query");
        process.exit(1);
    }

    console.log("overlay check: ok");
}

main();
