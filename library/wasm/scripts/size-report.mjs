#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import {
    copyFileSync,
    mkdirSync,
    readFileSync,
    writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
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

const PROFILE_FEATURES = {
    core: "profile-wasm-core",
    ide: "profile-wasm-ide",
    run: "profile-wasm-run",
    full: "profile-wasm-full",
};

const DEFAULT_OPTIONS = {
    build: true,
    profiles: ["core"],
    top: 20,
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

    // gather context and prepare report output paths
    const timestamp = formatTimestamp(new Date());
    const reportDirectory = join(options.outputDirectory, timestamp);
    mkdirSync(reportDirectory, { recursive: true });

    // analyze each requested profile or explicit wasm path
    const profileReports = [];
    if (options.wasmPath != null) {
        const label = "custom";
        const report = analyzeOneWasm(options.wasmPath, label, options.top, reportDirectory);
        profileReports.push(report);
    } else {
        for (const profile of options.profiles) {
            if (options.build) {
                buildProfile(profile);
            }

            const report = analyzeOneWasm(GENERATED_WASM_PATH, profile, options.top, reportDirectory);
            profileReports.push(report);
        }
    }

    // assemble and write the report payload
    const reportPayload = {
        generatedAt: new Date().toISOString(),
        options: {
            build: options.build,
            profiles: options.profiles,
            top: options.top,
            baselinePath: options.baselinePath,
            wasmPath: options.wasmPath,
        },
        environment: gatherEnvironment(),
        profiles: profileReports,
    };
    const reportPath = join(reportDirectory, "report.json");
    writeFileSync(reportPath, JSON.stringify(reportPayload, null, 4));

    // print a concise terminal summary
    printSummary(reportPayload, reportPath, options.baselinePath);
}

