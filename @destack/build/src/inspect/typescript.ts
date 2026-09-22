import { BuildError } from "../error/index.ts";
import { realpathSync } from "node:fs";
import { readFile, realpath } from "node:fs/promises";
import { basename, dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { version } from "typescript";
import {
    API,
    type Project,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
} from "typescript/unstable/async";
import {
    isExportDeclaration,
    isNoSubstitutionTemplateLiteral,
    isStringLiteral,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import { DependencySymbol, ModuleDescription } from "@destack/package/code";
import { Package, DependencyPackage } from "@destack/package/package";
import { SymbolInspector } from "./symbol.ts";
import { describeFile } from "@destack/package/file";
import type { TestDeclaration } from "@destack/test/inspect";
import { inspectErrors } from "@destack/check/inspect";
import { collectTests } from "./test.ts";
import { collectDeclarations, type Declaration } from "./declaration.ts";
import { collectGlobals } from "./global.ts";
import { collectDirectories, type DirectoryReference } from "./directory.ts";
import { modulePackage } from "../source/dependency.ts";

/** Compiler descriptions of modules and statically identified tests. */
export interface TypeScriptInspection {
    /** Exact source bytes checked against the compiler's parsed modules. */
    sources: Map<string, Uint8Array<ArrayBuffer>>;
    /** Literal directory references indexed by absolute source module path. */
    directories: Map<string, DirectoryReference[]>;
    /** Exported domain declarations to evaluate after static collection. */
    declarations: Declaration[];
    /** Public declarations and exports. */
    modules: ModuleDescription[];
    /** Statically collected test and suite declarations. */
    tests: TestDeclaration[];
}

/** Retain TypeScript compiler state across inspections of one package. */
export class TypeScriptCompiler implements AsyncDisposable {
    /** The source package directory. */
    readonly directory: string;
    /** The compiler process and retained project state. */
    readonly #api: API;

    /** Start the native compiler for a source package. */
    constructor(directory: string) {
        this.directory = realpathSync(directory);
        this.#api = new API({
            cwd: this.directory,
        });
    }

    /** Inspect a configuration after notifying the compiler of changed files. */
    async inspect(configuration: string): Promise<TypeScriptInspection> {
        // refresh the selected configuration
        configuration = await realpath(resolve(this.directory, configuration));

        // refresh client ASTs while the native compiler retains incremental project state
        this.#api.clearSourceFileCache();
        const invalidated = await this.#api.updateSnapshot({
            fileChanges: { invalidateAll: true },
        });
        await invalidated.dispose();

        // reread configured include patterns so added and removed modules change the inventory
        const snapshot = await this.#api.updateSnapshot({
            openProjects: [configuration],
            fileChanges: { changed: [configuration] },
        });

        try {
            // require the requested compiler project
            const project = snapshot.getProject(configuration);
            if (!project) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `TypeScript did not load ${configuration}`,
                );
            }

            // collect configuration, source, and type diagnostics together
            const diagnostics = (
                await Promise.all([
                    project.program.getConfigFileParsingDiagnostics(),
                    project.program.getSyntacticDiagnostics(),
                    project.program.getBindDiagnostics(),
                    project.program.getProgramDiagnostics(),
                    project.program.getGlobalDiagnostics(),
                    project.program.getSemanticDiagnostics(),
                ])
            ).flat();
            if (diagnostics.length) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `TypeScript inspection failed: ${JSON.stringify(diagnostics)}`,
                );
            }

            return await describeProject(project, this.directory);
        } finally {
            await snapshot.dispose();
        }
    }

    /** Close the compiler and release retained projects. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#api.close();
    }
}

/** Describe source modules and associate reexports with their defining symbols. */
async function describeProject(project: Project, root: string): Promise<TypeScriptInspection> {
    // collect package descriptions and exact source bytes
    const modules = new Map<string, ModuleDescription>();
    const sources = new Map<string, Uint8Array<ArrayBuffer>>();
    const tests: TestDeclaration[] = [];
    const declarations: Declaration[] = [];
    const directories = new Map<string, DirectoryReference[]>();

    // queue declarations reached through exports and public types
    const pending = new Map<number, TypeScriptSymbol>();
    const inspector = new SymbolInspector(
        project,
        (symbol) => describeSymbol(symbol, project, root, modules, pending),
        (node) => ({
            file: relative(root, node.getSourceFile().fileName).split(sep).join("/"),
            start: node.getStart(),
            end: node.getEnd(),
        }),
    );

    // select authored modules from the compiler file inventory
    const fileNames = [...(await project.program.getSourceFileNames())].sort();
    const files = fileNames.filter(
        (file) => contains(root, file) && !relative(root, file).split(sep).includes("node_modules"),
    );
    const authored = new Set(files);

    // preserve domain declarations imported from shared source packages
    for (const file of fileNames) {
        if (/\.d\.[cm]?ts$/.test(file)) {
            continue;
        }
        const source = await project.program.getSourceFile(file);
        if (!source || source.isDeclarationFile) {
            continue;
        }

        // retain literal directory references for package files
        const references = await collectDirectories(source, project);
        if (references.length) {
            directories.set(file, references);
        }

        // associate domain declarations with their declaring package
        const owner = await modulePackage(dirname(file));
        const path = relative(owner.directory, file).split(sep).join("/");
        const cases = await collectTests(source, path, project);
        declarations.push(...(await collectDeclarations(source, path, project, owner, cases)));
        if (authored.has(file)) {
            tests.push(...cases);
        }
    }

    // collect modules before resolving declarations reached through reexports
    for (const file of files) {
        const source = await project.program.getSourceFile(file);
        if (!source) {
            throw new BuildError("INSPECTION_FAILED", `Missing compiler source: ${file}`);
        }

        // retain each authored module under its package-relative path
        const path = relative(root, file).split(sep).join("/");

        // require source bytes to match the compiler snapshot
        const bytes = new Uint8Array(await readFile(file));
        const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
        if (text !== source.text) {
            throw new BuildError("INSPECTION_FAILED", `Source changed during inspection: ${path}`);
        }

        // register every module before following references between declarations
        sources.set(path, bytes);
        modules.set(path, {
            path,
            globals: [],
            errors: [],
            source: await describeFile(path, "text/plain", bytes),
            length: source.text.length,
            imports: source.imports.map((entry) => {
                if (!isStringLiteral(entry) && !isNoSubstitutionTemplateLiteral(entry)) {
                    throw new BuildError("INSPECTION_FAILED", `Unsupported import in ${path}`);
                }

                return entry.text;
            }),
            symbols: [],
            exports: [],
        });
    }

    // preserve the compiler's resolution of aliases and merged declarations
    for (const file of files) {
        const source = await project.program.getSourceFile(file);
        if (!source) {
            throw new BuildError("INSPECTION_FAILED", `Missing compiler source: ${file}`);
        }

        // resolve globals and errors after every authored module has been registered
        const module = modules.get(relative(root, file).split(sep).join("/"))!;
        module.globals = await collectGlobals(source, project, module.path, inspector.reference);
        module.errors = await inspectErrors(source, inspector);
        const symbol = await project.checker.getSymbolAtLocation(source);
        if (!symbol) {
            continue;
        }

        // resolve exports to their original declarations
        for (const exported of await project.checker.getExportsOfModule(symbol)) {
            const original =
                exported.flags & SymbolFlags.Alias
                    ? await project.checker.getAliasedSymbol(exported)
                    : exported;
            if (await project.checker.isUnknownSymbol(original)) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Unresolved export: ${module.path}#${exported.name}`,
                );
            }

            // retain the public name and whether it exists at runtime
            const reference = await describeSymbol(original, project, root, modules, pending);
            module.exports.push({
                name: exported.name,
                symbol: reference,
                isTypeOnly: !(await isRuntimeExport(project, source, exported, new Set())),
            });
        }

        module.exports.sort((left, right) => compare(left.name, right.name));
    }

    // inspect exported declarations and local types reached through their public signatures
    for (const symbol of pending.values()) {
        const reference = await describeSymbol(symbol, project, root, modules, pending);
        if (!("module" in reference)) {
            throw new BuildError("INSPECTION_FAILED", "Queued an external declaration.");
        }
        const description = await inspector.describe(symbol, reference.name);
        modules.get(reference.module)!.symbols.push(description);
    }

    // order symbols after all referenced symbols have been inspected
    for (const module of modules.values()) {
        module.symbols.sort((left, right) => compare(left.name, right.name));
    }

    // parse compiler descriptions before returning them to executable inspectors
    const result = [...modules.values()].map((module) => ModuleDescription.parse(module));

    return { modules: result, sources, tests, declarations, directories };
}

