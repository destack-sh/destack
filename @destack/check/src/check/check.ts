import { mkdtemp, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { readExpectations, applyExpectations } from "./expectation.ts";
import { checkConfiguration, lintConfiguration, settingsRoot } from "./configuration.ts";
import { runTool } from "./tool.ts";
import { Toolchain } from "../toolchain/index.ts";
import { CheckResult, type Diagnostic } from "../inspect/diagnostic.ts";
import { checkMarkdown, fixMarkdown } from "../markdown/index.ts";
import { CheckError } from "../error/index.ts";
import type { Plugin } from "../lint/plugin.ts";
import { schema } from "@destack/schema";

/** This package's directory, whose dependencies hold the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** The diagnostics of an oxlint JSON report. */
const Report = schema.looseObject({ diagnostics: schema.unknown() });

/** Source selection for a package check. */
export interface CheckOptions {
    /** The package checkout. */
    directory: string;
    /** Selected files or directories, relative to the checkout. */
    files?: string[];
    /** Cancel the running checker or formatter. */
    signal?: AbortSignal;
    /** Additional trusted plugins selected by the host, with every rule enabled. */
    plugins?: readonly Plugin[];
}

/** Check source using the canonical rules. */
export function checkPackage(options: CheckOptions): Promise<CheckResult> {
    return lintPackage(options, false);
}

/** Apply the linter's safe fixes using the canonical rules. */
export function fixPackage(options: CheckOptions): Promise<CheckResult> {
    return lintPackage(options, true);
}

/** Check the selected Markdown and sources with the canonical rules, applying fixes when requested. */
async function lintPackage(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // verify editor settings before preparing the managed invocation
    const directory = resolve(options.directory);
    await checkConfiguration(directory, options.plugins);

    // require every selected path
    const selection = options.files ?? ["src"];
    for (const entry of selection) {
        if (!existsSync(resolve(directory, entry))) {
            throw new CheckError("CONFIGURATION", `selected path does not exist: ${entry}`);
        }
    }

    // check Markdown separately and lint the remaining selection with Oxc
    const markdown = await checkMarkdownFiles(directory, selection, fix);
    const sources: string[] = [];
    for (const entry of selection) {
        if (!entry.endsWith(".md") && (await hasSources(resolve(directory, entry)))) {
            sources.push(entry);
        }
    }
    if (!sources.length) {
        return CheckResult.parse({ diagnostics: markdown });
    }
    const result = await lintSources({ ...options, directory, files: sources }, fix);

    // accept the findings the covering workspace's packages expect, refusing expectations nothing met
    const expectations = await readExpectations(await settingsRoot(directory));

    return CheckResult.parse({
        diagnostics: [
            ...markdown,
            ...applyExpectations(result.diagnostics, expectations, directory, sources),
        ],
    });
}

/** Check selected Markdown files and those below selected directories, fixing them when requested. */
async function checkMarkdownFiles(
    directory: string,
    selection: readonly string[],
    fix: boolean,
): Promise<Diagnostic[]> {
    // expand directories into their Markdown files outside dependencies
    const files: string[] = [];
    for (const entry of selection) {
        const path = resolve(directory, entry);
        const stats = await stat(path);
        if (stats.isDirectory()) {
            files.push(...(await listFiles(path)).filter((file) => file.endsWith(".md")));
        } else if (entry.endsWith(".md")) {
            files.push(path);
        }
    }

    // fix sentence breaks before reporting what remains
    const diagnostics: Diagnostic[] = [];
    for (const file of files) {
        let text = await readFile(file, "utf8");
        if (fix) {
            const fixed = fixMarkdown(text);
            if (fixed !== text) {
                await writeFile(file, fixed);
                text = fixed;
            }
        }
        diagnostics.push(...checkMarkdown(relative(directory, file), text));
    }

    return diagnostics;
}

/** Report whether a selected file or directory contains JavaScript or TypeScript sources. */
async function hasSources(path: string): Promise<boolean> {
    // match a selected file directly and search a selected directory outside dependencies
    const pattern = /\.[cm]?[jt]sx?$/u;
    if (!(await stat(path)).isDirectory()) {
        return pattern.test(path);
    }

    return (await listFiles(path)).some((file) => pattern.test(file));
}

/** List the files below a directory, skipping dependencies and hidden tool state, and leaving symbolic links unfollowed. */
async function listFiles(directory: string): Promise<string[]> {
    const files: string[] = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        // descend into real directories outside dependencies and hidden ones, such as .terraform
        const path = join(directory, entry.name);
        if (entry.isDirectory() && entry.name !== "node_modules" && !entry.name.startsWith(".")) {
            files.push(...(await listFiles(path)));
        }
        // keep regular files
        else if (entry.isFile()) {
            files.push(path);
        }
    }

    return files;
}

/** Lint TypeScript and JavaScript sources with Oxc and the Destack rules. */
async function lintSources(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // find the linter and the type checker of this platform among the tools
    const executable = join(Toolchain.locate(["oxlint"], PACKAGE), "bin", "oxlint");
    const platform = `@oxlint-tsgolint/${process.platform}-${process.arch}`;
    const binary = process.platform === "win32" ? "tsgolint.exe" : "tsgolint";
    const typeChecker = join(Toolchain.locate(["oxlint-tsgolint", platform], PACKAGE), binary);

    // isolate the generated lint configuration
    const temporary = await mkdtemp(join(tmpdir(), "destack-check-"));
    try {
        // write the fixed rules with absolute plugin paths
        const configuration = join(temporary, "lint.json");
        await writeFile(configuration, JSON.stringify(lintConfiguration(options.plugins, true)));

        // run the checker against the selected source
        const output = await runTool(
            executable,
            [
                "--config",
                configuration,
                "--disable-nested-config",
                "--no-ignore",
                "--format",
                "json",
                ...(fix ? ["--fix"] : []),
                "--",
                ...(options.files ?? ["src"]).map((file) => resolve(options.directory, file)),
            ],
            options.directory,
            options.signal,
            { ...process.env, OXLINT_TSGOLINT_PATH: typeChecker },
        );

        // distinguish tool failure from ordinary rule diagnostics
        if (!output.stdout.trim()) {
            throw new CheckError("TOOL", output.stderr.trim() || "oxlint returned no diagnostics");
        }

        // decode the tool response and reject unexplained failures
        let report: unknown;
        try {
            report = JSON.parse(output.stdout);
        } catch (error) {
            throw new CheckError("TOOL", output.stderr.trim() || output.stdout.trim(), {
                cause: error,
            });
        }
        const { diagnostics } = Report.parse(report);
        const result = CheckResult.parse({ diagnostics });
        if (output.code !== 0 && result.diagnostics.length === 0) {
            throw new CheckError(
                "TOOL",
                output.stderr.trim() || "oxlint failed without diagnostics",
            );
        }

        return result;
    } finally {
        await rm(temporary, { recursive: true });
    }
}
