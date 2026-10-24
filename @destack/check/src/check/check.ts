import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment, FileSystem } from "@destack/environment";
import { readExpectations, applyExpectations } from "./expectation.ts";
import {
    checkConfiguration,
    lintConfiguration,
    lintSettings,
    settingsRoot,
} from "./configuration.ts";
import { runTool } from "./tool.ts";
import { Toolchain } from "../toolchain/index.ts";
import { CheckResult, type Diagnostic } from "../diagnostic/diagnostic.ts";
import { checkMarkdown, fixMarkdown } from "../markdown/index.ts";
import { CheckError } from "../error/index.ts";
import type { Plugin } from "../lint/plugin.ts";
import { schema } from "@destack/schema";

/** This package's directory, whose dependencies include the tools when running from a workspace. */
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
    /** The files, process and machine the tools run on. */
    environment: Environment<"files" | "process" | "host">;
}

/** Check source using the canonical rules. */
export function checkPackage(options: CheckOptions): Promise<CheckResult> {
    return lintPackage(options, false);
}

/** Apply the linter's safe fixes using the canonical rules. */
export function fixPackage(options: CheckOptions): Promise<CheckResult> {
    return lintPackage(options, true);
}

/** Check the selected Markdown and sources with the canonical rules, fixing when requested. */
async function lintPackage(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // verify editor settings before preparing the managed invocation
    const directory = resolve(options.directory);
    const { files } = options.environment;
    await checkConfiguration(directory, files, options.plugins);

    // require every selected path
    const selection = options.files ?? ["src"];
    for (const entry of selection) {
        if (!(await files.exists(resolve(directory, entry)))) {
            throw CheckError.of("INVALID_CONFIGURATION", `selected path does not exist: ${entry}`);
        }
    }

    // check Markdown separately and lint the remaining selection with Oxc
    const markdown = await checkMarkdownFiles(directory, selection, fix, files);
    const sources: string[] = [];
    for (const entry of selection) {
        if (!entry.endsWith(".md") && (await hasSources(resolve(directory, entry), files))) {
            sources.push(entry);
        }
    }
    if (!sources.length) {
        return CheckResult.parse({ diagnostics: markdown });
    }
    const result = await lintSources({ ...options, directory, files: sources }, fix);

    // accept the findings the workspace's packages expect, and refuse expectations nothing met
    const expectations = await readExpectations(await settingsRoot(directory, files), files);

    return CheckResult.parse({
        diagnostics: [
            ...markdown,
            ...applyExpectations(result.diagnostics, expectations, directory, sources),
        ],
    });
}

/** Check selected Markdown files and those below selected directories, fixing when requested. */
async function checkMarkdownFiles(
    directory: string,
    selection: readonly string[],
    fix: boolean,
    files: FileSystem,
): Promise<Diagnostic[]> {
    // expand directories into their Markdown files outside dependencies
    const documents: string[] = [];
    for (const entry of selection) {
        const path = resolve(directory, entry);
        if ((await files.stat(path)).kind === "directory") {
            const listed = await listFiles(path, files);
            documents.push(...listed.filter((file) => file.endsWith(".md")));
        } else if (entry.endsWith(".md")) {
            documents.push(path);
        }
    }

    // fix sentence breaks before reporting what remains
    const diagnostics: Diagnostic[] = [];
    for (const file of documents) {
        let text = await files.readTextFile(file);
        if (fix) {
            const fixed = fixMarkdown(text);
            if (fixed !== text) {
                await files.writeTextFile(file, fixed);
                text = fixed;
            }
        }
        diagnostics.push(...checkMarkdown(relative(directory, file), text));
    }

    return diagnostics;
}

/** Report whether a selected file or directory contains JavaScript or TypeScript sources. */
async function hasSources(path: string, files: FileSystem): Promise<boolean> {
    // match a selected file directly and search a selected directory outside dependencies
    const pattern = /\.[cm]?[jt]sx?$/u;
    if ((await files.stat(path)).kind !== "directory") {
        return pattern.test(path);
    }

    return (await listFiles(path, files)).some((file) => pattern.test(file));
}

/** List the files below a directory. */
async function listFiles(directory: string, files: FileSystem): Promise<string[]> {
    const listed: string[] = [];
    for (const entry of await files.readDirectory(directory)) {
        // descend into real directories outside dependencies and hidden ones, such as .terraform
        const path = join(directory, entry.name);
        const isDescended =
            entry.kind === "directory" &&
            entry.name !== "node_modules" &&
            !entry.name.startsWith(".");
        if (isDescended) {
            listed.push(...(await listFiles(path, files)));
        }
        // take regular files
        else if (entry.kind === "file") {
            listed.push(path);
        }
    }

    return listed;
}

/** Lint TypeScript and JavaScript sources with Oxc and the Destack rules. */
async function lintSources(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // find the linter and the type checker of this platform among the tools
    const { files, process, host } = options.environment;
    const executable = join(Toolchain.locate(["oxlint"], PACKAGE, process), "bin", "oxlint");
    const checker = `@oxlint-tsgolint/${host.os}-${host.architecture}`;
    const binary = host.os === "win32" ? "tsgolint.exe" : "tsgolint";
    const typeChecker = join(
        Toolchain.locate(["oxlint-tsgolint", checker], PACKAGE, process),
        binary,
    );

    // isolate the generated lint configuration
    const temporary = await files.makeTemporaryDirectory("destack-check-");
    try {
        // write the fixed rules with absolute plugin paths, and the settings root's packages
        const configuration = join(temporary, "lint.json");
        const settings = await lintSettings(await settingsRoot(options.directory, files), files);
        const lint = lintConfiguration(settings, {
            ...(options.plugins === undefined ? {} : { plugins: options.plugins }),
            builtin: Toolchain.lint(process),
        });
        await files.writeTextFile(configuration, JSON.stringify(lint));

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
            options.environment,
            {
                ...(options.signal === undefined ? {} : { signal: options.signal }),
                variables: { OXLINT_TSGOLINT_PATH: typeChecker },
            },
        );

        // distinguish tool failure from ordinary rule diagnostics
        if (!output.stdout.trim()) {
            throw CheckError.of(
                "TOOL_FAILED",
                output.stderr.trim() || "oxlint returned no diagnostics",
            );
        }

        // decode the tool response and reject unexplained failures
        let report: unknown;
        try {
            report = JSON.parse(output.stdout);
        } catch (error) {
            throw CheckError.of("TOOL_FAILED", output.stderr.trim() || output.stdout.trim(), {
                cause: error,
            });
        }
        const { diagnostics } = Report.parse(report);
        const result = CheckResult.parse({ diagnostics });
        if (output.code !== 0 && result.diagnostics.length === 0) {
            throw CheckError.of(
                "TOOL_FAILED",
                output.stderr.trim() || "oxlint failed without diagnostics",
            );
        }

        return result;
    } finally {
        await files.remove(temporary, { recursive: true });
    }
}
