import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { checkConfiguration, lintConfiguration } from "./configuration.ts";
import { runTool } from "./tool.ts";
import { CheckResult } from "../inspect/diagnostic.ts";
import { CheckError } from "../error/index.ts";
import type { Plugin } from "../lint/plugin.ts";

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

/** Keep mutable checkout configuration out of managed invocations. */
async function lintPackage(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // verify editor settings before preparing the managed invocation
    const directory = resolve(options.directory);
    await checkConfiguration(directory, options.plugins);
    const temporary = await mkdtemp(join(tmpdir(), "destack-check-"));
    try {
        // write the fixed rules with absolute plugin paths
        const configuration = join(temporary, "lint.json");
        await writeFile(configuration, JSON.stringify(lintConfiguration(options.plugins, true)));

        // run the checker against the selected source
        const output = await runTool(
            "oxlint",
            [
                "--config",
                configuration,
                "--disable-nested-config",
                "--no-ignore",
                "--format",
                "json",
                ...(fix ? ["--fix"] : []),
                "--",
                ...(options.files ?? ["src"]).map((file) => resolve(directory, file)),
            ],
            directory,
            options.signal,
        );

        // distinguish tool failure from ordinary rule diagnostics
        if (!output.stdout.trim()) {
            throw new CheckError("tool", output.stderr.trim() || "oxlint returned no diagnostics");
        }

        // decode the tool response and reject unexplained failures
        const report = JSON.parse(output.stdout);
        const result = CheckResult.parse({ diagnostics: report.diagnostics });
        if (output.code !== 0 && result.diagnostics.length === 0) {
            throw new CheckError(
                "tool",
                output.stderr.trim() || "oxlint failed without diagnostics",
            );
        }

        return result;
    } finally {
        await rm(temporary, { recursive: true });
    }
}
