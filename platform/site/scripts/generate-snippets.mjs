import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const grammarDirectory = join(repositoryDirectory, "language/grammar/destack");
const treeSitter = join(repositoryDirectory, "node_modules/.bin/tree-sitter");
const highlightQuery = join(grammarDirectory, "queries/highlights.scm");
const snippetDirectory = join(siteDirectory, "src/snippets");
const generatedSnippetFile = join(siteDirectory, "src/generated/snippets.ts");

const keywords = new Set([
    "async",
    "await",
    "case",
    "catch",
    "class",
    "comptime",
    "const",
    "declare",
    "else",
    "enum",
    "export",
    "extension",
    "finally",
    "function",
    "global",
    "if",
    "implements",
    "import",
    "interface",
    "let",
    "match",
    "module",
    "new",
    "newtype",
    "readonly",
    "return",
    "satisfies",
    "shared",
    "static",
    "struct",
    "throw",
    "try",
    "type",
    "using",
    "from",
    "with",
    "where",
]);

const builtinTypes = new Set([
    "any",
    "boolean",
    "float",
    "float32",
    "float64",
    "int",
    "int32",
    "int64",
    "never",
    "number",
    "string",
    "uint",
    "uint3",
    "uint8",
    "uint32",
    "uint64",
    "unknown",
    "usize",
    "void",
]);

const snippetOrder = [
    "types/primitives",
    "types/intervals",
    "types/newtypes",
    "types/extensions",
    "types/enums",
    "types/structs",
    "types/tuples",
    "types/slices",
    "types/arrays",
    "types/readonly",
    "types/generics",
    "types/constraints",
    "types/associated",
    "types/reflection",
    "types/tagged-unions",
    "types/static",
    "types/any",
    "expressions/blocks",
    "expressions/patterns",
    "expressions/let-else",
    "expressions/match",
    "expressions/is",
    "expressions/loops",
    "expressions/using",
    "expressions/ranges",
    "expressions/overloads",
    "expressions/errors",
    "expressions/tsx",
    "expressions/decorators",
    "expressions/module",
    "expressions/comptime",
    "expressions/macros",
    "expressions/captures",
    "memory/space",
    "memory/ownership",
    "memory/borrowing",
    "memory/lifetimes",
    "memory/drop",
    "memory/dispose",
    "memory/unsafe",
    "memory/polymorphism",
    "runtime/modules",
    "runtime/conditions",
    "runtime/import-meta",
    "runtime/data-modules",
    "runtime/policy",
    "runtime/test",
    "runtime/fuzz",
    "runtime/bench",
    "runtime/simulation",
];

const entries = snippetOrder.map((name) => {
    const file = join(snippetDirectory, `${name}.ds`);
    const source = readFileSync(file, "utf8");
    const ranges = highlightRanges(file, source);
    const html = render(source, ranges);

    return [name, html];
});

mkdirSync(dirname(generatedSnippetFile), { recursive: true });
writeFileSync(
    generatedSnippetFile,
    `export const snippets = ${JSON.stringify(Object.fromEntries(entries), null, 4)} as const;\n`,
);

const snippetFiles = listSnippetFiles(snippetDirectory, snippetDirectory);
const expectedFiles = new Set(snippetOrder.map((name) => `${name}.ds`));

for (const file of snippetFiles) {
    if (!expectedFiles.has(file)) {
        throw new Error(`unexpected snippet file: ${file}`);
    }
}

for (const file of expectedFiles) {
    if (!snippetFiles.has(file)) {
        throw new Error(`missing snippet file: ${file}`);
    }
}

function highlightRanges(file, source) {
    const ranges = lexicalRanges(source);
    const queryOutput = execFileSync(treeSitter, ["query", highlightQuery, file], {
        cwd: grammarDirectory,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
    });

    const queryRanges = parseQueryRanges(queryOutput, source);

    return mergeRanges([...ranges, ...queryRanges]);
}

function parseQueryRanges(queryOutput, source) {
    const lines = source.split("\n");
    const starts = [];
    let offset = 0;

    for (const line of lines) {
        starts.push(offset);
        offset += line.length + 1;
    }

    const ranges = [];
    const pattern =
        /capture: \d+ - ([\w.]+), start: \((\d+), (\d+)\), end: \((\d+), (\d+)\), text:/g;

    for (const match of queryOutput.matchAll(pattern)) {
        const kind = captureKind(match[1]);
        if (kind == undefined) {
            continue;
        }

        const start = starts[Number(match[2])] + Number(match[3]);
        const end = starts[Number(match[4])] + Number(match[5]);
        ranges.push({ start, end, kind, priority: 2 });
    }

    return ranges;
}

