import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkConfiguration, formatConfiguration } from "./configuration.ts";
import type { CheckOptions } from "./index.ts";
import { runTool, type ToolResult } from "./tool.ts";
import { Toolchain } from "../toolchain/index.ts";
import { CheckError } from "../error/index.ts";

/** The longest the formatter runs, in milliseconds, far above formatting a scope's sources. */
const FORMAT_TIMEOUT = 30_000;

/** This package's directory, whose dependencies include the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** Format generated source with the same fixed settings as package files. */
export async function formatSource(filename: string, source: string): Promise<string> {
    // format with the fixed settings and reject parse failures
    const { format } = await import("oxfmt");
    const result = await format(filename, source, formatConfiguration);
    if (result.errors.length) {
        throw CheckError.of("TOOL_FAILED", JSON.stringify(result.errors));
    }

    return result.code;
}

/** Format package source, or check its formatting without writing. */
export async function formatPackage(options: CheckOptions, write = true): Promise<ToolResult> {
    // verify editor settings before preparing the managed invocation
    const { files, process } = options.environment;
    await checkConfiguration(options.directory, files, options.plugins);

    // find the formatter among the tools
    const executable = join(Toolchain.locate(["oxfmt"], PACKAGE, process), "bin", "oxfmt");

    // isolate the generated format configuration
    const temporary = await files.makeTemporaryDirectory("destack-format-");
    try {
        // supply fixed formatting and include every selected file
        const configuration = join(temporary, "format.json");
        await files.writeTextFile(configuration, JSON.stringify(formatConfiguration));
        // leave migration snapshots, generated JSON Schemas and byte-exact fixture files as written
        const ignore = join(temporary, "ignore");
        await files.writeTextFile(
            ignore,
            [
                "**/migration/**/snapshot.json",
                "**/schemas/*.json",
                "**/fixture/**/source/**",
                "**/fixture/**/expected/**",
            ]
                .map((pattern) => `${pattern}\n`)
                .join(""),
        );

        return await runTool(
            executable,
            [
                "--config",
                configuration,
                "--disable-nested-config",
                "--ignore-path",
                ignore,
                write ? "--write" : "--check",
                "--",
                ...(options.files ?? ["src"]).map((file) => resolve(options.directory, file)),
            ],
            resolve(options.directory),
            options.environment,
            {
                signal: AbortSignal.any([
                    options.environment.clock.timeout(FORMAT_TIMEOUT),
                    ...(options.signal === undefined ? [] : [options.signal]),
                ]),
            },
        );
    } finally {
        await files.remove(temporary, { recursive: true });
    }
}
