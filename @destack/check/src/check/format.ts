import type { FileSystem } from "@destack/environment";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type * as oxfmt from "oxfmt";
import type { Toolchain } from "../toolchain/index.ts";
import { checkConfiguration, formatConfiguration } from "./configuration.ts";
import type { CheckOptions } from "./index.ts";
import { runTool, type ToolResult } from "./tool.ts";
import { selectFiles } from "./selection.ts";
import { CheckError } from "../error/index.ts";

/** The files kept as written: migration snapshots, generated JSON Schemas, byte-exact fixtures. */
const WRITTEN_AS_IS = [
    "**/migration/**/snapshot.json",
    "**/schemas/*.json",
    "**/fixture/**/source/**",
    "**/fixture/**/expected/**",
] as const;

/** The longest the formatter runs, in milliseconds, far above formatting a scope's sources. */
const FORMAT_TIMEOUT = 30_000;

/** This package's directory, whose dependencies include the tools when running from a workspace. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** Format generated source with a toolchain's formatter and the package files' fixed settings. */
export async function formatSource(
    filename: string,
    source: string,
    toolchain: Toolchain,
): Promise<string> {
    // load the formatter's module from the toolchain, refusing one without its format function
    const directory = toolchain.locate(["oxfmt"], PACKAGE);
    const entry = createRequire(join(directory, "package.json")).resolve("oxfmt");
    const loaded: unknown = await import(pathToFileURL(entry).href);
    if (!isFormatter(loaded)) {
        throw CheckError.of("TOOL_FAILED", `the formatter has no format function: ${entry}`);
    }

    // format with the fixed settings and reject parse failures
    const result = await loaded.format(filename, source, formatConfiguration);
    if (result.errors.length) {
        throw CheckError.of("TOOL_FAILED", JSON.stringify(result.errors));
    }

    return result.code;
}

/** Format package source, or check its formatting without writing. */
export async function formatPackage(options: CheckOptions, write = true): Promise<ToolResult> {
    // verify editor settings before preparing the managed invocation
    const { files } = options.environment;
    await checkConfiguration(options.directory, files, options.plugins);

    // find the formatter among the tools
    const executable = join(options.toolchain.locate(["oxfmt"], PACKAGE), "bin", "oxfmt");

    // isolate the generated format configuration
    const temporary = await files.makeTemporaryDirectory("destack-format-");
    try {
        // supply fixed formatting and include every selected file
        const configuration = join(temporary, "format.json");
        await files.writeTextFile(configuration, JSON.stringify(formatConfiguration));
        // leave the files kept as written
        const ignore = await writeIgnore(temporary, files);

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
                ...(await selectFiles(options)).map((file) => resolve(options.directory, file)),
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

/** Write the ignore file of the files kept as written into a directory, answering its path. */
export async function writeIgnore(directory: string, files: FileSystem): Promise<string> {
    const ignore = join(directory, "ignore");
    await files.writeTextFile(ignore, WRITTEN_AS_IS.map((pattern) => `${pattern}\n`).join(""));

    return ignore;
}

/** Report whether a loaded module is the formatter's, with its format function. */
function isFormatter(module: unknown): module is Pick<typeof oxfmt, "format"> {
    return (
        typeof module === "object" &&
        module !== null &&
        "format" in module &&
        typeof module.format === "function"
    );
}
