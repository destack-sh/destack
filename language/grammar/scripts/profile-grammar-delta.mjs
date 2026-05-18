import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const grammarRoot = path.resolve(scriptDir, "..");
const forkRoot = path.resolve(grammarRoot, "destack");
const generatedPaths = [
    "src/grammar.json",
    "src/node-types.json",
    "src/parser.c",
    "src/tree_sitter/array.h",
    "src/tree_sitter/parser.h",
];

function parseArgs(argv) {
    const defaults = {
        target: "destack",
        baseline: "tsx",
        top: 30,
        json: "",
        maxParserSizeRatio: 2,
        maxStateRatio: 1.5,
    };

    const options = { ...defaults };

    for (let i = 0; i < argv.length; i += 1) {
        const token = argv[i];

        if (token === "--target" && argv[i + 1]) {
            options.target = argv[i + 1];
            i += 1;
            continue;
        }

        if (token === "--baseline" && argv[i + 1]) {
            options.baseline = argv[i + 1];
            i += 1;
            continue;
        }

        if (token === "--top" && argv[i + 1]) {
            options.top = Number.parseInt(argv[i + 1], 10);
            i += 1;
            continue;
        }

        if (token === "--json" && argv[i + 1]) {
            options.json = argv[i + 1];
            i += 1;
            continue;
        }

        if (token === "--max-parser-size-ratio" && argv[i + 1]) {
            options.maxParserSizeRatio = Number.parseFloat(argv[i + 1]);
            i += 1;
            continue;
        }

        if (token === "--max-state-ratio" && argv[i + 1]) {
            options.maxStateRatio = Number.parseFloat(argv[i + 1]);
            i += 1;
            continue;
        }

        if (token === "--help" || token === "-h") {
            printHelp();
            process.exit(0);
        }
    }

    if (!Number.isFinite(options.top) || options.top <= 0) {
        throw new Error(`invalid --top value: ${options.top}`);
    }

    if (!Number.isFinite(options.maxParserSizeRatio) || options.maxParserSizeRatio <= 0) {
        throw new Error(`invalid --max-parser-size-ratio value: ${options.maxParserSizeRatio}`);
    }

    if (!Number.isFinite(options.maxStateRatio) || options.maxStateRatio <= 0) {
        throw new Error(`invalid --max-state-ratio value: ${options.maxStateRatio}`);
    }

    return options;
}

function printHelp() {
    console.log("Profile tree-sitter rule state deltas between two dialects.");
    console.log("");
    console.log("Usage:");
    console.log("  node language/grammar/scripts/profile-grammar-delta.mjs [options]");
    console.log("");
    console.log("Options:");
    console.log("  --target <name>      target grammar dialect (default: destack)");
    console.log("  --baseline <name>    baseline grammar dialect (default: tsx)");
    console.log("  --top <n>            number of top rows to print (default: 30)");
    console.log("  --json <path>        optional output path for machine-readable JSON");
    console.log("  --max-parser-size-ratio <n>");
    console.log("                       fail if target parser.c exceeds this baseline multiple");
    console.log("  --max-state-ratio <n>");
    console.log("                       fail if target state count exceeds this baseline multiple");
}

function resolveDialectDir(name) {
    const candidates = [
        path.resolve(forkRoot, name),
        path.resolve(grammarRoot, name),
    ];

    for (const candidate of candidates) {
        if (fs.existsSync(path.resolve(candidate, "grammar.js"))) {
            return candidate;
        }
    }

    throw new Error(`could not resolve grammar directory for dialect: ${name}`);
}

function runReport(dialect, directory) {
    const snapshot = snapshotGeneratedFiles(directory);

    try {
        const start = process.hrtime.bigint();
        const result = spawnSync(
            "bunx",
            ["tree-sitter-cli@0.24.4", "generate", "--report-states-for-rule", "-"],
            {
                cwd: directory,
                encoding: "utf8",
                stdio: ["ignore", "pipe", "pipe"],
            },
        );
        const end = process.hrtime.bigint();

        if (result.error) {
            throw result.error;
        }

        if (result.status !== 0) {
            throw new Error(
                `tree-sitter generate failed for ${dialect} (${directory})\n${result.stdout}\n${result.stderr}`,
            );
        }

        const combined = `${result.stdout}\n${result.stderr}`;
        const rules = parseRuleStates(combined);
        const parserMetrics = parseParserMetrics(path.resolve(directory, "src", "parser.c"));
        const seconds = Number(end - start) / 1_000_000_000;

        return {
            dialect,
            directory,
            seconds,
            rules,
            parserMetrics,
        };
    } finally {
        restoreGeneratedFiles(snapshot);
    }
}

