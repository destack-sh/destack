import { BuildError } from "../error/index.ts";
import { found, present } from "@destack/schema";
import { readFile } from "node:fs/promises";
import { basename, dirname } from "node:path";
import { version } from "typescript";
import {
    type NodeHandle,
    type Project,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
} from "typescript/unstable/async";
import {
    type Expression,
    isExportDeclaration,
    isImportDeclaration,
    isNoSubstitutionTemplateLiteral,
    isStringLiteral,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import {
    DependencySymbol,
    ModuleDescription,
    type SymbolDescription,
    type SymbolReference,
} from "@destack/package/code";
import { declaredName, SymbolInspector } from "./symbol.ts";
import { PackageFile } from "@destack/package/file";
import type { TestDeclaration } from "@destack/test/inspect";
import { inspectErrors } from "@destack/check/inspect";
import { collectTests } from "./test.ts";
import { collectDeclarations, ConstructorCatalog } from "./declaration.ts";
import type { ModulePackage } from "@destack/package/transform";
import { collectExamples, type ExampleDeclaration } from "./example.ts";
import { collectScenarios, type ScenarioDeclaration } from "./scenario.ts";
import type { DeclarationExport } from "../declaration/declaration.ts";
import { collectGlobals } from "./global.ts";
import { collectDirectories, type DirectoryReference } from "./directory.ts";
import { compareText } from "../build/serialization.ts";
import { isAuthored, relativePath } from "../source/dependency.ts";

/** The identifier `meta`, which every `import.meta` expression spells without escapes. */
const META = /(?<![\w$])meta(?![\w$])/u;

/** Compiler descriptions of modules and statically identified tests. */
export interface TypeScriptInspection {
    /** Exact source bytes checked against the compiler's parsed modules. */
    sources: Map<string, Uint8Array<ArrayBuffer>>;
    /** Literal directory references indexed by absolute source module path. */
    directories: Map<string, DirectoryReference[]>;
    /** Exported domain declarations to evaluate after static collection. */
    declarations: DeclarationExport[];
    /** Public declarations and exports. */
    modules: ModuleDescription[];
    /** Statically collected test and suite declarations. */
    tests: TestDeclaration[];
    /** Statically collected examples. */
    examples: ExampleDeclaration[];
    /** Statically collected scenarios. */
    scenarios: ScenarioDeclaration[];
}

/** Describe source modules, and collect the declarations of the modules the entries import, identifying the compiled package as its build releases it. */
export async function describeProject(
    project: Project,
    compiled: ModulePackage,
    entries: readonly string[],
): Promise<TypeScriptInspection> {
    // read the authored modules from the compiler file list
    const root = compiled.directory;
    const inspection = new ProjectInspection(project, compiled);
    const fileNames = [...(await project.program.getSourceFileNames())].toSorted();
    const authored = await readAuthored(project, root, fileNames);
    const runtime = await collectRuntimeFiles(project, entries, authored);

    // read every source module's directory references, and the tests and declarations of authored modules
    const collected = await Promise.all(
        fileNames
            .filter((file) => authored.has(file) || !isDeclarationPath(file))
            .map((file) => inspection.collect(file, authored.get(file), runtime.has(file))),
    );
    const directories = new Map<string, DirectoryReference[]>();
    for (const { file, references } of collected) {
        if (references.length) {
            directories.set(file, references);
        }
    }

    // register every authored module before following references between declarations
    for (const [file, source] of authored) {
        await inspection.register(file, source);
    }

    // describe each module's imports, globals, errors and exports, then the declarations they queue
    await Promise.all([...authored].map(([file, source]) => inspection.describe(file, source)));
    await inspection.describeQueued();

    // parse compiler descriptions before returning them to executable inspectors
    const modules = [...inspection.modules.values()].map((module) =>
        ModuleDescription.parse(module),
    );

    return {
        modules,
        sources: inspection.sources,
        tests: collected.flatMap((entry) => entry.tests),
        examples: inspection.examples,
        scenarios: inspection.scenarios,
        declarations: collected.flatMap((entry) => entry.declarations),
        directories,
    };
}

/** What one source module holds besides its description: directory references, tests and declarations. */
interface CollectedModule {
    /** The module's absolute path. */
    readonly file: string;
    /** The literal directory URLs the module reads. */
    readonly references: DirectoryReference[];
    /** The tests an authored module declares. */
    readonly tests: TestDeclaration[];
    /** The declarations an authored runtime module exports. */
    readonly declarations: DeclarationExport[];
}

/** One inspection of a compiler project: its modules, their bytes and the declarations they queue. */
class ProjectInspection {
    /** The compiler project. */
    readonly project: Project;
    /** The package directory. */
    readonly root: string;
    /** The inspected constructors and package identities. */
    readonly catalog: ConstructorCatalog;
    /** The described modules, by package-relative path. */
    readonly modules = new Map<string, ModuleDescription>();
    /** The exact source bytes, by package-relative path. */
    readonly sources = new Map<string, Uint8Array<ArrayBuffer>>();
    /** The symbol inspector of the project's snapshot. */
    readonly inspector: SymbolInspector;
    /** The examples of the described modules. */
    readonly examples: ExampleDeclaration[] = [];
    /** The scenarios of the described modules. */
    readonly scenarios: ScenarioDeclaration[] = [];
    /** The package's declarations queued for description, by symbol. */
    readonly #pending = new Map<number, TypeScriptSymbol>();
    /** The reference of each located symbol. */
    readonly #locations = new Map<number, Promise<SymbolReference>>();

    /** Start inspecting a project of a compiled package. */
    constructor(project: Project, compiled: ModulePackage) {
        // keep the project, the package's directory and its catalog, then inspect its symbols
        this.project = project;
        this.root = compiled.directory;
        this.catalog = new ConstructorCatalog(compiled);
        this.inspector = new SymbolInspector(
            project,
            (symbol) => this.reference(symbol),
            this.root,
        );
    }

    /** Locate a symbol once, queueing the package's declarations its exports and public types name. */
    reference(symbol: TypeScriptSymbol): Promise<SymbolReference> {
        let result = this.#locations.get(symbol.id);
        if (result === undefined) {
            result = describeSymbol(
                symbol,
                this.inspector,
                this.root,
                this.catalog,
                this.modules,
                this.#pending,
            );
            this.#locations.set(symbol.id, result);
        }

        return result;
    }

    /** Read a module's directory references, and the tests and runtime declarations of an authored one. */
    async collect(
        file: string,
        authored: SourceFile | undefined,
        isRuntime: boolean,
    ): Promise<CollectedModule> {
        // read a dependency module only when it can construct a directory URL from import.meta
        const { project } = this;
        const isMeta = authored === undefined && META.test(await readFile(file, "utf8"));
        const source = isMeta ? await project.program.getSourceFile(file) : authored;
        if (!source || source.isDeclarationFile) {
            return { file, references: [], tests: [], declarations: [] };
        }

        // retain literal directory references for package files
        const references = await collectDirectories(source, project);
        if (!authored) {
            return { file, references, tests: [], declarations: [] };
        }

        // describe the tests of authored modules and the declarations of runtime modules
        const owner = await this.catalog.locate(dirname(file));
        const path = relativePath(owner.directory, file);
        const tests = await collectTests(source, path, project);
        const declarations = isRuntime
            ? await collectDeclarations(source, path, this.inspector, owner, this.catalog, tests)
            : [];

        return { file, references, tests, declarations };
    }

    /** Register an authored module under its package-relative path, requiring its bytes to match the compiler snapshot. */
    async register(file: string, source: SourceFile): Promise<void> {
        // require source bytes to match the compiler snapshot
        const path = relativePath(this.root, file);
        const bytes = new Uint8Array(await readFile(file));
        const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
        if (text !== source.text) {
            throw new BuildError("INSPECTION_FAILED", `source changed during inspection: ${path}`);
        }

        // retain the module's bytes and an empty description
        this.sources.set(path, bytes);
        this.modules.set(path, {
            path,
            globals: [],
            errors: [],
            source: await PackageFile.describe(path, "text/plain", bytes),
            length: source.text.length,
            imports: [],
            symbols: [],
            exports: [],
        });
    }

    /** Describe a registered module's imports, globals, errors, exports, examples and scenarios together. */
    async describe(file: string, source: SourceFile): Promise<void> {
        // describe the module's imports, globals, errors, exports, examples and scenarios together
        const module = found(this.modules, relativePath(this.root, file));
        let examples: ExampleDeclaration[];
        let scenarios: ScenarioDeclaration[];
        [module.imports, module.globals, module.errors, module.exports, examples, scenarios] =
            await Promise.all([
                describeImports(source, this.inspector, this.root, this.catalog),
                collectGlobals(source, this.project, module.path, (symbol) =>
                    this.reference(symbol),
                ),
                inspectErrors(source, this.inspector),
                describeExports(source, module.path, this.inspector),
                collectExamples(source, module.path, this.inspector),
                collectScenarios(source, module.path, this.inspector),
            ]);
        this.examples.push(...examples);
        this.scenarios.push(...scenarios);
    }

    /** Describe the queued declarations in waves, as each wave queues the local types it names. */
    async describeQueued(): Promise<void> {
        // describe each wave's symbols together
        const described = new Set<number>();
        for (
            let wave = [...this.#pending.values()];
            wave.length;
            wave = [...this.#pending.values()].filter((symbol) => !described.has(symbol.id))
        ) {
            for (const symbol of wave) {
                described.add(symbol.id);
            }
            const descriptions = await Promise.all(
                wave.map((symbol) => this.#describeSymbol(symbol)),
            );
            for (const { module, symbol } of descriptions) {
                found(this.modules, module).symbols.push(symbol);
            }
        }

        // order symbols once every referenced symbol is described
        for (const module of this.modules.values()) {
            module.symbols.sort((left, right) => compareText(left.name, right.name));
        }
    }

    /** Describe a queued declaration of one of the package's modules. */
    async #describeSymbol(
        symbol: TypeScriptSymbol,
    ): Promise<{ module: string; symbol: SymbolDescription }> {
        const located = await this.reference(symbol);
        if (!("module" in located)) {
            throw new BuildError("INSPECTION_FAILED", "queued an external declaration");
        }

        return {
            module: located.module,
            symbol: await this.inspector.describe(symbol, located.name),
        };
    }
}

/** Read the compiler's source of each authored module. */
async function readAuthored(
    project: Project,
    root: string,
    fileNames: readonly string[],
): Promise<Map<string, SourceFile>> {
    const authored = fileNames.filter((file) => isAuthored(root, file));
    const sources = await Promise.all(
        authored.map(async (file) => {
            const source = await project.program.getSourceFile(file);
            if (!source) {
                throw new BuildError("INSPECTION_FAILED", `missing compiler source: ${file}`);
            }

            return [file, source] as const;
        }),
    );

    return new Map(sources);
}

/** Resolve a module's exports to their original declarations, by name. */
async function describeExports(
    source: SourceFile,
    path: string,
    inspector: SymbolInspector,
): Promise<ModuleDescription["exports"]> {
    // read the module's exports
    const project = inspector.project;
    const symbol = await project.checker.getSymbolAtLocation(source);
    if (!symbol) {
        return [];
    }
    const exports = await Promise.all(
        (await project.checker.getExportsOfModule(symbol)).map(async (exported) => {
            // resolve the export to its original declaration
            const original = await inspector.original(exported);
            if (await project.checker.isUnknownSymbol(original)) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `unresolved export: ${path}#${exported.name}`,
                );
            }

            // retain the public name and whether it exists at runtime
            const [located, isRuntime] = await Promise.all([
                inspector.reference(original),
                isRuntimeExport(inspector, source, exported, new Set()),
            ]);

            return { name: exported.name, symbol: located, isTypeOnly: !isRuntime };
        }),
    );

    return exports.toSorted((left, right) => compareText(left.name, right.name));
}

/** Collect the package's modules the entries import through imports and reexports kept at runtime. */
async function collectRuntimeFiles(
    project: Project,
    entries: readonly string[],
    sourceFiles: ReadonlyMap<string, SourceFile>,
): Promise<Set<string>> {
    // index the package's modules by the compiler's path
    const byPath = new Map([...sourceFiles.values()].map((source) => [source.path, source]));

    // follow imports from the entry modules
    const pending = await Promise.all(
        entries.map(async (entry) => {
            const source = await project.program.getSourceFile(entry);

            return found(byPath, present(source, `the entry module ${entry}`).path);
        }),
    );
    const visited = new Set(pending.map((source) => source.fileName));
    for (const source of pending) {
        // skip declaration files, which contribute no runtime modules
        if (source.isDeclarationFile) {
            continue;
        }

        // select specifiers of imports and reexports that remain after type erasure
        const specifiers = source.statements.flatMap((statement) => {
            if (isImportDeclaration(statement)) {
                return statement.importClause?.phaseModifier === SyntaxKind.TypeKeyword
                    ? []
                    : [statement.moduleSpecifier];
            } else if (isExportDeclaration(statement) && statement.moduleSpecifier) {
                return statement.isTypeOnly ? [] : [statement.moduleSpecifier];
            } else {
                return [];
            }
        });
        if (!specifiers.length) {
            continue;
        }

        // follow each of the package's modules once, leaving dependencies to their builds
        const modules = await project.checker.getSymbolAtLocation(specifiers);
        for (const module of modules) {
            const path = module?.declarations[0]?.path;
            const target = path === undefined ? undefined : byPath.get(path);
            if (target !== undefined && !visited.has(target.fileName)) {
                visited.add(target.fileName);
                pending.push(target);
            }
        }
    }

    return visited;
}

/** Follow named and star reexports while preserving explicit type-only declarations. */
async function isRuntimeExport(
    inspector: SymbolInspector,
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
    const original = await inspector.original(exported);
    if (!(original.flags & SymbolFlags.Value)) {
        return false;
    }

    // inspect declarations written in this module before following star exports
    for (const handle of exported.declarations) {
        if (handle.path === source.path && !isTypeOnly(await inspector.node(handle))) {
            return true;
        }
    }

    // find the value through a value-exporting star declaration
    for (const statement of source.statements) {
        if (
            !isExportDeclaration(statement) ||
            statement.exportClause ||
            statement.isTypeOnly ||
            !statement.moduleSpecifier
        ) {
            continue;
        }

        // follow the export through the star export module
        if (await isStarExport(inspector, statement.moduleSpecifier, original, visited, exported)) {
            return true;
        }
    }

    return false;
}

/** Follow one star export to a reexport of the same original symbol that carries a value. */
async function isStarExport(
    inspector: SymbolInspector,
    specifier: Expression,
    original: TypeScriptSymbol,
    visited: Set<string>,
    exported: TypeScriptSymbol,
): Promise<boolean> {
    // resolve the star export module
    const { checker } = inspector.project;
    const target = await checker.getSymbolAtLocation(specifier);
    if (!target) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `unresolved export module: ${specifier.getText()}`,
        );
    }

    // follow only exports that resolve to the same original symbol
    const candidate = (await checker.getExportsOfModule(target)).find(
        (entry) => entry.name === exported.name,
    );
    if (!candidate) {
        return false;
    }
    const resolved = await inspector.original(candidate);
    if (resolved.id !== original.id) {
        return false;
    }

    // follow the export through its source module
    const [module] = target.declarations;
    if (module === undefined) {
        throw new BuildError("INSPECTION_FAILED", `unresolved module declaration: ${target.name}`);
    }
    const declaration = await inspector.node(module);

    return isRuntimeExport(inspector, declaration.getSourceFile(), candidate, visited);
}

