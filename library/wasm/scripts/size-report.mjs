#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
    copyFileSync,
    existsSync,
    mkdirSync,
    readFileSync,
    writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
    brotliCompressSync,
    constants as zlibConstants,
    gzipSync,
} from "node:zlib";

const SCRIPT_FILE = fileURLToPath(import.meta.url);
const SCRIPT_DIR = dirname(SCRIPT_FILE);
const PACKAGE_DIR = resolve(SCRIPT_DIR, "..");
const REPO_DIR = resolve(PACKAGE_DIR, "../..");
const GENERATED_WASM_PATH = join(PACKAGE_DIR, "src/generated/index_bg.wasm");
const DEFAULT_SYMBOL_WASM_PATH = join(
    REPO_DIR,
    "target/wasm32-unknown-unknown/release/destack_wasm.wasm",
);

const PRESET_FEATURES = {
    core: "core",
    ide: "ide",
    run: "run",
    full: "full",
};

const MAX_TWIGGY_ITEMS = "50000";

const DEFAULT_OPTIONS = {
    build: true,
    presets: ["core"],
    top: 20,
    retainedTop: 20,
    crateTop: 12,
    monoTop: 8,
    symbols: true,
    symbolWasmPath: null,
    outputDirectory: join(PACKAGE_DIR, "size-reports"),
    baselinePath: null,
    wasmPath: null,
};

const SECTION_NAMES = {
    0: "custom",
    1: "type",
    2: "import",
    3: "function",
    4: "table",
    5: "memory",
    6: "global",
    7: "export",
    8: "start",
    9: "element",
    10: "code",
    11: "data",
    12: "data_count",
};

const textDecoder = new TextDecoder("utf-8");

main();

function main() {
    // parse and validate options
    const options = parseArguments(process.argv.slice(2));
    validateOptions(options);

    // prepare output directory
    const timestamp = formatTimestamp(new Date());
    const reportDirectory = join(options.outputDirectory, timestamp);
    mkdirSync(reportDirectory, { recursive: true });

    // analyze each selected preset or explicit wasm
    const presetReports = [];
    if (options.wasmPath != null) {
        const label = "custom";
        const report = analyzeOneWasm(options.wasmPath, label, options, reportDirectory);
        presetReports.push(report);
    } else {
        for (const preset of options.presets) {
            if (options.build) {
                buildPreset(preset);
            }

            if (options.build && options.symbols) {
                buildSymbolizedPreset(preset);
            }

            const report = analyzeOneWasm(
                GENERATED_WASM_PATH,
                preset,
                options,
                reportDirectory,
            );
            presetReports.push(report);
        }
    }

    // assemble and write report
    const reportPayload = {
        generatedAt: new Date().toISOString(),
        options: {
            build: options.build,
            presets: options.presets,
            top: options.top,
            retainedTop: options.retainedTop,
            crateTop: options.crateTop,
            monoTop: options.monoTop,
            symbols: options.symbols,
            symbolWasmPath: options.symbolWasmPath,
            baselinePath: options.baselinePath,
            wasmPath: options.wasmPath,
        },
        environment: gatherEnvironment(),
        presets: presetReports,
    };

    const reportPath = join(reportDirectory, "report.json");
    writeFileSync(reportPath, JSON.stringify(reportPayload, null, 4));

    // print terminal summary
    printSummary(reportPayload, reportPath, options.baselinePath);
}

