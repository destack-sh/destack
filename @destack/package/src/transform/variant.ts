import { readdirSync, statSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import MagicString from "magic-string";
import { parseAst } from "rolldown/parseAst";
import { parseSync } from "rolldown/utils";
import type { Target } from "../definition/target.ts";

/** The source of each relative static import or re-export. */
const RELATIVE_SOURCE = /\bfrom\s*["'](\.[^"']*)["']/g;

/** A TypeScript module path, split into its stem, variant target and extension. */
const MODULE_PATH = /^(?<stem>.*?)(?:\.(?<target>browser|server))?(?<extension>\.[cm]?tsx?)$/;

/** The files of each directory, listed once when first asked. */
export class FileIndex {
    /** The file names by directory. */
    readonly #directories = new Map<string, ReadonlySet<string>>();

    /** Report whether a file exists, listing its directory on first use. */
    has(path: string): boolean {
        // list the directory once, as an absent directory holds no files
        const directory = dirname(path);
        let files = this.#directories.get(directory);
        if (files === undefined) {
            files = FileIndex.#list(directory);
            this.#directories.set(directory, files);
        }

        return files.has(basename(path));
    }

    /** List a directory's files, following links, failing on every error but its absence. */
    static #list(directory: string): ReadonlySet<string> {
        try {
            const entries = readdirSync(directory, { withFileTypes: true });
            const files = entries.filter(
                (entry) =>
                    entry.isFile() ||
                    (entry.isSymbolicLink() && isFile(join(directory, entry.name))),
            );

            return new Set(files.map((entry) => entry.name));
        } catch (error) {
            // treat an absent directory as empty
            if ((error as NodeJS.ErrnoException).code === "ENOENT") {
                return new Set();
            }
            throw error;
        }
    }
}

/** Resolve a relative import to the importing target's variant of the module, when one exists. */
export function resolveVariant(
    specifier: string,
    importer: string,
    target: Target,
    exists: (path: string) => boolean = isFile,
): string | undefined {
    // consider relative imports of TypeScript modules only
    const path = resolve(dirname(importer), specifier);
    const parts = specifier.startsWith(".") ? MODULE_PATH.exec(path)?.groups : undefined;
    if (!parts) {
        return undefined;
    }

    // reject explicit imports of another target's variant
    if (parts.target !== undefined && parts.target !== target) {
        throw new TypeError(`${target} module ${importer} imports ${parts.target} variant ${path}`);
    }

    // keep the base for its own variant and for explicit variant imports
    const variant = `${parts.stem}.${target}${parts.extension}`;
    if (parts.target !== undefined || importer === variant) {
        return undefined;
    }

    return exists(variant) ? variant : undefined;
}

/** Point a module's relative imports and re-exports at the target's variants. */
export function importVariants(
    code: string,
    path: string,
    target: Target,
    exists: (path: string) => boolean = isFile,
): string | undefined {
    // skip modules whose relative imports have no variant
    const specifiers = [...code.matchAll(RELATIVE_SOURCE)].map((match) => match[1]!);
    if (!specifiers.some((specifier) => resolveVariant(specifier, path, target, exists))) {
        return undefined;
    }

    // find the import and re-export sources without building the syntax tree
    const lang = /\.[cm]?tsx$/.test(path) ? "tsx" : "ts";
    const { module } = parseSync(path, code, { lang });
    const sources = new Map(
        [
            ...module.staticImports.map((statement) => statement.moduleRequest),
            ...module.staticExports.flatMap((statement) =>
                statement.entries.flatMap((entry) => entry.moduleRequest ?? []),
            ),
        ].map((source) => [source.start, source]),
    );

    // name each variant in its import specifier
    const source = new MagicString(code);
    let isChanged = false;
    for (const specifier of sources.values()) {
        if (resolveVariant(specifier.value, path, target, exists)) {
            const variant = specifier.value.replace(/(\.[cm]?tsx?)$/, `.${target}$1`);
            source.overwrite(specifier.start, specifier.end, JSON.stringify(variant));
            isChanged = true;
        }
    }

    return isChanged ? source.toString() : undefined;
}

/** Require a variant module to re-export every export of the base it replaces. */
export function requireBase(code: string, path: string): void {
    // leave modules that are not variants unchecked
    const parts = MODULE_PATH.exec(path)?.groups;
    if (parts?.target === undefined) {
        return;
    }

    // find `export * from "./base.ts"` among the module's statements
    const base = `./${basename(parts.stem)}${parts.extension}`;
    const program = parseAst(code, { lang: /\.[cm]?tsx$/.test(path) ? "tsx" : "ts" }, path);
    const isReexported = program.body.some(
        (statement) =>
            statement.type === "ExportAllDeclaration" &&
            statement.exported === null &&
            statement.source.value === base,
    );
    if (!isReexported) {
        throw new TypeError(`variant ${path} must re-export its base with export * from "${base}"`);
    }
}

/** Report whether a file exists, failing on every error other than its absence. */
function isFile(path: string): boolean {
    return statSync(path, { throwIfNoEntry: false })?.isFile() ?? false;
}
