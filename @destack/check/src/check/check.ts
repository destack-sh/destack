import { basename, dirname, join, relative, resolve } from "node:path";
import type { Environment, FileSystem } from "@destack/environment";
import { writeIgnore } from "./format.ts";
import { selectFiles } from "./selection.ts";
import { readExpectations, applyExpectations } from "./expectation.ts";
import {
    checkConfiguration,
    lintConfiguration,
    lintSettings,
    settingsRoot,
} from "./configuration.ts";
import { runBun } from "./tool.ts";
import { Toolchain } from "../toolchain/index.ts";
import { CheckResult, type Diagnostic } from "../diagnostic/diagnostic.ts";
import { checkMarkdown, fixMarkdown } from "../markdown/index.ts";
import { Architecture } from "../architecture/index.ts";
import { CheckError } from "../error/index.ts";
import type { Plugin } from "../lint/plugin.ts";
import { Digest, schema } from "@destack/schema";
import { TransformCache } from "@destack/package/build";

/** The directory below a package with its lint results, by the digest of what each lint read. */
const LINT_CACHE = "node_modules/.cache/destack/lint";

/** The file that lays out a workspace's packages in layers. */
const ARCHITECTURE_FILE = "ARCHITECTURE.md";

/** A package.json: a package's description, or a workspace's packages. */
const Manifest = schema.looseObject({
    description: schema.string().min(1).exactOptional(),
    workspaces: schema.array(schema.string()).exactOptional(),
});

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
    /** The tools the check runs. */
    toolchain: Toolchain;
    /** The digest of the program a typecheck just read, keying kept lint results. */
    program?: Digest;
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
    const selection = await selectFiles(options);
    for (const entry of selection) {
        if (!(await files.exists(resolve(directory, entry)))) {
            throw CheckError.of(
                "CONFIGURATION_INVALID",
                `selected path '${entry}' is invalid: it does not exist`,
            );
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
        const description = await readmeDescription(file, files);
        diagnostics.push(...checkMarkdown(relative(directory, file), text, description));

        // check the layers of an architecture against its workspace's packages
        if (basename(file) === ARCHITECTURE_FILE) {
            const packages = await Architecture.packages(dirname(file), files);
            diagnostics.push(
                ...Architecture.parse(text).check(packages, relative(directory, file), text),
            );
        }
    }

    return diagnostics;
}

/** Read the description of a README's package, none for another file. */
async function readmeDescription(file: string, files: FileSystem): Promise<string | undefined> {
    // read the sibling package.json of a README
    const manifest = join(dirname(file), "package.json");
    if (basename(file) !== "README.md" || !(await files.exists(manifest))) {
        return undefined;
    }
    // read a package's description, refusing a package without one, and none for a workspace's root
    const read = Manifest.parse(JSON.parse(await files.readTextFile(manifest)));
    if (read.workspaces !== undefined) {
        return undefined;
    } else if (read.description === undefined) {
        throw CheckError.of(
            "CONFIGURATION_INVALID",
            `package '${dirname(manifest)}' is invalid: its package.json has no description`,
        );
    }

    return read.description;
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
export async function listFiles(directory: string, files: FileSystem): Promise<string[]> {
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

/** Read a lint plugin's rules: a toolchain's bundle, else every file beside the plugin's module. */
async function readRules(
    plugin: string,
    toolchain: Toolchain,
    files: FileSystem,
): Promise<string[]> {
    // read the released bundle, else each source file in the plugin's directory, in a stable order
    const paths =
        toolchain.directory === undefined
            ? (await listFiles(dirname(plugin), files)).toSorted()
            : [plugin];

    return await Promise.all(
        paths.map(async (path) => `${path}\n${await files.readTextFile(path)}`),
    );
}

/** Lint TypeScript and JavaScript sources with Oxc and the Destack rules. */
async function lintSources(options: CheckOptions, fix: boolean): Promise<CheckResult> {
    // lint with the running Bun's linter
    const { toolchain } = options;
    const { files, process } = options.environment;

    // isolate the generated lint configuration
    const temporary = await files.makeTemporaryDirectory("destack-check-");
    try {
        // write the fixed rules with absolute plugin paths, and the settings root's packages
        const configuration = join(temporary, "lint.json");
        const settings = await lintSettings(await settingsRoot(options.directory, files), files);
        const lint = lintConfiguration(settings, {
            ...(options.plugins === undefined ? {} : { plugins: options.plugins }),
            builtin: toolchain.lint(),
        });
        await files.writeTextFile(configuration, JSON.stringify(lint));
        const ignore = await writeIgnore(temporary, files);
        const selected = (await selectFiles(options)).map((file) =>
            resolve(options.directory, file),
        );

        // reuse the result of the same program, files, rules and tools, never a fix's
        const cache =
            options.program === undefined || fix
                ? undefined
                : TransformCache.directory(
                      join(options.directory, LINT_CACHE),
                      options.environment,
                  );
        const plugins = [
            toolchain.lint(),
            ...(options.plugins ?? []).map((plugin) => plugin.specifier),
        ];
        const inputs = {
            program: options.program,
            lint,
            selected,
            rules: await Promise.all(plugins.map((plugin) => readRules(plugin, toolchain, files))),
            runtime: process.runtime.revision,
        };
        const key =
            cache === undefined ? undefined : await Digest.json(inputs, options.environment.crypto);
        const stored = key === undefined ? undefined : await cache?.read(key);
        if (stored !== undefined) {
            return CheckResult.parse(JSON.parse(new TextDecoder().decode(stored)));
        }

        // run the linter against the selected source, its type-aware rules included
        const output = await runBun(
            [
                "lint",
                "--no-script",
                "--config",
                configuration,
                "--no-config-lookup",
                "--ignore-path",
                ignore,
                "--format",
                "json",
                "--type-aware",
                ...(fix ? ["--fix"] : []),
                "--",
                ...selected,
            ],
            "bun lint",
            options.directory,
            options.environment,
            options.signal === undefined ? {} : { signal: options.signal },
        );

        // distinguish tool failure from ordinary rule diagnostics
        if (!output.stdout.trim()) {
            throw CheckError.of(
                "TOOL_BROKEN",
                output.stderr.trim() || "bun lint returned no diagnostics",
            );
        }

        // decode the tool response and reject unexplained failures
        let report: unknown;
        try {
            report = JSON.parse(output.stdout);
        } catch (error) {
            throw CheckError.of("TOOL_BROKEN", output.stderr.trim() || output.stdout.trim(), {
                cause: error,
            });
        }
        const { diagnostics } = Report.parse(report);
        const result = CheckResult.parse({ diagnostics });
        if (output.code !== 0 && result.diagnostics.length === 0) {
            throw CheckError.of(
                "TOOL_BROKEN",
                output.stderr.trim() || "bun lint failed without diagnostics",
            );
        }

        // keep the result under its inputs' digest
        if (key !== undefined) {
            await cache?.write(key, new TextEncoder().encode(JSON.stringify(result)));
        }

        return result;
    } finally {
        await files.remove(temporary, { recursive: true });
    }
}