/** Report whether a declaration or one of its ancestors has a type-only modifier. */
function isTypeOnly(declaration: Node): boolean {
    for (let node = declaration; node.kind !== SyntaxKind.SourceFile; node = node.parent) {
        if ("isTypeOnly" in node && node.isTypeOnly === true) {
            return true;
        }
    }

    return false;
}

/** Locate a symbol in its source package, queueing the package's declarations. */
async function describeSymbol(
    symbol: TypeScriptSymbol,
    inspector: SymbolInspector,
    root: string,
    catalog: ConstructorCatalog,
    modules: Map<string, ModuleDescription>,
    pending: Map<number, TypeScriptSymbol>,
): Promise<SymbolReference> {
    // qualify the first declaration within its source module
    const [declaration] = symbol.declarations;
    if (declaration === undefined) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `export has no source declaration: ${symbol.name}`,
        );
    }
    const name = declaration.kind === SyntaxKind.SourceFile ? "*" : await declarationName(symbol);
    const reference = await locateSymbol(declaration, name, inspector, root, catalog);

    // retain the original declaration once when several modules reexport it
    if ("module" in reference) {
        if (!modules.has(reference.module)) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `declaration belongs to an unknown module: ${reference.module}`,
            );
        }
        pending.set(symbol.id, symbol);
    }

    return reference;
}