function parseArguments(argumentsList) {
    // apply defaults
    const options = { ...DEFAULT_OPTIONS };

    // parse each argument token
    for (let index = 0; index < argumentsList.length; index += 1) {
        const argument = argumentsList[index];

        if (argument === "--build") {
            options.build = true;
            continue;
        }

        if (argument === "--no-build") {
            options.build = false;
            continue;
        }

        if (argument === "--profile") {
            const value = argumentsList[index + 1];
            if (value == null) {
                fail("--profile requires a value");
            }
            options.profiles = [value];
            index += 1;
            continue;
        }

        if (argument === "--profiles") {
            const value = argumentsList[index + 1];
            if (value == null) {
                fail("--profiles requires a comma separated value");
            }
            options.profiles = value
                .split(",")
                .map((entry) => entry.trim())
                .filter((entry) => entry.length > 0);
            index += 1;
            continue;
        }

        if (argument === "--top") {
            const value = argumentsList[index + 1];
            if (value == null) {
                fail("--top requires a numeric value");
            }
            const parsed = Number.parseInt(value, 10);
            if (!Number.isFinite(parsed) || parsed <= 0) {
                fail("--top must be a positive integer");
            }
            options.top = parsed;
            index += 1;
            continue;
        }

        if (argument === "--out") {
            const value = argumentsList[index + 1];
            if (value == null) {
                fail("--out requires a directory path");
            }
            options.outputDirectory = resolve(process.cwd(), value);
            index += 1;
            continue;
        }

        if (argument === "--baseline") {
            const value = argumentsList[index + 1];
            if (value == null) {
                fail("--baseline requires a report.json path");
            }
            options.baselinePath = resolve(process.cwd(), value);
            index += 1;
            continue;
        }

        if (argument === "--wasm") {
            const value = argumentsList[index + 1];
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

function validateOptions(options) {
    // validate profile names
    if (options.wasmPath == null) {
        if (options.profiles.length === 0) {
            fail("at least one profile is required when --wasm is not set");
        }

        for (const profile of options.profiles) {
            if (!(profile in PROFILE_FEATURES)) {
                fail(`unsupported profile: ${profile}`);
            }
        }
    }

    // guard ambiguous no build multi profile runs
    if (!options.build && options.wasmPath == null && options.profiles.length > 1) {
        fail("--no-build supports only one profile unless --wasm is provided");
    }
}

function printUsage() {
    const usage = [
        "Usage: bun ./scripts/size-report.mjs [options]",
        "",
        "Options:",
        "  --build                 build each selected profile before analysis",
        "  --no-build              analyze existing generated wasm only",
        "  --profile <name>        single profile: core, ide, run, full",
        "  --profiles <csv>        multiple profiles: core,ide,run,full",
        "  --top <count>           twiggy top entries to capture, default 20",
        "  --out <directory>       output directory root, default ./size-reports",
        "  --baseline <report>     compare size metrics against prior report.json",
        "  --wasm <path>           analyze explicit wasm file and skip builds",
        "  --help                  show this message",
    ];

    console.log(usage.join("\n"));
}

function buildProfile(profile) {
    // compute profile feature and build command
    const feature = PROFILE_FEATURES[profile];
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
    console.log(`\n[build] profile=${profile} feature=${feature}`);
    runCommand("wasm-pack", buildArguments, {
        cwd: PACKAGE_DIR,
        env: environment,
        stdio: "inherit",
    });
}

function analyzeOneWasm(wasmPath, profile, topCount, reportDirectory) {
    // read wasm bytes and copy artifact for reproducibility
    const absoluteWasmPath = resolve(wasmPath);
    const wasmBuffer = readFileSync(absoluteWasmPath);
    const copiedPath = join(reportDirectory, `${profile}.wasm`);
    copyFileSync(absoluteWasmPath, copiedPath);

    // calculate compression sizes
    const gzipBuffer = gzipSync(wasmBuffer, { level: 9 });
    const brotliBuffer = brotliCompressSync(wasmBuffer, {
        params: {
            [zlibConstants.BROTLI_PARAM_QUALITY]: 11,
        },
    });

    // parse section level sizes from the wasm binary
    const sectionBreakdown = parseSectionBreakdown(wasmBuffer);

    // gather top offenders with twiggy
    const twiggyTop = runTwiggyTop(absoluteWasmPath, topCount);

    return {
        profile,
        wasmPath: absoluteWasmPath,
        copiedWasmPath: copiedPath,
        rawBytes: wasmBuffer.length,
        gzipBytes: gzipBuffer.length,
        brotliBytes: brotliBuffer.length,
        sections: sectionBreakdown,
        top: twiggyTop,
    };
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

    // scan sections and aggregate by logical section name
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
        const existing = aggregate.get(sectionName) ?? { name: sectionName, bytes: 0, count: 0 };
        existing.bytes += totalBytes;
        existing.count += 1;
        aggregate.set(sectionName, existing);

        offset = payloadEnd;
    }

    // finalize sorted section rows with percentages
    const totalBytes = wasmBuffer.length;
    const rows = Array.from(aggregate.values())
        .sort((left, right) => right.bytes - left.bytes)
        .map((row) => ({
            name: row.name,
            bytes: row.bytes,
            count: row.count,
            percent: ((row.bytes / totalBytes) * 100).toFixed(2),
        }));

    return rows;
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
    // read the utf 8 byte length
    const lengthInfo = readVarUint32(bytes, offset, limit);
    const nameLength = lengthInfo.value;
    const start = lengthInfo.nextOffset;
    const end = start + nameLength;

    // validate bounds and decode the bytes
    if (end > limit) {
        throw new Error("custom section name exceeds payload bounds");
    }

    const view = bytes.subarray(start, end);
    const name = textDecoder.decode(view);
    return { name, nextOffset: end };
}

function readVarUint32(bytes, offset, limit) {
    // decode a leb128 unsigned integer
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

function runTwiggyTop(wasmPath, topCount) {
    // skip gracefully when twiggy is unavailable
    if (!commandExists("twiggy")) {
        return {
            available: false,
            entries: [],
            error: "twiggy is not installed",
        };
    }

    // run twiggy and parse json output
    const result = runCommand("twiggy", ["top", "-n", String(topCount), "-f", "json", wasmPath], {
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
    // print size summary row for each profile
    console.log("\n[size summary]");
    console.log("profile\traw\tgzip\tbrotli");
    for (const profile of reportPayload.profiles) {
        console.log(
            `${profile.profile}\t${formatBytes(profile.rawBytes)}\t${formatBytes(profile.gzipBytes)}\t${formatBytes(profile.brotliBytes)}`,
        );
    }

    // print largest sections and top offenders per profile
    for (const profile of reportPayload.profiles) {
        console.log(`\n[${profile.profile}] largest sections`);
        for (const section of profile.sections.slice(0, 6)) {
            console.log(`- ${section.name}: ${formatBytes(section.bytes)} (${section.percent}%) x${section.count}`);
        }

        if (profile.top.available) {
            console.log(`[${profile.profile}] twiggy top`);
            for (const item of profile.top.entries.slice(0, 10)) {
                const size = formatBytes(item.shallow_size ?? 0);
                const percent = Number(item.shallow_size_percent ?? 0).toFixed(2);
                console.log(`- ${item.name}: ${size} (${percent}%)`);
            }
        } else {
            console.log(`[${profile.profile}] twiggy top unavailable: ${profile.top.error}`);
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

    // compare current and baseline size metrics per profile
    const baselineByProfile = new Map();
    for (const profile of baseline.profiles ?? []) {
        baselineByProfile.set(profile.profile, profile);
    }

    console.log("\n[baseline delta]");
    console.log("profile\traw\tgzip\tbrotli");
    for (const profile of reportPayload.profiles) {
        const previous = baselineByProfile.get(profile.profile);
        if (previous == null) {
            console.log(`${profile.profile}\tn/a\tn/a\tn/a`);
            continue;
        }

        const rawDelta = formatDelta(profile.rawBytes - previous.rawBytes);
        const gzipDelta = formatDelta(profile.gzipBytes - previous.gzipBytes);
        const brotliDelta = formatDelta(profile.brotliBytes - previous.brotliBytes);
        console.log(`${profile.profile}\t${rawDelta}\t${gzipDelta}\t${brotliDelta}`);
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

function runCommand(command, argumentsList, options) {
    const result = spawnSync(command, argumentsList, {
        cwd: options.cwd,
        env: {
            ...process.env,
            ...(options.env ?? {}),
        },
        encoding: "utf8",
        stdio: options.stdio,
    });

    if (result.error != null) {
        fail(`failed to run ${command}: ${result.error.message}`);
    }

    if (!options.allowFailure && result.status !== 0) {
        const stderr = (result.stderr ?? "").trim();
        if (stderr.length > 0) {
            fail(`${command} failed: ${stderr}`);
        }
        fail(`${command} failed with status ${result.status}`);
    }

    return {
        status: result.status ?? 1,
        stdout: result.stdout,
        stderr: result.stderr,
    };
}

function fail(message) {
    console.error(`[size-report] ${message}`);
    process.exit(1);
}