function parseArguments(argumentList) {
    // apply defaults
    const options = { ...DEFAULT_OPTIONS };

    // parse each argument token
    for (let index = 0; index < argumentList.length; index += 1) {
        const argument = argumentList[index];

        if (argument === "--build") {
            options.build = true;
            continue;
        }

        if (argument === "--no-build") {
            options.build = false;
            continue;
        }

        if (argument === "--preset") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--preset requires a value");
            }
            options.presets = [value];
            index += 1;
            continue;
        }

        if (argument === "--presets") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--presets requires a comma separated value");
            }
            options.presets = value
                .split(",")
                .map((entry) => entry.trim())
                .filter((entry) => entry.length > 0);
            index += 1;
            continue;
        }

        if (argument === "--top") {
            const value = argumentList[index + 1];
            options.top = parsePositiveIntegerOption(value, "--top");
            index += 1;
            continue;
        }

        if (argument === "--retained-top") {
            const value = argumentList[index + 1];
            options.retainedTop = parsePositiveIntegerOption(value, "--retained-top");
            index += 1;
            continue;
        }

        if (argument === "--crate-top") {
            const value = argumentList[index + 1];
            options.crateTop = parsePositiveIntegerOption(value, "--crate-top");
            index += 1;
            continue;
        }

        if (argument === "--mono-top") {
            const value = argumentList[index + 1];
            options.monoTop = parsePositiveIntegerOption(value, "--mono-top");
            index += 1;
            continue;
        }

        if (argument === "--symbols") {
            options.symbols = true;
            continue;
        }

        if (argument === "--no-symbols") {
            options.symbols = false;
            continue;
        }

        if (argument === "--symbol-wasm") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--symbol-wasm requires a wasm file path");
            }
            options.symbolWasmPath = resolve(process.cwd(), value);
            options.symbols = true;
            index += 1;
            continue;
        }

        if (argument === "--out") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--out requires a directory path");
            }
            options.outputDirectory = resolve(process.cwd(), value);
            index += 1;
            continue;
        }

        if (argument === "--baseline") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--baseline requires a report.json path");
            }
            options.baselinePath = resolve(process.cwd(), value);
            index += 1;
            continue;
        }

        if (argument === "--wasm") {
            const value = argumentList[index + 1];
            if (value == null) {
                fail("--wasm requires a wasm file path");
            }
            options.wasmPath = resolve(process.cwd(), value);
            options.build = false;
            index += 1;
            continue;
        }

        if (argument === "--help" || argument === "-h") {
            printUsage();
            process.exit(0);
        }

        fail(`unknown argument: ${argument}`);
    }

    return options;
}

function parsePositiveIntegerOption(value, flag) {
    if (value == null) {
        fail(`${flag} requires a numeric value`);
    }

    const parsed = Number.parseInt(value, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
        fail(`${flag} must be a positive integer`);
    }

    return parsed;
}

function validateOptions(options) {
    // validate preset names
    if (options.wasmPath == null) {
        if (options.presets.length === 0) {
            fail("at least one preset is required when --wasm is not set");
        }

        for (const preset of options.presets) {
            if (!(preset in PRESET_FEATURES)) {
                fail(`unsupported preset: ${preset}`);
            }
        }
    }

    // guard ambiguous no-build multi preset runs
    if (!options.build && options.wasmPath == null && options.presets.length > 1) {
        fail("--no-build supports only one preset unless --wasm is provided");
    }
}

function printUsage() {
    const usage = [
        "Usage: bun ./scripts/size-report.mjs [options]",
        "",
        "Options:",
        "  --build                 build each selected preset before analysis",
        "  --no-build              analyze existing generated wasm only",
        "  --preset <name>         single preset: core, ide, run, full",
        "  --presets <csv>         multiple presets: core,ide,run,full",
        "  --top <count>           shallow top offenders to print, default 20",
        "  --retained-top <count>  retained top offenders to print, default 20",
        "  --crate-top <count>     crate buckets to print, default 12",
        "  --mono-top <count>      monomorphization groups to print, default 8",
        "  --symbols               enable symbolized attribution analysis, default on",
        "  --no-symbols            skip symbolized attribution analysis",
        "  --symbol-wasm <path>    symbolized wasm path for attribution analysis",
        "  --out <directory>       output directory root, default ./size-reports",
        "  --baseline <report>     compare size metrics against prior report.json",
        "  --wasm <path>           analyze explicit wasm file and skip builds",
        "  --help                  show this message",
    ];

    console.log(usage.join("\n"));
}

