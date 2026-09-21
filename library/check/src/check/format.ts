import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { checkConfiguration, formatConfiguration } from "./configuration.ts";
import type { CheckOptions } from "./index.ts";
import { runTool, type ToolResult } from "./tool.ts";

/** Format package source, or check its formatting without writing. */
export async function formatPackage(options: CheckOptions, write = true): Promise<ToolResult> {
    // verify editor settings before preparing the managed invocation
    await checkConfiguration(options.directory, options.plugins);
    const temporary = await mkdtemp(join(tmpdir(), "destack-format-"));
    try {
        // supply fixed formatting and include every selected file
        const configuration = join(temporary, "format.json");
        await writeFile(configuration, JSON.stringify(formatConfiguration));
        const ignore = join(temporary, "ignore");
        await writeFile(ignore, "");

        return await runTool(
            "oxfmt",
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