/** Follow named and star reexports while preserving explicit type-only declarations. */
async function isRuntimeExport(
    project: Project,
    source: SourceFile,
    exported: TypeScriptSymbol,
    visited: Set<string>,
): Promise<boolean> {
    // stop cycles through star exports
    const key = `${source.fileName}#${exported.name}`;
    if (visited.has(key)) {
        return false;
    }
    visited.add(key);

    // discard symbols that exist only in the type namespace
    const original =
        exported.flags & SymbolFlags.Alias
            ? await project.checker.getAliasedSymbol(exported)
            : exported;
    if (!(original.flags & SymbolFlags.Value)) {
        return false;
    }

    // inspect declarations written in this module before following star exports
    for (const handle of exported.declarations) {
        const declaration = await handle.resolve(project);
        if (!declaration) {
            throw new BuildError("INSPECTION_FAILED", `Unresolved export declaration: ${key}`);
        }
        if (declaration.getSourceFile().fileName !== source.fileName) {
            continue;
        }

        // inspect type-only modifiers on the declaration and its ancestors
        let node = declaration;
        let isTypeOnly = false;
        while (node.kind !== SyntaxKind.SourceFile) {
            if ("isTypeOnly" in node && node.isTypeOnly === true) {
                isTypeOnly = true;
            }
            node = node.parent;
        }
        if (!isTypeOnly) {
            return true;
        }
    }

    // a value must be reachable through at least one value-exporting star declaration
    for (const statement of source.statements) {
        if (
            !isExportDeclaration(statement) ||
            statement.exportClause ||
            statement.isTypeOnly ||
            !statement.moduleSpecifier
        ) {
            continue;
        }

        // resolve the star export module
        const target = await project.checker.getSymbolAtLocation(statement.moduleSpecifier);
        if (!target) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Unresolved export module: ${statement.moduleSpecifier.getText()}`,
            );
        }

        // follow only exports that resolve to the same original symbol
        const candidate = (await project.checker.getExportsOfModule(target)).find(
            (entry) => entry.name === exported.name,
        );
        if (!candidate) {
            continue;
        }
        const resolved =
            candidate.flags & SymbolFlags.Alias
                ? await project.checker.getAliasedSymbol(candidate)
                : candidate;
        if (resolved.id !== original.id) {
            continue;
        }

        // follow the export through its source module
        const declaration = await target.declarations[0]?.resolve(project);
        if (!declaration) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Unresolved module declaration: ${target.name}`,
            );
        }
        if (await isRuntimeExport(project, declaration.getSourceFile(), candidate, visited)) {
            return true;
        }
    }

    return false;
}