/** Resolve each import specifier of a module to the source module the compiler reads. */
async function describeImports(
    source: SourceFile,
    inspector: SymbolInspector,
    root: string,
    catalog: ConstructorCatalog,
): Promise<ModuleDescription["imports"]> {
    // read literal specifiers, including dynamic imports
    const specifiers = source.imports.map((entry) => {
        if (!isStringLiteral(entry) && !isNoSubstitutionTemplateLiteral(entry)) {
            throw new BuildError("INSPECTION_FAILED", `unsupported import in ${source.fileName}`);
        }

        return entry;
    });
    if (!specifiers.length) {
        return [];
    }

    // locate each resolved source file, leaving ambient module declarations unresolved
    const symbols = await inspector.project.checker.getSymbolAtLocation(specifiers);

    return await Promise.all(
        specifiers.map(async (entry, index) => {
            const declaration = symbols[index]?.declarations[0];
            if (declaration?.kind !== SyntaxKind.SourceFile) {
                return { specifier: entry.text };
            }

            return {
                specifier: entry.text,
                target: await locateSymbol(declaration, "*", inspector, root, catalog),
            };
        }),
    );
}

/** Locate a named declaration in the compiler libraries, a dependency or the package. */
async function locateSymbol(
    declaration: NodeHandle,
    name: string,
    inspector: SymbolInspector,
    root: string,
    catalog: ConstructorCatalog,
): Promise<SymbolReference> {
    // name compiler libraries by their lowercase file names, leaving their large sources unread
    const metadata = await inspector.project.program.getSourceFileMetadataByPath(declaration.path);
    if (metadata === undefined) {
        throw new BuildError("INSPECTION_FAILED", `undeclared source file: ${declaration.path}`);
    }
    if (metadata.isDefaultLibrary) {
        return {
            compiler: { name: "typescript", version },
            symbol: { module: basename(declaration.path), name },
        };
    }

    // read the declaring file's name, which the compiler keys in lowercase on case-insensitive systems
    const file = (await inspector.node(declaration)).getSourceFile().fileName;

    // name a dependency's module within its package
    if (!isAuthored(root, file)) {
        const location = await catalog.locate(dirname(file));

        return DependencySymbol.parse({
            package: await catalog.owner(location),
            symbol: { module: relativePath(location.directory, file), name },
        });
    }

    // name the package's module by its package-relative path
    return { module: relativePath(root, file), name };
}

/** Qualify a declaration through named namespaces and types within its module. */
async function declarationName(symbol: TypeScriptSymbol): Promise<string> {
    // qualify nested declarations until the enclosing module
    const names = [declaredName(symbol)];
    let parent = await symbol.getParent();
    while (parent) {
        const [declaration] = parent.declarations;
        if (declaration === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `unresolved declaration parent: ${parent.name}`,
            );
        }
        if (declaration.kind === SyntaxKind.SourceFile) {
            break;
        }

        names.unshift(declaredName(parent));
        parent = await parent.getParent();
    }

    return names.join(".");
}

/** Report whether a file holds declarations only, by the compiler's naming rule for `.d.ts` files. */
export function isDeclarationPath(path: string): boolean {
    return /\.d\.[cm]?ts$/u.test(path) || (path.endsWith(".ts") && basename(path).includes(".d."));
}
