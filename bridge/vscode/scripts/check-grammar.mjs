import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..", "..");

const keywordPath = path.join(repoRoot, "language", "ast", "src", "tree", "keyword.rs");
const tokenPath = path.join(repoRoot, "language", "ast", "src", "token", "print.rs");
const grammarPath = path.join(repoRoot, "bridge", "vscode", "destack.tmLanguage.json");

function readFileOrThrow(filePath) {
    if (!fs.existsSync(filePath)) {
        throw new Error(`missing file: ${filePath}`);
    }
    return fs.readFileSync(filePath, "utf8");
}

function collectKeywordStrings(source) {
    const matches = [...source.matchAll(/Keyword::[A-Za-z0-9_]+\s*=>\s*"([^"]+)"/g)];
    const values = matches.map((match) => match[1]);
    return [...new Set(values)].sort();
}

function collectTokenStrings(source) {
    const matches = [...source.matchAll(/TokenType::[A-Za-z0-9_]+\s*=>\s*"([^"]*)"/g)];
    const values = matches.map((match) => match[1]);
    return [...new Set(values)];
}

function isOperatorToken(value) {
    if (!value) return false;
    if (value == "." || value == "," || value == ":" || value == ";" || value == "@" || value == "#") {
        return false;
    }
    if (value == "(" || value == ")" || value == "[" || value == "]" || value == "{" || value == "}") {
        return false;
    }
    if (value == "\n") return false;

    if (value == ".." || value == "...") return true;
    return /[+\-*/%=&|^!<>?~]/.test(value);
}

function escapeRegexLiteral(value) {
    return value.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&");
}

function extractWordTokens(pattern) {
    let buffer = "";
    let in_class = false;
    let escaped = false;

    for (let i = 0; i < pattern.length; i += 1) {
        const ch = pattern[i];

        if (escaped) {
            buffer += " ";
            escaped = false;
            continue;
        }

        if (ch == "\\") {
            escaped = true;
            buffer += " ";
            continue;
        }

        if (in_class) {
            if (ch == "]") {
                in_class = false;
            }
            continue;
        }

        if (ch == "[") {
            in_class = true;
            continue;
        }

        if (/[A-Za-z0-9_]/.test(ch)) {
            buffer += ch;
        } else {
            buffer += " ";
        }
    }

    return buffer.split(/\s+/).filter(Boolean);
}

function collectGrammarPatterns(grammar, keys) {
    const patterns = [];
    for (const key of keys) {
        const repo = grammar.repository?.[key];
        if (!repo || !Array.isArray(repo.patterns)) {
            continue;
        }
        for (const entry of repo.patterns) {
            if (typeof entry.match == "string") {
                patterns.push(entry.match);
            }
        }
    }
    return patterns;
}

function main() {
    const keywordSource = readFileOrThrow(keywordPath);
    const tokenSource = readFileOrThrow(tokenPath);
    const grammarSource = readFileOrThrow(grammarPath);
    const grammar = JSON.parse(grammarSource);

    const keywords = collectKeywordStrings(keywordSource);
    const operatorTokens = collectTokenStrings(tokenSource)
        .filter(isOperatorToken)
        .sort();

    const keywordPatterns = collectGrammarPatterns(grammar, ["keyword", "operator", "literal-type"]);
    const keywordTokens = new Set(
        keywordPatterns.flatMap((pattern) => extractWordTokens(pattern)),
    );

    const missingKeywords = keywords.filter((kw) => !keywordTokens.has(kw));

    const operatorPatterns = collectGrammarPatterns(grammar, ["operator"]);
    const missingOperators = operatorTokens.filter((op) => {
        const escaped = escapeRegexLiteral(op);
        return !operatorPatterns.some((pattern) => pattern.includes(escaped) || pattern.includes(op));
    });

    if (missingKeywords.length == 0 && missingOperators.length == 0) {
        console.log("grammar check: ok");
        return;
    }

    if (missingKeywords.length > 0) {
        console.error("missing keyword highlights:");
        for (const kw of missingKeywords) {
            console.error(`- ${kw}`);
        }
    }

    if (missingOperators.length > 0) {
        console.error("missing operator highlights:");
        for (const op of missingOperators) {
            console.error(`- ${op}`);
        }
    }

    process.exit(1);
}

main();