/** Locate a symbol in its source package and retain each declaration range. */
async function describeSymbol(
    symbol: TypeScriptSymbol,
    project: Project,
    root: string,
    modules: Map<string, ModuleDescription>,
    pending: Map<number, TypeScriptSymbol>,
): Promise<ModuleDescription["exports"][number]["symbol"]> {
    // resolve merged declaration locations
    const nodes = await Promise.all(
        symbol.declarations.map(async (handle) => {
            const node = await handle.resolve(project);
            if (!node) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Unresolved source declaration: ${symbol.name}`,
                );
            }

            return node;
        }),
    );
    if (!nodes.length) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `Export has no source declaration: ${symbol.name}`,
        );
    }

    // qualify the declaration within its source module
    const file = nodes[0].getSourceFile().fileName;
    const name =
        nodes[0].kind === SyntaxKind.SourceFile ? "*" : await declarationName(symbol, project);

    // identify compiler libraries independently of the installed native executable's platform
    if (await project.program.isSourceFileDefaultLibrary(nodes[0].getSourceFile())) {
        return {
            compiler: { name: "typescript", version },
            symbol: { module: basename(file), name },
        };
    }

    // resolve external declarations against their package metadata
    if (!contains(root, file) || relative(root, file).split(sep).includes("node_modules")) {
        const metadata = await modulePackage(dirname(file));
        let definition: string | undefined;

        // read optional Destack metadata once and preserve every other filesystem failure
        try {
            definition = await readFile(resolve(metadata.directory, "destack.json"), "utf8");
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
        }

        // retain stable identity for Destack packages and registry identity for npm packages
        const identity = { name: metadata.name, version: metadata.version };
        const owner =
            definition === undefined
                ? DependencyPackage.parse(identity)
                : Package.parse({ ...identity, id: JSON.parse(definition).id });

        return DependencySymbol.parse({
            package: owner,
            symbol: {
                module: relative(metadata.directory, file).split(sep).join("/"),
                name,
            },
        });
    }

    // retain the original declaration once when several modules reexport it
    const path = relative(root, file).split(sep).join("/");
    const module = modules.get(path);
    if (!module) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `Declaration belongs to an unknown module: ${path}`,
        );
    }
    pending.set(symbol.id, symbol);

    return { module: path, name };
}

/** Qualify a declaration through named namespaces and types within its module. */
async function declarationName(symbol: TypeScriptSymbol, project: Project): Promise<string> {
    // qualify nested declarations until the enclosing module
    const names = [symbol.name];
    let parent = await symbol.getParent();
    while (parent) {
        const declaration = await parent.declarations[0]?.resolve(project);
        if (!declaration) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Unresolved declaration parent: ${parent.name}`,
            );
        }
        if (declaration.kind === SyntaxKind.SourceFile) {
            break;
        }

        names.unshift(parent.name);
        parent = await parent.getParent();
    }

    return names.join(".");
}

/** Check whether a source path belongs to the package directory. */
function contains(directory: string, file: string): boolean {
    const path = relative(directory, file);

    return path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path);
}

/** Sort names by UTF-16 code units independently of the host locale. */
function compare(left: string, right: string): number {
    return left < right ? -1 : left > right ? 1 : 0;
}
