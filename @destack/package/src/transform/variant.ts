import { statSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { parseAst } from "rolldown/parseAst";
import type { Target } from "../definition/target.ts";

/** A TypeScript module path, split into its stem, variant target and extension. */
const MODULE_PATH = /^(?<stem>.*?)(?:\.(?<target>browser|server))?(?<extension>\.[cm]?tsx?)$/;

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