function buildPreset(preset) {
    // compute feature and command arguments
    const feature = PRESET_FEATURES[preset];
    const buildArguments = [
        "build",
        "--target",
        "web",
        "--release",
        "--out-dir",
        "./src/generated",
        "--out-name",
        "index",
        "--",
        "--no-default-features",
        "--features",
        feature,
    ];

    // run wasm pack with size focused flags
    const environment = {
        CARGO_PROFILE_RELEASE_OPT_LEVEL: "z",
        CARGO_PROFILE_RELEASE_LTO: "thin",
        CARGO_PROFILE_RELEASE_CODEGEN_UNITS: "1",
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: "-C target-feature=+tail-call",
    };

    console.log(`\n[build] preset=${preset} feature=${feature}`);
    runCommand("wasm-pack", buildArguments, {
        cwd: PACKAGE_DIR,
        env: environment,
        stdio: "inherit",
    });
}

function buildSymbolizedPreset(preset) {
    // build a non-stripped wasm for symbol level attribution
    const feature = PRESET_FEATURES[preset];
    const buildArguments = [
        "build",
        "-p",
        "destack_wasm",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
        "--no-default-features",
        "--features",
        feature,
    ];

    const environment = {
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:
            "-C target-feature=+tail-call -C strip=none",
    };

    console.log(`\n[build-symbols] preset=${preset} feature=${feature}`);
    runCommand("cargo", buildArguments, {
        cwd: REPO_DIR,
        env: environment,
        stdio: "inherit",
    });
}

function analyzeOneWasm(wasmPath, preset, options, reportDirectory) {
    // read wasm bytes and copy artifact for reproducibility
    const absoluteWasmPath = resolve(wasmPath);
    const wasmBuffer = readFileSync(absoluteWasmPath);
    const copiedPath = join(reportDirectory, `${preset}.wasm`);
    copyFileSync(absoluteWasmPath, copiedPath);

    // calculate compression sizes
    const gzipBuffer = gzipSync(wasmBuffer, { level: 9 });
    const brotliBuffer = brotliCompressSync(wasmBuffer, {
        params: {
            [zlibConstants.BROTLI_PARAM_QUALITY]: 11,
        },
    });

    // parse wasm sections
    const sectionBreakdown = parseSectionBreakdown(wasmBuffer);

    // gather shallow and retained top offenders on shipped artifact
    const shallowTop = runTwiggyTop(absoluteWasmPath, options.top, false);
    const retainedTop = runTwiggyTop(absoluteWasmPath, options.retainedTop, true);

    // resolve symbolized wasm path when requested
    const symbolWasmPath = resolveSymbolWasmPath(options);

    // gather attribution and data segment details
    const attributionPath = symbolWasmPath ?? absoluteWasmPath;
    const fullShallow = runTwiggyTop(attributionPath, MAX_TWIGGY_ITEMS, false);
    const crateAttribution = fullShallow.available
        ? summarizeCrateAttribution(
            fullShallow.entries,
            options.crateTop,
            Boolean(symbolWasmPath),
        )
        : {
            available: false,
            totalBytes: 0,
            rows: [],
            error: fullShallow.error,
        };
    const dataSegments = fullShallow.available
        ? summarizeDataSegments(fullShallow.entries, 8)
        : [];

    // gather generic monomorphization offenders
    const monomorphizations = symbolWasmPath == null
        ? {
            available: false,
            entries: [],
            error: "symbolized wasm unavailable, pass --symbol-wasm or run with --build",
        }
        : runTwiggyMonos(symbolWasmPath, options.monoTop);

    return {
        preset,
        wasmPath: absoluteWasmPath,
        symbolWasmPath,
        copiedWasmPath: copiedPath,
        rawBytes: wasmBuffer.length,
        gzipBytes: gzipBuffer.length,
        brotliBytes: brotliBuffer.length,
        sections: sectionBreakdown,
        shallowTop,
        retainedTop,
        crateAttribution,
        dataSegments,
        monomorphizations,
    };
}

function resolveSymbolWasmPath(options) {
    // skip symbol analysis when disabled
    if (!options.symbols) {
        return null;
    }

    // prefer explicit symbol path when provided
    if (options.symbolWasmPath != null) {
        return existsSync(options.symbolWasmPath) ? options.symbolWasmPath : null;
    }

    // fall back to the default cargo target path
    return existsSync(DEFAULT_SYMBOL_WASM_PATH) ? DEFAULT_SYMBOL_WASM_PATH : null;
}

