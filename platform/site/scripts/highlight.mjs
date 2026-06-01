import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const treeSitter = join(repositoryDirectory, "node_modules/.bin/tree-sitter");

const languages = {
    destack: loadGrammar(join(repositoryDirectory, "language/grammar/destack"), "destack"),
    mir: loadGrammar(join(repositoryDirectory, "language/grammar/mir"), "mir"),
};

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("ts", typescript);
hljs.registerLanguage("tsx", typescript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("svg", xml);

function loadGrammar(directory, name) {
    const config = JSON.parse(readFileSync(join(directory, "tree-sitter.json"), "utf8"));
    const grammar = config.grammars.find((grammar) => grammar.name === name);
    if (grammar == undefined) {
        throw new Error(`missing ${name} grammar in ${directory}`);
    }

    return {
        directory,
        extension: grammar["file-types"][0],
        queries: (grammar.highlights ?? []).map((query) => resolve(directory, query)),
    };
}

export function highlightCode(source, language) {
    const normalized = normalizeLanguage(language);
    const grammar = grammarFor(normalized);
    if (grammar != undefined) {
        return highlightGrammarSource(source, grammar);
    }

    if (normalized !== "" && hljs.getLanguage(normalized) != undefined) {
        return hljs.highlight(source, { language: normalized }).value;
    }

    return hljs.highlightAuto(source).value;
}

export function highlightDestackFile(file, source) {
    return highlightGrammarFile(file, source, languages.destack);
}

function normalizeLanguage(language) {
    return (language ?? "").trim().split(/\s+/)[0].toLowerCase();
}

function grammarFor(language) {
    if (language === "ds" || language === "destack") {
        return languages.destack;
    }

    if (language === "mir" || language === "dsmir") {
        return languages.mir;
    }

    return undefined;
}

function highlightGrammarSource(source, grammar) {
    const directory = mkdtempSync(join(tmpdir(), "destack-highlight-"));
    const file = join(directory, `source.${grammar.extension}`);

    try {
        writeFileSync(file, source);

        return highlightGrammarFile(file, source, grammar);
    } finally {
        rmSync(directory, { force: true, recursive: true });
    }
}

function highlightGrammarFile(file, source, grammar) {
    const ranges = grammarRanges(file, source, grammar);

    return render(source, ranges);
}

function grammarRanges(file, source, grammar) {
    const ranges = lexicalRanges(source);

    for (const query of grammar.queries) {
        const output = execFileSync(treeSitter, ["query", query, file, "--captures"], {
            cwd: grammar.directory,
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"],
        });
        ranges.push(...parseQueryRanges(output, source));
    }

    return mergeRanges(ranges);
}

function parseQueryRanges(output, source) {
    const starts = lineStarts(source.split("\n"));
    const ranges = [];
    const pattern =
        /capture: \d+ - ([\w.]+), start: \((\d+), (\d+)\), end: \((\d+), (\d+)\), text:/g;

    for (const match of output.matchAll(pattern)) {
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
        /\/\*[\s\S]*?\*\/|\/\/[^\n]*|`(?:\\.|[^`\\])*`|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'|\b\d+(?:\.\d+)?\b|\.\.=?|=>|==|!=|<=|>=|&&|\|\||[{}()[\]<>:;,.|?=!&%*+-]/g;

    for (const match of source.matchAll(pattern)) {
        const value = match[0];
        const start = match.index;
        const kind = lexicalKind(value);
        if (kind == undefined) {
            continue;
        }

        ranges.push({
            start,
            end: start + value.length,
            kind,
            priority: 1,
        });
    }

    return ranges;
}

function diagnosticRanges(source) {
    const ranges = [];
    const lines = source.split("\n");
    const starts = lineStarts(lines);

    for (let index = 0; index < lines.length; index += 1) {
        const squiggle = lines[index].match(/~+/);
        if (squiggle != undefined) {
            const start = starts[index] + squiggle.index;
            ranges.push({
                start,
                end: start + squiggle[0].length,
                kind: "squiggle",
                priority: 3,
            });
        }

        const message = lines[index].match(/error:.*/);
        if (message != undefined) {
            const start = starts[index] + message.index;
            ranges.push({
                start,
                end: start + message[0].length,
                kind: "message",
                priority: 3,
            });
        }
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

    if (/^(?:[{}()[\]<>:;,.|?=!&%*+-]|\.\.=?|=>|==|!=|<=|>=|&&|\|\|)$/.test(value)) {
        return "punct";
    }

    return undefined;
}

function captureKind(capture) {
    if (capture === "keyword" || capture.startsWith("keyword.")) {
        return "keyword";
    }

    if (capture === "type" || capture === "type.builtin") {
        return "type";
    }

    if (
        capture === "constant" ||
        capture === "constant.builtin" ||
        capture === "number" ||
        capture === "boolean"
    ) {
        return "literal";
    }

    if (capture === "string" || capture.startsWith("string.")) {
        return "string";
    }

    if (capture === "comment") {
        return "comment";
    }

    if (
        capture === "function" ||
        capture === "property" ||
        capture === "variable" ||
        capture === "variable.parameter"
    ) {
        return "name";
    }

    if (capture === "operator" || capture.startsWith("punctuation")) {
        return "punct";
    }

    return undefined;
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

function lineStarts(lines) {
    const starts = [];
    let offset = 0;

    for (const line of lines) {
        starts.push(offset);
        offset += line.length + 1;
    }

    return starts;
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