function lexicalRanges(source) {
    const ranges = diagnosticRanges(source);
    const pattern =
        /\/\*[\s\S]*?\*\/|\/\/[^\n]*|`(?:\\.|[^`\\])*`|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'|\b\d+(?:\.\d+)?\b|\.\.=?|=>|==|!=|<=|>=|&&|\|\||\b[A-Za-z_][A-Za-z0-9_]*\b|[{}()[\]<>:;,.|?=!&%*+-]/g;

    for (const match of source.matchAll(pattern)) {
        const value = match[0];
        const start = match.index;
        const end = start + value.length;
        const kind = lexicalKind(value);
        if (kind == undefined) {
            continue;
        }

        ranges.push({ start, end, kind, priority: 1 });
    }

    return ranges;
}

function diagnosticRanges(source) {
    const ranges = [];
    const lines = source.split("\n");
    let offset = 0;

    for (const line of lines) {
        const squiggle = line.match(/~+/);
        if (squiggle != undefined) {
            const start = offset + squiggle.index;
            ranges.push({
                start,
                end: start + squiggle[0].length,
                kind: "squiggle",
                priority: 3,
            });
        }

        const message = line.match(/error:.*/);
        if (message != undefined) {
            const start = offset + message.index;
            ranges.push({
                start,
                end: start + message[0].length,
                kind: "message",
                priority: 3,
            });
        }

        offset += line.length + 1;
    }

    return ranges;
}

function lexicalKind(value) {
    if (value.startsWith("//") || value.startsWith("/*")) {
        return "comment";
    }

    if (value.startsWith("`") || value.startsWith("\"") || value.startsWith("'")) {
        return "string";
    }

    if (/^\d/.test(value)) {
        return "literal";
    }

    if (keywords.has(value)) {
        return "keyword";
    }

    if (builtinTypes.has(value)) {
        return "type";
    }

    if (/^(?:[{}()[\]<>:;,.|?=!&%*+-]|\.\.=?|=>|==|!=|<=|>=|&&|\|\|)$/.test(value)) {
        return "punct";
    }

    return undefined;
}

function captureKind(capture) {
    if (capture == "keyword") {
        return "keyword";
    }

    if (capture == "type" || capture == "type.builtin") {
        return "type";
    }

    if (capture == "variable.parameter") {
        return "name";
    }

    if (capture.startsWith("punctuation")) {
        return "punct";
    }

    return undefined;
}

function listSnippetFiles(rootDirectory, directory) {
    const files = new Set();

    for (const entry of readdirSync(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            for (const file of listSnippetFiles(rootDirectory, path)) {
                files.add(file);
            }
        } else if (entry.isFile() && entry.name.endsWith(".ds")) {
            const file = relative(rootDirectory, path).replaceAll("\\", "/");
            files.add(file);
        } else {
            const file = relative(rootDirectory, path).replaceAll("\\", "/");
            throw new Error(`unexpected snippet file: ${file}`);
        }
    }

    return files;
}

function mergeRanges(ranges) {
    const sorted = ranges
        .filter((range) => range.end > range.start)
        .sort((a, b) => a.start - b.start || b.priority - a.priority || b.end - a.end);
    const merged = [];

    for (const range of sorted) {
        const previous = merged.at(-1);
        if (previous != undefined && previous.end > range.start) {
            continue;
        }

        merged.push(range);
    }

    return merged;
}

function render(source, ranges) {
    const lines = source.split("\n");
    const starts = lineStarts(lines);
    let html = "";

    for (let index = 0; index < lines.length; ) {
        const line = lines[index];
        const lineStart = starts[index];
        const lineEnd = lineStart + line.length;

        if (isInlineDiagnosticLine(line)) {
            let nextIndex = index + 1;

            while (nextIndex < lines.length && isInlineDiagnosticLine(lines[nextIndex])) {
                nextIndex += 1;
            }

            html += renderInlineDiagnostic(source, ranges, lines, starts, index, nextIndex);

            if (nextIndex < lines.length) {
                html += "\n";
            }

            index = nextIndex;
        } else {
            html += renderSegment(source, ranges, lineStart, lineEnd);

            if (index + 1 < lines.length) {
                html += "\n";
            }

            index += 1;
        }
    }

    return html;
}

function lineStarts(lines) {
    const starts = [];
    let offset = 0;

    for (const line of lines) {
        starts.push(offset);
        offset += line.length + 1;
    }

    return starts;
}

function renderInlineDiagnostic(source, ranges, lines, starts, startIndex, endIndex) {
    const indent = commonInlineDiagnosticIndent(lines, startIndex, endIndex);
    const parts = [];

    for (let index = startIndex; index < endIndex; index += 1) {
        const start = starts[index] + indent;
        const end = starts[index] + lines[index].length;
        parts.push(renderSegment(source, ranges, start, end));
    }

    const padding = escapeHtml(" ".repeat(indent));
    const body = parts.join("\n");

    return `${padding}<span data-k="inline-diagnostic-shadow"><span data-k="inline-diagnostic-box">${body}</span></span>`;
}

function commonInlineDiagnosticIndent(lines, startIndex, endIndex) {
    let indent = Infinity;

    for (let index = startIndex; index < endIndex; index += 1) {
        const leading = lines[index].match(/^\s*/)[0].length;
        indent = Math.min(indent, leading);
    }

    return Number.isFinite(indent) ? indent : 0;
}

function renderSegment(source, ranges, start, end) {
    let html = "";
    let cursor = start;

    for (const range of ranges) {
        if (range.end <= start) {
            continue;
        }

        if (range.start >= end) {
            break;
        }

        const rangeStart = Math.max(range.start, start);
        const rangeEnd = Math.min(range.end, end);
        html += escapeHtml(source.slice(cursor, rangeStart));
        html += `<span data-k="${range.kind}">${escapeHtml(source.slice(rangeStart, rangeEnd))}</span>`;
        cursor = rangeEnd;
    }

    html += escapeHtml(source.slice(cursor, end));

    return html;
}

function isInlineDiagnosticLine(line) {
    return /^\s*~+\s*$/.test(line) || /^\s*error:/.test(line);
}

function escapeHtml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}