function snapshotGeneratedFiles(directory) {
    return generatedPaths.map((relativePath) => {
        const filePath = path.resolve(directory, relativePath);
        const content = fs.existsSync(filePath) ? fs.readFileSync(filePath) : null;

        return { filePath, content };
    });
}

function restoreGeneratedFiles(snapshot) {
    for (const entry of snapshot) {
        if (entry.content === null) {
            fs.rmSync(entry.filePath, { force: true });
        } else {
            fs.mkdirSync(path.dirname(entry.filePath), { recursive: true });
            fs.writeFileSync(entry.filePath, entry.content);
        }
    }
}

function parseRuleStates(source) {
    const statesByRule = new Map();
    const lines = source.split(/\r?\n/u);

    for (const rawLine of lines) {
        const line = rawLine.trimEnd();
        const match = line.match(/^(.+?)\s+\t([0-9]+)$/u);
        if (!match) {
            continue;
        }

        const rule = match[1].trim();
        const states = Number.parseInt(match[2], 10);
        statesByRule.set(rule, states);
    }

    return statesByRule;
}

function parseParserMetrics(parserPath) {
    if (!fs.existsSync(parserPath)) {
        throw new Error(`missing parser file: ${parserPath}`);
    }

    const source = fs.readFileSync(parserPath, "utf8");
    const metrics = {};

    for (const key of ["STATE_COUNT", "LARGE_STATE_COUNT", "SYMBOL_COUNT", "TOKEN_COUNT"]) {
        const regex = new RegExp(`#define\\s+${key}\\s+([0-9]+)`, "u");
        const match = source.match(regex);
        if (!match) {
            throw new Error(`missing ${key} in ${parserPath}`);
        }
        metrics[key] = Number.parseInt(match[1], 10);
    }

    metrics.PARSER_C_SIZE = fs.statSync(parserPath).size;
    return metrics;
}

function computeDelta(targetRules, baselineRules) {
    const allRuleNames = new Set([...targetRules.keys(), ...baselineRules.keys()]);
    const rows = [];

    for (const rule of allRuleNames) {
        const target = targetRules.get(rule) ?? 0;
        const baseline = baselineRules.get(rule) ?? 0;
        rows.push({
            rule,
            target,
            baseline,
            delta: target - baseline,
            targetOnly: baseline === 0 && target > 0,
        });
    }

    const sortedByDelta = [...rows].sort((left, right) => right.delta - left.delta);
    const sortedTargetOnly = sortedByDelta.filter((row) => row.targetOnly);
    const positiveDeltaRows = sortedByDelta.filter((row) => row.delta > 0);

    const totalPositiveDelta = positiveDeltaRows.reduce((sum, row) => sum + row.delta, 0);
    const targetOnlyDelta = sortedTargetOnly.reduce((sum, row) => sum + row.delta, 0);

    return {
        rows,
        sortedByDelta,
        sortedTargetOnly,
        totalPositiveDelta,
        targetOnlyDelta,
    };
}

function clusterDelta(rows) {
    const expressionRules = new Set([
        "assignment_expression",
        "primary_expression",
        "arrow_function",
        "_augmented_assignment_lhs",
        "binary_expression",
        "call_expression",
        "member_expression",
        "subscript_expression",
        "ternary_expression",
        "update_expression",
        "as_expression",
        "satisfies_expression",
        "instantiation_expression",
        "non_null_expression",
        "try_propagation_expression",
        "if_expression",
    ]);

    const memberDeclarationRules = new Set([
        "public_field_definition",
        "method_definition",
        "method_signature",
        "class_declaration",
        "function_signature",
        "function_declaration",
        "_parameter_name",
        "_property_name",
        "class",
        "interface_declaration",
        "constructor_type",
        "function_expression",
        "generator_function",
        "generator_function_declaration",
    ]);

    const expressionDelta = rows
        .filter((row) => expressionRules.has(row.rule))
        .reduce((sum, row) => sum + Math.max(0, row.delta), 0);

    const memberDeclarationDelta = rows
        .filter((row) => memberDeclarationRules.has(row.rule))
        .reduce((sum, row) => sum + Math.max(0, row.delta), 0);

    return {
        expressionDelta,
        memberDeclarationDelta,
    };
}

function ratio(numerator, denominator) {
    if (denominator === 0) {
        return "n/a";
    }

    return `${(numerator / denominator).toFixed(2)}x`;
}

