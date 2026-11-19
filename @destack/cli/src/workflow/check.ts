import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { CheckResult } from "@destack/check/diagnostic";
import type { Project, TestTier } from "@destack/check/project";
import { present } from "@destack/schema";
import type { Verb } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { CliError } from "../error/index.ts";

/** The tiers a test run can select. */
const TEST_TIERS = new Set<string>(["unit", "scenario", "bench"]);

/** Check a checkout's package: formatting, types, README listings, lints and its own check. */
export function checkCommand(context: Context): Verb {
    return {
        description: "Check a package's formatting, types and lints with its pinned toolchain",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        options: {
            since: {
                type: "string",
                description: "Lint only the modules changed since a commit",
                argument: "commit",
            },
        },
        json: true,
        async run({ arguments: { package: directory }, options, json }) {
            // check the package with the toolchain it pins
            const project = await open(present(directory, "the package argument"), context);
            const since = typeof options["since"] === "string" ? options["since"] : undefined;
            const checked = await project.check(since === undefined ? {} : { since });

            // print every diagnostic, refusing a package with any
            await report(checked, json, context);
        },
    };
}

/** Fix a checkout's package: its generated settings, formatting, lints and its own fix. */
export function fixCommand(context: Context): Verb {
    return {
        description: "Write a package's generated settings, format it and fix its lints",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        json: true,
        async run({ arguments: { package: directory }, json }) {
            // fix the package with the toolchain it pins
            const project = await open(present(directory, "the package argument"), context);

            // print what is left to fix by hand, refusing a package with any
            await report(await project.fix(), json, context);
        },
    };
}

/** Run a checkout's package's tests, its files' names deriving their tiers and environments. */
export function testCommand(context: Context): Verb {
    return {
        description:
            "Run a package's tests with its pinned toolchain, unit and scenario by default",
        arguments: [{ name: "package", default: ".", isDirectory: true }],
        rest: { name: "files", usage: "file" },
        options: {
            tier: {
                type: "string",
                description: "The tiers to run, comma separated: unit, scenario, bench",
                argument: "tiers",
            },
            database: {
                type: "string",
                description: "The one database to run against: sqlite or postgresql",
                argument: "database",
            },
            grep: {
                type: "string",
                description: "Run the tests whose full name matches the pattern",
                argument: "pattern",
            },
            list: { type: "boolean", description: "List the tests instead of running them" },
            update: {
                type: "boolean",
                description: "Write the snapshots the tests compare against anew",
            },
        },
        json: false,
        async run({ arguments: { package: directory }, rest, options }) {
            // run the package's tests with the toolchain it pins, as the options select
            const project = await open(present(directory, "the package argument"), context);
            const code = await project.test({
                ...testTiers(options["tier"]),
                ...testDatabase(options["database"]),
                files: rest,
                isListing: options["list"] === true,
                isUpdating: options["update"] === true,
                ...(typeof options["grep"] === "string" ? { grep: options["grep"] } : {}),
            });

            // refuse a run with failures
            if (code !== 0) {
                throw CliError.of("TEST_INVALID", `test is invalid: the run exited with ${code}`);
            }
        },
    };
}

/** Read the tiers a test run selects, refusing an unknown tier. */
function testTiers(value: unknown): { readonly tiers?: readonly TestTier[] } {
    // select the default tiers without the option
    if (typeof value !== "string") {
        return {};
    }

    // read each named tier
    const tiers = value.split(",").map((each) => each.trim());
    const unknown = tiers.find((each) => !TEST_TIERS.has(each));
    if (unknown !== undefined) {
        throw CliError.of(
            "USAGE_INVALID",
            `tier '${unknown}' is invalid: use unit, scenario or bench`,
        );
    }

    return { tiers: tiers.filter((each): each is TestTier => TEST_TIERS.has(each)) };
}

/** Read the one database a test run selects, refusing an unknown database. */
function testDatabase(value: unknown): { readonly database?: "sqlite" | "postgresql" } {
    // run every planned database without the option
    if (typeof value !== "string") {
        return {};
    } else if (value !== "sqlite" && value !== "postgresql") {
        throw CliError.of(
            "USAGE_INVALID",
            `database '${value}' is invalid: use sqlite or postgresql`,
        );
    }

    return { database: value };
}

/** Open a package with the toolchain it pins, as eslint and prettier run the local install. */
async function open(directory: string, context: Context): Promise<Project> {
    // resolve the package's own @destack/check, refusing a package that pins none
    const absolute = resolve(directory);
    const require = createRequire(join(absolute, "package.json"));
    const location = (() => {
        try {
            return require.resolve("@destack/check/project");
        } catch {
            throw CliError.of(
                "PACKAGE_INVALID",
                `package '${absolute}' is invalid: it pins no toolchain, so add @destack/check as a devDependency`,
            );
        }
    })();

    // open the package through it, refusing a module that exports no project
    const toolchain: unknown = await import(pathToFileURL(location).href);
    if (!isToolchain(toolchain)) {
        throw CliError.of(
            "PACKAGE_INVALID",
            `package '${absolute}' is invalid: its @destack/check exports no Project`,
        );
    }

    return toolchain.Project.open(absolute, context.environment);
}

/** Print a check's diagnostics, then refuse a package that has any. */
async function report(checked: CheckResult, json: boolean, context: Context): Promise<void> {
    // print the diagnostics as JSON, or one line each
    const { diagnostics } = checked;
    const lines = diagnostics.map((each) => {
        const [label] = each.labels;
        const place = label === undefined ? "" : `:${label.span.line}:${label.span.column}`;

        return `${each.filename}${place}  ${each.code ?? each.severity}  ${each.message}`;
    });
    const { terminal } = context;
    const clean = terminal.event("Checked", "done", "no diagnostics");
    await terminal.print(
        json ? JSON.stringify(checked) : diagnostics.length === 0 ? clean : lines.join("\n"),
    );

    // refuse a package with diagnostics
    if (diagnostics.length > 0) {
        throw CliError.of(
            "CHECK_INVALID",
            `check is invalid: the package has ${diagnostics.length} diagnostics`,
        );
    }
}

/** Report whether a module is a toolchain's project module, exporting its Project. */
function isToolchain(module: unknown): module is { readonly Project: typeof Project } {
    return typeof module === "object" && module !== null && "Project" in module;
}