function parseSectionBreakdown(wasmBuffer) {
    // validate wasm header bytes
    const bytes = new Uint8Array(wasmBuffer);
    if (bytes.length < 8) {
        fail("wasm file is too small to contain a valid header");
    }

    const magic = [0x00, 0x61, 0x73, 0x6d];
    const version = [0x01, 0x00, 0x00, 0x00];
    for (let index = 0; index < magic.length; index += 1) {
        if (bytes[index] !== magic[index]) {
            fail("wasm header magic does not match expected format");
        }
    }
    for (let index = 0; index < version.length; index += 1) {
        if (bytes[index + 4] !== version[index]) {
            fail("wasm header version does not match expected format");
        }
    }

    // scan sections and aggregate totals
    const aggregate = new Map();
    let offset = 8;
    while (offset < bytes.length) {
        const sectionId = bytes[offset];
        offset += 1;

        const payloadLengthInfo = readVarUint32(bytes, offset, bytes.length);
        const payloadLength = payloadLengthInfo.value;
        const payloadLengthByteCount = payloadLengthInfo.byteCount;
        const payloadOffset = payloadLengthInfo.nextOffset;
        const payloadEnd = payloadOffset + payloadLength;

        if (payloadEnd > bytes.length) {
            fail(`section payload extends past file size at offset ${offset}`);
        }

        const totalBytes = 1 + payloadLengthByteCount + payloadLength;
        const sectionName = resolveSectionName(bytes, sectionId, payloadOffset, payloadEnd);
        const existing = aggregate.get(sectionName) ?? {
            name: sectionName,
            bytes: 0,
            count: 0,
        };
        existing.bytes += totalBytes;
        existing.count += 1;
        aggregate.set(sectionName, existing);

        offset = payloadEnd;
    }

    // finalize sorted rows with percentages
    const totalBytes = wasmBuffer.length;
    return Array.from(aggregate.values())
        .sort((left, right) => right.bytes - left.bytes)
        .map((row) => ({
            name: row.name,
            bytes: row.bytes,
            count: row.count,
            percent: ((row.bytes / totalBytes) * 100).toFixed(2),
        }));
}

function resolveSectionName(bytes, sectionId, payloadOffset, payloadEnd) {
    // return known core section names
    if (sectionId !== 0) {
        return SECTION_NAMES[sectionId] ?? `unknown:${sectionId}`;
    }

    // decode custom section names as custom:name
    try {
        const nameInfo = readName(bytes, payloadOffset, payloadEnd);
        return `custom:${nameInfo.name}`;
    } catch {
        return "custom";
    }
}

function readName(bytes, offset, limit) {
    // read utf8 byte length
    const lengthInfo = readVarUint32(bytes, offset, limit);
    const nameLength = lengthInfo.value;
    const start = lengthInfo.nextOffset;
    const end = start + nameLength;

    // validate bounds and decode bytes
    if (end > limit) {
        throw new Error("custom section name exceeds payload bounds");
    }

    const view = bytes.subarray(start, end);
    const name = textDecoder.decode(view);
    return { name, nextOffset: end };
}

function readVarUint32(bytes, offset, limit) {
    // decode leb128 unsigned integer
    let value = 0;
    let shift = 0;
    let cursor = offset;
    while (cursor < limit) {
        const byte = bytes[cursor];
        cursor += 1;

        value += (byte & 0x7f) * 2 ** shift;
        if ((byte & 0x80) === 0) {
            return {
                value,
                nextOffset: cursor,
                byteCount: cursor - offset,
            };
        }

        shift += 7;
        if (shift > 35) {
            break;
        }
    }

    fail(`invalid leb128 integer at offset ${offset}`);
}

