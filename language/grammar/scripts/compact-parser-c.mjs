import fs from "node:fs";
import process from "node:process";

const parserPath = process.argv[2];

if (!parserPath) {
    throw new Error("usage: compact-parser-c.mjs <parser.c>");
}

let source = fs.readFileSync(parserPath, "utf8");

if (source.includes("ts_parse_table[LARGE_STATE_COUNT * SYMBOL_COUNT]")) {
    source = compactGeneratedTables(source);
    fs.writeFileSync(parserPath, source);
    process.exit(0);
}

const symbolCount = readDefine("SYMBOL_COUNT");
const largeStateCount = readDefine("LARGE_STATE_COUNT");
const symbols = readSymbolIndexes(source);
const table = readSparseParseTable(source, symbols, largeStateCount, symbolCount);
const compactTable = writeDenseParseTable(table);

source = replaceParseTable(source, compactTable);
source = source.replace(".parse_table = &ts_parse_table[0][0],", ".parse_table = ts_parse_table,");
source = compactGeneratedTables(source);

fs.writeFileSync(parserPath, source);

function readDefine(name) {
    const pattern = new RegExp(`#define\\s+${name}\\s+(\\d+)`, "u");
    const match = source.match(pattern);

    if (!match) {
        throw new Error(`missing ${name}`);
    }

    return Number(match[1]);
}

function readSymbolIndexes(source) {
    const enumStart = source.indexOf("enum ts_symbol_identifiers");
    const enumOpen = source.indexOf("{", enumStart);
    const enumClose = source.indexOf("\n};", enumOpen);

    if (enumStart < 0 || enumOpen < 0 || enumClose < 0) {
        throw new Error("missing symbol enum");
    }

    const symbols = new Map([["ts_builtin_sym_end", 0]]);
    const enumBody = source.slice(enumOpen + 1, enumClose);

    for (const match of enumBody.matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+)\s*,/gu)) {
        symbols.set(match[1], Number(match[2]));
    }

    return symbols;
}

function readSparseParseTable(source, symbols, largeStateCount, symbolCount) {
    const tableStart = source.indexOf("static const uint16_t ts_parse_table");
    const tableOpen = source.indexOf("{", tableStart);
    const tableClose = source.indexOf("\n};", tableOpen);

    if (tableStart < 0 || tableOpen < 0 || tableClose < 0) {
        throw new Error("missing parse table");
    }

    const table = Array.from({ length: largeStateCount }, () => Array(symbolCount).fill("0"));
    const tableBody = source.slice(tableOpen + 1, tableClose);

    for (const rowMatch of tableBody.matchAll(/\n\s*\[(\d+)\]\s*=\s*\{([\s\S]*?)\n\s*\},/gu)) {
        const state = Number(rowMatch[1]);
        const rowBody = rowMatch[2];

        for (const entryMatch of rowBody.matchAll(/\[([A-Za-z_][A-Za-z0-9_]*)\]\s*=\s*([^,\n]+),/gu)) {
            const symbol = symbols.get(entryMatch[1]);

            if (symbol === undefined || symbol >= symbolCount) {
                throw new Error(`bad parse table symbol: ${entryMatch[1]}`);
            }

            table[state][symbol] = entryMatch[2].trim();
        }
    }

    return table;
}

function writeDenseParseTable(table) {
    const rows = table.map((row) => row.join(","));

    return [
        "static const uint16_t ts_parse_table[LARGE_STATE_COUNT * SYMBOL_COUNT] = {",
        rows.join(",\n"),
        "};",
    ].join("\n");
}

function replaceParseTable(source, compactTable) {
    const tableStart = source.indexOf("static const uint16_t ts_parse_table");
    const tableOpen = source.indexOf("{", tableStart);
    const tableClose = source.indexOf("\n};", tableOpen);

    return source.slice(0, tableStart) + compactTable + source.slice(tableClose + 3);
}

function compactGeneratedTables(source) {
    const names = [
        "ts_lex_modes",
        "ts_symbol_metadata",
        "ts_field_map_slices",
        "ts_field_map_entries",
        "ts_symbol_map",
        "ts_non_terminal_alias_map",
        "ts_alias_sequences",
        "ts_small_parse_table",
        "ts_small_parse_table_map",
        "ts_parse_actions",
    ];

    for (const name of names) {
        source = replaceInitializer(source, name, compactInitializer);
    }

    return source;
}

function replaceInitializer(source, name, transform) {
    const nameIndex = source.indexOf(name);
    const start = source.lastIndexOf("static const", nameIndex);
    const open = source.indexOf("{", nameIndex);
    const close = source.indexOf("\n};", open);

    if (nameIndex < 0 || start < 0 || open < 0 || close < 0) {
        return source;
    }

    const body = source.slice(open + 1, close);
    const compactBody = transform(body);

    return source.slice(0, open + 1) + compactBody + source.slice(close);
}

function compactInitializer(body) {
    return body
        .replace(/^[ \t]+/gmu, "")
        .replace(/[ \t]+$/gmu, "")
        .replace(/[ \t]*=[ \t]*/gu, "=")
        .replace(/[ \t]*,[ \t]*/gu, ",")
        .replace(/[ \t]*\{[ \t]*/gu, "{")
        .replace(/[ \t]*\}[ \t]*/gu, "}");
}
