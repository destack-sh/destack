/// Grammar inputs used for parser compilation and cache invalidation.
type Grammar = { directory: string; extension: string; fingerprint: string; name: string; queries: string[] };
/// Compiler token ranges use UTF-8 byte offsets.
export type SemanticToken = { start: number; end: number; kind: string; modifiers?: number };
/// Rendered highlight ranges use JavaScript character offsets.
type HighlightRange = { start: number; end: number; kind: string; priority: number; modifiers?: number; semantic?: string; href?: string };

import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
    existsSync,
    mkdirSync,
    mkdtempSync,
    readFileSync,
    renameSync,
    rmSync,
    writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptFile = fileURLToPath(import.meta.url);
const repositoryDirectory = resolve(dirname(scriptFile), "../../..");
const treeSitter = join(repositoryDirectory, "node_modules/.bin/tree-sitter");
const cacheDirectory = join(tmpdir(), "destack-highlight-cache");
const highlighterFingerprint = createHash("sha256").update(readFileSync(scriptFile)).digest("hex");
const maximumQueryOutputBytes = 64 * 1024 * 1024;

const languages = {
    bytecode: loadGrammar(
        join(repositoryDirectory, "language/grammar/bytecode"),
        "destack_bytecode",
    ),
    destack: loadGrammar(join(repositoryDirectory, "language/grammar/destack"), "destack"),
    mir: loadGrammar(join(repositoryDirectory, "language/grammar/mir"), "mir"),
};

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("rs", rust);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("ts", typescript);
hljs.registerLanguage("tsx", typescript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("svg", xml);

/// Load one checked-in Tree-sitter grammar.
function loadGrammar(directory: string, name: string) {
    const config: { grammars: { name: string; highlights?: string[]; "file-types": string[]; path?: string }[] } = JSON.parse(readFileSync(join(directory, "tree-sitter.json"), "utf8"));
    const grammar = config.grammars.find((grammar) => grammar.name === name);
    if (grammar == undefined) {
        throw new Error(`missing ${name} grammar in ${directory}`);
    }

    const queries = (grammar.highlights ?? []).map((query) => resolveQuery(directory, query));

    return {
        directory,
        extension: grammar["file-types"][0],
        fingerprint: grammarFingerprint(join(directory, grammar.path ?? ""), queries),
        name,
        queries,
    };
}

/// Hash every grammar input that affects highlighted output.
function grammarFingerprint(directory: string, queries: string[]) {
    const parser = join(directory, "src/parser.c");
    const scanner = join(directory, "src/scanner.c");
    const scannerHeader = join(directory, "src/javascript-scanner.h");
    const files = [parser, scanner, scannerHeader, ...queries].filter(existsSync);
    const hash = createHash("sha256");

    // invalidate cached output whenever the parser or highlighting queries change
    for (const file of files) {
        hash.update(file);
        hash.update(readFileSync(file));
    }

    return hash.digest("hex");
}

/// Resolve one declared highlighting query.
function resolveQuery(directory: string, query: string) {
    const localQuery = resolve(directory, query);
    if (existsSync(localQuery)) {
        return localQuery;
    }

    throw new Error(`missing grammar query: ${query}`);
}

/// Highlight one source string.
export function highlightCode(source: string, language: string): string {
    const normalized = normalizeLanguage(language);
    if (normalized === "text") {
        return escapeHtml(source);
    }

    const grammar = grammarFor(normalized);
    if (grammar != undefined) {
        return highlightGrammarSource(source, grammar);
    }

    if (normalized !== "" && hljs.getLanguage(normalized) != undefined) {
        return hljs.highlight(source, { language: normalized }).value;
    }

    return hljs.highlightAuto(source).value;
}

/// Highlight several independent snippets with one parser invocation.
export function highlightCodeFragments(sources: string[], language: string, semanticTokens: SemanticToken[][] = [], references: Record<string, string> = {}): string[] {
    const normalized = normalizeLanguage(language);
    const grammar = grammarFor(normalized);
    if (grammar == undefined) {
        return sources.map((source) => highlightCode(source, language));
    }

    // return the cached fragment set when its grammar and inputs are unchanged
    const key = createHash("sha256")
        .update(highlighterFingerprint)
        .update(grammar.fingerprint)
        .update(JSON.stringify(sources))
        .update(JSON.stringify(semanticTokens))
        .update(JSON.stringify(references))
        .digest("hex");
    const cached = join(cacheDirectory, `${key}.json`);
    if (existsSync(cached)) {
        return JSON.parse(readFileSync(cached, "utf8"));
    }

    // parse snippets as separate files so incomplete declarations cannot share context
    const directory = mkdtempSync(join(tmpdir(), "destack-highlight-"));
    let highlighted;
    try {
        const files = sources.map((source, index) => {
            const file = join(directory, `${index}.${grammar.extension}`);
            writeFileSync(file, source);

            return file;
        });
        const captures = sources.map((source) => diagnosticRanges(source));
        const library = grammarLibrary(grammar);

        // combine grammar captures so each snippet is parsed once
        const query = join(directory, "highlights.scm");
        writeFileSync(query, grammar.queries.map((path) => readFileSync(path, "utf8")).join("\n"));

        // bound command size and parser execution time for each batch
        for (let offset = 0; offset < files.length; offset += 128) {
            const batch = files.slice(offset, offset + 128);
            const output = execFileSync(treeSitter, [
                "query", "--lib-path", library, "--lang-name", grammar.name,
                query, ...batch, "--captures",
            ], {
                cwd: grammar.directory, encoding: "utf8", timeout: 30000, killSignal: "SIGKILL",
                maxBuffer: maximumQueryOutputBytes, stdio: ["ignore", "pipe", "pipe"],
            });
            const byFile = new Map(files.map((file, index) => [file, index]));
            const outputs: string[][] = sources.map(() => []);
            let index: number | undefined;
            for (const line of output.split("\n")) {
                if (byFile.has(line)) index = byFile.get(line);
                else if (index !== undefined) outputs[index].push(line);
            }
            outputs.forEach((lines, index) => captures[index].push(...parseQueryRanges(lines.join("\n"), sources[index])));
        }

        // apply semantic names and links within each snippet
        highlighted = sources.map((fragment, index) => {
            const ranges = captures[index].map((range) => linkReference(fragment, range, references));
            const checked = semanticRanges(fragment, semanticTokens[index] ?? []);

            return render(fragment, mergeRanges([...ranges, ...checked]));
        });
    } finally {
        rmSync(directory, { force: true, recursive: true });
    }

    // cache the complete batch atomically
    mkdirSync(cacheDirectory, { recursive: true });
    const temporary = `${cached}.${process.pid}.tmp`;
    writeFileSync(temporary, JSON.stringify(highlighted));
    renameSync(temporary, cached);

    return highlighted;
}

/// Link one parsed type name to its unambiguous public declaration.
/// Link one highlighted name to its generated reference page.
function linkReference(source: string, range: HighlightRange, references: Record<string, string>) {
    if (range.kind !== "type") {
        return range;
    }
    const name = source.slice(range.start, range.end);
    const href = references[name];

    return href == undefined ? range : { ...range, href };
}

/// Convert checked UTF-8 token intervals into JavaScript string intervals.
/// Convert compiler semantic tokens into JavaScript string ranges.
function semanticRanges(source: string, tokens: SemanticToken[]): HighlightRange[] {
    return tokens.map((token) => ({
        start: codeUnitOffset(source, token.start),
        end: codeUnitOffset(source, token.end),
        kind: semanticKind(token.kind),
        modifiers: token.modifiers,
        priority: 5,
        semantic: token.kind,
    }));
}

/// Return the syntax color shared by one semantic token kind.
/// Return the stable CSS category for one compiler token kind.
function semanticKind(kind: string) {
    if (["type", "class", "enum", "interface", "newtype_interface", "struct", "type_parameter"].includes(kind)) {
        return "type";
    }

    if (["function", "method"].includes(kind)) {
        return "function";
    }

    if (["property", "enum_member"].includes(kind)) {
        return "property";
    }

    if (kind === "comment") {
        return "comment";
    }

    return "name";
}

/// Convert one UTF-8 byte offset into a JavaScript string offset.
function codeUnitOffset(source: string, byteOffset: number) {
    const bytes = Buffer.from(source);
    if (!Number.isInteger(byteOffset) || byteOffset < 0 || byteOffset > bytes.length) {
        throw new Error(`invalid semantic byte offset ${byteOffset} for ${bytes.length} bytes`);
    }

    return bytes.subarray(0, byteOffset).toString("utf8").length;
}

/// Highlight one Destack source file.
export function highlightDestackFile(file: string, source: string) {
    return highlightGrammarFile(file, source, languages.destack);
}

/// Normalize one Markdown language identifier.
function normalizeLanguage(language: string) {
    return (language ?? "").trim().split(/[:\s]+/)[0].toLowerCase().replace(/^\./, "");
}

/// Select the Tree-sitter grammar for one normalized language.
function grammarFor(language: string) {
    if (language === "ds" || language === "destack" || language === "pattern") {
        return languages.destack;
    }

    if (language === "dsm" || language === "mir") {
        return languages.mir;
    }

    if (language === "dsa" || language === "bytecode") {
        return languages.bytecode;
    }

    return undefined;
}

/// Highlight an in-memory source string with one grammar.
function highlightGrammarSource(source: string, grammar: Grammar) {
    const key = createHash("sha256")
        .update(highlighterFingerprint)
        .update(grammar.fingerprint)
        .update(source)
        .digest("hex");
    const cached = join(cacheDirectory, key);
    if (existsSync(cached)) {
        return readFileSync(cached, "utf8");
    }

    const ranges = grammarSourceRanges(source, grammar);
    const highlighted = render(source, ranges);
    mkdirSync(cacheDirectory, { recursive: true });
    const temporary = `${cached}.${process.pid}.tmp`;
    writeFileSync(temporary, highlighted);
    renameSync(temporary, cached);

    return highlighted;
}

/// Parse one in-memory source and return its highlighting ranges.
/// Parse an in-memory source string and return its capture ranges.
function grammarSourceRanges(source: string, grammar: Grammar) {
    const directory = mkdtempSync(join(tmpdir(), "destack-highlight-"));
    const file = join(directory, `source.${grammar.extension}`);

    try {
        writeFileSync(file, source);

        return grammarRanges(file, source, grammar);
    } finally {
        rmSync(directory, { force: true, recursive: true });
    }
}

/// Highlight one existing source file with one grammar.
function highlightGrammarFile(file: string, source: string, grammar: Grammar) {
    const ranges = grammarRanges(file, source, grammar);

    return render(source, ranges);
}

/// Query one parsed source file for highlighting captures.
function grammarRanges(file: string, source: string, grammar: Grammar) {
    const ranges = diagnosticRanges(source);
    const library = grammarLibrary(grammar);

    for (const query of grammar.queries) {
        const output = execFileSync(treeSitter, [
            "query",
            "--lib-path",
            library,
            "--lang-name",
            grammar.name,
            query,
            file,
            "--captures",
        ], {
            cwd: grammar.directory,
            encoding: "utf8",
            maxBuffer: maximumQueryOutputBytes,
            stdio: ["ignore", "pipe", "pipe"],
        });
        ranges.push(...parseQueryRanges(output, source));
    }

    return mergeRanges(ranges);
}

/// Build or reuse one native Tree-sitter grammar library.
function grammarLibrary(grammar: Grammar) {
    const extension = process.platform === "darwin" ? "dylib" : process.platform === "win32" ? "dll" : "so";
    const library = join(cacheDirectory, `${grammar.name}-${grammar.fingerprint}.${extension}`);
    if (existsSync(library)) {
        return library;
    }

    // compile one parser library for every generated grammar revision
    mkdirSync(cacheDirectory, { recursive: true });
    const temporary = `${library}.${process.pid}.tmp`;
    execFileSync(treeSitter, ["build", "--output", temporary, grammar.directory], {
        cwd: grammar.directory,
        stdio: ["ignore", "pipe", "pipe"],
    });
    renameSync(temporary, library);

    return library;
}

/// Parse Tree-sitter query captures into source ranges.
function parseQueryRanges(output: string, source: string): HighlightRange[] {
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
        ranges.push({ start, end, kind, priority: capturePriority(match[1]) });
    }

    return ranges;
}