function runTwiggyTop(wasmPath, count, retained) {
    // skip gracefully when twiggy is unavailable
    if (!commandExists("twiggy")) {
        return {
            available: false,
            entries: [],
            error: "twiggy is not installed",
        };
    }

    // run twiggy top and parse json output
    const commandArguments = [
        "top",
        "-n",
        String(count),
        "-f",
        "json",
        ...(retained ? ["--retained"] : []),
        wasmPath,
    ];

    const result = runCommand("twiggy", commandArguments, {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });

    if (result.status !== 0) {
        const stderr = (result.stderr ?? "").trim();
        return {
            available: false,
            entries: [],
            error: stderr.length > 0 ? stderr : "twiggy top failed",
        };
    }

    try {
        const entries = JSON.parse(result.stdout ?? "[]");
        return {
            available: true,
            entries,
        };
    } catch {
        return {
            available: false,
            entries: [],
            error: "twiggy returned invalid json",
        };
    }
}

function runTwiggyMonos(wasmPath, genericCount) {
    // skip gracefully when twiggy is unavailable
    if (!commandExists("twiggy")) {
        return {
            available: false,
            entries: [],
            error: "twiggy is not installed",
        };
    }

    // run twiggy monos and parse json output
    const result = runCommand(
        "twiggy",
        ["monos", "-f", "json", "-m", String(genericCount), "-n", "3", wasmPath],
        {
            cwd: REPO_DIR,
            stdio: "pipe",
            allowFailure: true,
        },
    );

    if (result.status !== 0) {
        const stderr = (result.stderr ?? "").trim();
        return {
            available: false,
            entries: [],
            error: stderr.length > 0 ? stderr : "twiggy monos failed",
        };
    }

    try {
        const rows = JSON.parse(result.stdout ?? "[]");
        const entries = rows
            .filter((row) => !isSummaryRow(row.generic ?? ""))
            .map((row) => ({
                generic: row.generic,
                bloatBytes: Number(row.approximate_monomorphization_bloat_bytes ?? 0),
                bloatPercent: Number(row.approximate_monomorphization_bloat_percent ?? 0),
                totalBytes: Number(row.total_size ?? 0),
                totalPercent: Number(row.total_size_percent ?? 0),
            }));

        if (entries.length === 0) {
            return {
                available: false,
                entries: [],
                error: "no generic symbols found in this wasm",
            };
        }

        return {
            available: true,
            entries,
        };
    } catch {
        return {
            available: false,
            entries: [],
            error: "twiggy monos returned invalid json",
        };
    }
}

function summarizeCrateAttribution(entries, maxItems, usedSymbols) {
    // return unavailable when entries are missing
    if (!Array.isArray(entries) || entries.length === 0) {
        return {
            available: false,
            totalBytes: 0,
            rows: [],
            error: "no twiggy entries for attribution",
        };
    }

    // aggregate shallow bytes by crate-like symbol prefix
    const totals = new Map();
    let totalBytes = 0;

    for (const item of entries) {
        const name = String(item.name ?? "");
        const shallowBytes = Number(item.shallow_size ?? 0);

        if (shallowBytes <= 0 || isSummaryRow(name)) {
            continue;
        }

        if (name.startsWith("custom section '") || name === '"function names" subsection') {
            continue;
        }

        const bucket = bucketNameForSymbol(name);
        if (bucket == null) {
            continue;
        }

        totals.set(bucket, (totals.get(bucket) ?? 0) + shallowBytes);
        totalBytes += shallowBytes;
    }

    if (totalBytes === 0) {
        return {
            available: false,
            totalBytes: 0,
            rows: [],
            error: usedSymbols
                ? "no demangled crate symbols in symbolized wasm"
                : "no demangled crate symbols, enable --symbols for attribution",
        };
    }

    const rows = Array.from(totals.entries())
        .map(([name, bytes]) => ({
            name,
            bytes,
            percent: (100 * bytes) / totalBytes,
        }))
        .sort((left, right) => right.bytes - left.bytes)
        .slice(0, maxItems);

    return {
        available: true,
        totalBytes,
        rows,
    };
}

function summarizeDataSegments(entries, maxItems) {
    // aggregate data segment items separately
    const rows = [];

    for (const item of entries) {
        const name = String(item.name ?? "");
        if (!name.startsWith("data segment")) {
            continue;
        }

        rows.push({
            name,
            bytes: Number(item.shallow_size ?? 0),
            percent: Number(item.shallow_size_percent ?? 0),
        });
    }

    rows.sort((left, right) => right.bytes - left.bytes);
    return rows.slice(0, maxItems);
}