function printMetricSummary(targetReport, baselineReport) {
    const keys = [
        "STATE_COUNT",
        "LARGE_STATE_COUNT",
        "SYMBOL_COUNT",
        "TOKEN_COUNT",
        "PARSER_C_SIZE",
    ];

    console.log("## parser summary");
    for (const key of keys) {
        const target = targetReport.parserMetrics[key];
        const baseline = baselineReport.parserMetrics[key];
        const keyLabel = key.toLowerCase();
        console.log(
            `${keyLabel}: target=${target} baseline=${baseline} ratio=${ratio(target, baseline)}`,
        );
    }
    console.log(
        `generate_time_seconds: target=${targetReport.seconds.toFixed(2)} baseline=${baselineReport.seconds.toFixed(2)}`,
    );
}

function printRuleTable(title, rows, top) {
    console.log(`## ${title}`);
    const slice = rows.slice(0, top);

    for (const row of slice) {
        console.log(
            `${row.rule}: target=${row.target} baseline=${row.baseline} delta=${row.delta}`,
        );
    }
}

function maybeWriteJson(pathValue, payload) {
    if (!pathValue) {
        return;
    }

    const outputPath = path.resolve(process.cwd(), pathValue);
    const outputDir = path.dirname(outputPath);
    fs.mkdirSync(outputDir, { recursive: true });
    fs.writeFileSync(outputPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
    console.log(`json report written: ${outputPath}`);
}

function assertBudget(targetReport, baselineReport, options) {
    const parserSizeRatio =
        targetReport.parserMetrics.PARSER_C_SIZE / baselineReport.parserMetrics.PARSER_C_SIZE;
    const stateRatio =
        targetReport.parserMetrics.STATE_COUNT / baselineReport.parserMetrics.STATE_COUNT;
    const failures = [];

    if (parserSizeRatio > options.maxParserSizeRatio) {
        failures.push(
            `parser_c_size ratio ${parserSizeRatio.toFixed(2)}x exceeds ${options.maxParserSizeRatio.toFixed(2)}x`,
        );
    }

    if (stateRatio > options.maxStateRatio) {
        failures.push(
            `state_count ratio ${stateRatio.toFixed(2)}x exceeds ${options.maxStateRatio.toFixed(2)}x`,
        );
    }

    if (failures.length > 0) {
        throw new Error(`grammar budget exceeded: ${failures.join("; ")}`);
    }
}

function main() {
    const options = parseArgs(process.argv.slice(2));
    const targetDir = resolveDialectDir(options.target);
    const baselineDir = resolveDialectDir(options.baseline);

    const targetReport = runReport(options.target, targetDir);
    const baselineReport = runReport(options.baseline, baselineDir);
    const delta = computeDelta(targetReport.rules, baselineReport.rules);
    const clusters = clusterDelta(delta.rows);

    const top20Delta = delta.sortedByDelta
        .slice(0, 20)
        .filter((row) => row.delta > 0)
        .reduce((sum, row) => sum + row.delta, 0);

    console.log(`# grammar delta profile: ${options.target} vs ${options.baseline}`);
    console.log(
        `target_rules=${targetReport.rules.size} baseline_rules=${baselineReport.rules.size}`,
    );
    console.log(
        `positive_delta_total=${delta.totalPositiveDelta} target_only_delta_total=${delta.targetOnlyDelta}`,
    );
    console.log(`top20_positive_delta_total=${top20Delta}`);
    console.log(`expression_cluster_delta=${clusters.expressionDelta}`);
    console.log(`member_declaration_cluster_delta=${clusters.memberDeclarationDelta}`);

    printMetricSummary(targetReport, baselineReport);
    printRuleTable("top positive rule deltas", delta.sortedByDelta.filter((row) => row.delta > 0), options.top);
    printRuleTable("target-only rules", delta.sortedTargetOnly, options.top);

    maybeWriteJson(options.json, {
        options,
        target: {
            dialect: targetReport.dialect,
            directory: targetReport.directory,
            seconds: targetReport.seconds,
            parserMetrics: targetReport.parserMetrics,
            rules: Object.fromEntries(targetReport.rules),
        },
        baseline: {
            dialect: baselineReport.dialect,
            directory: baselineReport.directory,
            seconds: baselineReport.seconds,
            parserMetrics: baselineReport.parserMetrics,
            rules: Object.fromEntries(baselineReport.rules),
        },
        delta: {
            totalPositiveDelta: delta.totalPositiveDelta,
            targetOnlyDelta: delta.targetOnlyDelta,
            top20Delta,
            clusters,
            rows: delta.sortedByDelta,
        },
    });

    assertBudget(targetReport, baselineReport, options);
}

main();
