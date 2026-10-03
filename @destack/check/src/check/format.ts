import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkConfiguration, formatConfiguration } from "./configuration.ts";
import type { CheckOptions } from "./index.ts";
import { runTool, type ToolResult } from "./tool.ts";
import { Toolchain } from "./toolchain.ts";
import { CheckError } from "../error/index.ts";

/** This package's directory, whose dependencies hold the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** Format generated source with the same fixed settings as package files. */
export async function formatSource(filename: string, source: string): Promise<string> {
    // format with the fixed settings and reject parse failures
    const { format } = await import("oxfmt");
    const result = await format(filename, source, formatConfiguration);
    if (result.errors.length) {
        throw new CheckError("tool", JSON.stringify(result.errors));
    }

    return result.code;
}

/** Format package source, or check its formatting without writing. */
export async function formatPackage(options: CheckOptions, write = true): Promise<ToolResult> {
    // verify editor settings before preparing the managed invocation
    await checkConfiguration(options.directory, options.plugins);

    // find the formatter among the tools
    const executable = join(Toolchain.locate(["oxfmt"], PACKAGE), "bin", "oxfmt");

    // isolate the generated format configuration
    const temporary = await mkdtemp(join(tmpdir(), "destack-format-"));
    try {
        // supply fixed formatting and include every selected file
        const configuration = join(temporary, "format.json");
        await writeFile(configuration, JSON.stringify(formatConfiguration));
        // leave migration snapshots and byte-exact fixture sources and outputs as written
        const ignore = join(temporary, "ignore");
        await writeFile(
            ignore,
            [
                "**/migration/**/snapshot.json",
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
            options.signal,
        );
    } finally {
        await rm(temporary, { recursive: true });
    }
}