function bucketNameForSymbol(symbolName) {
    // extract crate-like prefix from rust demangled item names
    const crateMatch = symbolName.match(/([a-zA-Z_][a-zA-Z0-9_]*)\[[0-9a-f]{8,}\]/);
    if (crateMatch != null) {
        return crateMatch[1];
    }

    return null;
}

function isSummaryRow(name) {
    return name.startsWith("... and ") || name.startsWith("Σ [");
}

function gatherEnvironment() {
    // collect git and tool versions for reproducibility
    const commit = runCommand("git", ["rev-parse", "HEAD"], {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });
    const branch = runCommand("git", ["rev-parse", "--abbrev-ref", "HEAD"], {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });
    const status = runCommand("git", ["status", "--short"], {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });

    const wasmPackVersion = runCommand("wasm-pack", ["--version"], {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });
    const twiggyVersion = runCommand("twiggy", ["--version"], {
        cwd: REPO_DIR,
        stdio: "pipe",
        allowFailure: true,
    });

    return {
        nodeVersion: process.version,
        wasmPackVersion: (wasmPackVersion.stdout ?? "").trim() || null,
        twiggyVersion: (twiggyVersion.stdout ?? "").trim() || null,
        gitCommit: (commit.stdout ?? "").trim() || null,
        gitBranch: (branch.stdout ?? "").trim() || null,
        gitDirty: (status.stdout ?? "").trim().length > 0,
    };
}

function printSummary(reportPayload, reportPath, baselinePath) {
    // print size summary rows
    console.log("\n[size summary]");
    console.log("preset\traw\tgzip\tbrotli");
    for (const preset of reportPayload.presets) {
        console.log(
            `${preset.preset}\t${formatBytes(preset.rawBytes)}\t${formatBytes(preset.gzipBytes)}\t${formatBytes(preset.brotliBytes)}`,
        );
    }

    // print preset details
    for (const preset of reportPayload.presets) {
        console.log(`\n[${preset.preset}] largest sections`);
        for (const section of preset.sections.slice(0, 6)) {
            console.log(
                `- ${section.name}: ${formatBytes(section.bytes)} (${section.percent}%) x${section.count}`,
            );
        }

        if (preset.shallowTop.available) {
            console.log(`[${preset.preset}] shallow top`);
            for (const item of preset.shallowTop.entries.slice(0, 10)) {
                const size = formatBytes(item.shallow_size ?? 0);
                const percent = Number(item.shallow_size_percent ?? 0).toFixed(2);
                console.log(`- ${item.name}: ${size} (${percent}%)`);
            }
        } else {
            console.log(`[${preset.preset}] shallow top unavailable: ${preset.shallowTop.error}`);
        }

        if (preset.retainedTop.available) {
            console.log(`[${preset.preset}] retained top`);
            for (const item of preset.retainedTop.entries.slice(0, 10)) {
                const size = formatBytes(item.shallow_size ?? 0);
                const percent = Number(item.shallow_size_percent ?? 0).toFixed(2);
                console.log(`- ${item.name}: ${size} (${percent}%)`);
            }
        } else {
            console.log(`[${preset.preset}] retained top unavailable: ${preset.retainedTop.error}`);
        }

        if (preset.crateAttribution.available) {
            const sourceLabel = preset.symbolWasmPath == null
                ? ""
                : ` from ${basename(preset.symbolWasmPath)}`;
            console.log(
                `[${preset.preset}] crate attribution${sourceLabel} (analyzable ${formatBytes(preset.crateAttribution.totalBytes)})`,
            );
            for (const row of preset.crateAttribution.rows) {
                console.log(
                    `- ${row.name}: ${formatBytes(row.bytes)} (${row.percent.toFixed(2)}%)`,
                );
            }
        } else {
            console.log(
                `[${preset.preset}] crate attribution unavailable: ${preset.crateAttribution.error}`,
            );
        }

        if (preset.dataSegments.length > 0) {
            console.log(`[${preset.preset}] largest data segments`);
            for (const row of preset.dataSegments.slice(0, 6)) {
                console.log(
                    `- ${row.name}: ${formatBytes(row.bytes)} (${row.percent.toFixed(2)}%)`,
                );
            }
        }

        if (preset.monomorphizations.available) {
            const sourceLabel = preset.symbolWasmPath == null
                ? ""
                : ` from ${basename(preset.symbolWasmPath)}`;
            console.log(`[${preset.preset}] monomorphization bloat${sourceLabel}`);
            for (const row of preset.monomorphizations.entries.slice(0, 8)) {
                console.log(
                    `- ${row.generic}: bloat ${formatBytes(row.bloatBytes)} (${row.bloatPercent.toFixed(2)}%), total ${formatBytes(row.totalBytes)}`,
                );
            }
        } else {
            console.log(
                `[${preset.preset}] monomorphization bloat unavailable: ${preset.monomorphizations.error}`,
            );
        }
    }

    // print optional baseline comparison
    if (baselinePath != null) {
        printBaselineComparison(reportPayload, baselinePath);
    }

    console.log(`\n[report] ${reportPath}`);
}