/// Return inline diagnostic ranges embedded in example source.
function diagnosticRanges(source: string): HighlightRange[] {
    const ranges = [];
    const lines = source.split("\n");
    const starts = lineStarts(lines);

    for (let index = 0; index < lines.length; index += 1) {
        const squiggle = lines[index].match(/~+/);
        if (squiggle != undefined) {
            const start = starts[index] + squiggle.index!;
            ranges.push({
                start,
                end: start + squiggle[0].length,
                kind: "squiggle",
                priority: 4,
            });
        }

        const message = lines[index].match(/error:.*/);
        if (message != undefined) {
            const start = starts[index] + message.index!;
            ranges.push({
                start,
                end: start + message[0].length,
                kind: "message",
                priority: 4,
            });
        }
    }

    return ranges;
}

/// Return the CSS category for one Tree-sitter capture name.
function captureKind(capture: string) {
    if (capture === "keyword" || capture.startsWith("keyword.")) {
        return "keyword";
    }

    if (capture === "type" || capture.startsWith("type.") || capture === "constructor") {
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

    if (capture === "comment" || capture.startsWith("comment.")) {
        return "comment";
    }

    if (capture === "function" || capture.startsWith("function.")) {
        return "function";
    }

    if (capture === "property" || capture.startsWith("property.")) {
        return "property";
    }

    if (capture === "variable" || capture.startsWith("variable.")) {
        return "name";
    }

    if (capture === "label" || capture.startsWith("label.")) {
        return "name";
    }

    if (capture === "operator" || capture.startsWith("punctuation")) {
        return "punct";
    }

    return undefined;
}

/// Return the precedence of one overlapping capture.
function capturePriority(capture: string) {
    if (
        capture.includes(".definition") ||
        capture.includes(".method") ||
        capture.includes(".builtin") ||
        capture.includes(".parameter")
    ) {
        return 4;
    }

    if (
        capture === "constructor" ||
        capture === "function" ||
        capture === "property" ||
        capture === "type" ||
        capture.startsWith("type.")
    ) {
        return 3;
    }

    return 2;
}

/// Merge compatible adjacent highlighting ranges.
function mergeRanges(ranges: HighlightRange[]) {
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

/// Render highlighted source ranges as escaped HTML.
function render(source: string, ranges: HighlightRange[]) {
    const lines = source.split("\n");
    const starts = lineStarts(lines);
    let html = "";

    for (let index = 0; index < lines.length;) {
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

/// Render one diagnostic marker and message as an inline annotation.
function renderInlineDiagnostic(source: string, ranges: HighlightRange[], lines: string[], starts: number[], startIndex: number, endIndex: number) {
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

/// Return the shared indentation of one inline diagnostic block.
function commonInlineDiagnosticIndent(lines: string[], startIndex: number, endIndex: number) {
    let indent = Infinity;

    for (let index = startIndex; index < endIndex; index += 1) {
        const leading = lines[index].match(/^\s*/)![0].length;
        indent = Math.min(indent, leading);
    }

    return Number.isFinite(indent) ? indent : 0;
}

/// Render one source interval with its active highlighting ranges.
function renderSegment(source: string, ranges: HighlightRange[], start: number, end: number) {
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
        const semantic = range.semantic == undefined ? "" : ` data-s="${range.semantic}"`;
        const modifiers = range.modifiers == undefined ? "" : ` data-m="${range.modifiers}"`;
        const value = `<span data-k="${range.kind}"${semantic}${modifiers}>${escapeHtml(source.slice(rangeStart, rangeEnd))}</span>`;
        html += range.href == undefined
            ? value
            : `<a data-reference href="${escapeHtml(range.href)}">${value}</a>`;
        cursor = rangeEnd;
    }

    html += escapeHtml(source.slice(cursor, end));

    return html;
}

/// Return the source offset of every line.
function lineStarts(lines: string[]) {
    const starts = [];
    let offset = 0;

    for (const line of lines) {
        starts.push(offset);
        offset += line.length + 1;
    }

    return starts;
}

/// Return whether one line belongs to an inline diagnostic.
function isInlineDiagnosticLine(line: string) {
    return /^\s*~+\s*$/.test(line) || /^\s*error:/.test(line);
}

/// Escape one string for insertion into generated HTML.
function escapeHtml(value: string) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}