function printBaselineComparison(reportPayload, baselinePath) {
    // read baseline report file
    let baseline;
    try {
        const baselineRaw = readFileSync(baselinePath, "utf8");
        baseline = JSON.parse(baselineRaw);
    } catch {
        console.log(`\n[baseline] unable to read baseline report: ${baselinePath}`);
        return;
    }

    // compare current and baseline size metrics per preset
    const baselinePresets = baseline.presets ?? baseline.profiles ?? [];
    const baselineByPreset = new Map();
    for (const preset of baselinePresets) {
        const name = preset.preset ?? preset.profile;
        baselineByPreset.set(name, preset);
    }

    console.log("\n[baseline delta]");
    console.log("preset\traw\tgzip\tbrotli");
    for (const preset of reportPayload.presets) {
        const previous = baselineByPreset.get(preset.preset);
        if (previous == null) {
            console.log(`${preset.preset}\tn/a\tn/a\tn/a`);
            continue;
        }

        const rawDelta = formatDelta(preset.rawBytes - previous.rawBytes);
        const gzipDelta = formatDelta(preset.gzipBytes - previous.gzipBytes);
        const brotliDelta = formatDelta(preset.brotliBytes - previous.brotliBytes);
        console.log(`${preset.preset}\t${rawDelta}\t${gzipDelta}\t${brotliDelta}`);
    }
}

function formatBytes(bytes) {
    if (!Number.isFinite(bytes)) {
        return "n/a";
    }

    if (bytes < 1024) {
        return `${bytes} B`;
    }

    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(2)} KiB`;
    }

    return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function formatDelta(bytes) {
    const sign = bytes >= 0 ? "+" : "-";
    return `${sign}${formatBytes(Math.abs(bytes))}`;
}

function formatTimestamp(date) {
    const year = String(date.getFullYear());
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const seconds = String(date.getSeconds()).padStart(2, "0");

    return `${year}${month}${day}-${hours}${minutes}${seconds}`;
}

function commandExists(command) {
    const result = spawnSync(command, ["--version"], {
        cwd: REPO_DIR,
        stdio: "ignore",
    });
    return result.status === 0;
}

function runCommand(command, argumentList, options) {
    const result = spawnSync(command, argumentList, {
        cwd: options.cwd,
        env: {
            ...process.env,
            ...(options.env ?? {}),
        },
        stdio: options.stdio,
        encoding: "utf8",
        maxBuffer: 256 * 1024 * 1024,
    });

    const allowFailure = options.allowFailure ?? false;
    if (!allowFailure && result.status !== 0) {
        const rendered = [command, ...argumentList].join(" ");
        const stderr = (result.stderr ?? "").trim();
        fail(`command failed: ${rendered}${stderr.length > 0 ? `\n${stderr}` : ""}`);
    }

    return {
        status: result.status ?? 1,
        stdout: result.stdout ?? "",
        stderr: result.stderr ?? "",
    };
}

function fail(message) {
    console.error(`error: ${message}`);
    process.exit(1);
}
