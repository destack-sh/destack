import { BuildError } from "../error/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { found, nth, present, Text } from "@destack/schema";
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
import type { TestDeclaration } from "./test.ts";
import { inspectErrors } from "@destack/check/diagnostic";
import { collectTests } from "./test.ts";
import { collectDeclarations, ConstructorCatalog } from "./declaration.ts";
import type { PackageSource } from "../source/index.ts";
import { ExampleDeclaration } from "./example.ts";
import { ScenarioDeclaration } from "./scenario.ts";
import type { DeclarationExport } from "../declaration/declaration.ts";
import { collectGlobals } from "./global.ts";
import { DirectoryReference } from "./directory.ts";
import { isAuthored, relativePath } from "../source/dependency.ts";

/** The identifier `meta`, which every `import.meta` expression spells without escapes. */
const META = /(?<![\w$])meta(?![\w$])/u;

/** A module's exports by name: each export, its original and whether it exists at runtime. */
type ExportTable = ReadonlyMap<
    string,
    {
        /** The exported symbol. */
        readonly symbol: TypeScriptSymbol;
        /** The symbol it refers to past aliases. */
        readonly original: TypeScriptSymbol;
        /** Whether a value of it exists at runtime. */
        readonly isRuntime: boolean;
    }
>;

/** Compiler descriptions of modules and statically identified tests. */
export interface TypeScriptInspection {
    /** The files of the program, by absolute path. */
    files: readonly string[];
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
    /** Statically collected examples, their properties left to evaluation. */
    examples: Omit<ExampleDeclaration, "properties">[];
    /** Statically collected scenarios. */
    scenarios: ScenarioDeclaration[];
}

/** How much an inspection reads: what a build needs, or every symbol for the code index. */
export type InspectionDepth = "build" | "index";

/** The modules and declarations the compiler reads from a package's source. */
export const TypeScriptInspection = {
    /** Read a project's source modules to a depth, adopting another program's it also read. */
    read: inspectProject,
};

/** Inspect a project's modules to a depth, every symbol their exports lead to at the index's. */
async function inspectProject(
    project: Project,
    compiled: Pick<PackageSource, "directory" | "versions" | "entries">,
    depth: InspectionDepth,
    described?: Promise<TypeScriptInspection>,
): Promise<TypeScriptInspection> {
    // read the program's entries
    const { entries } = compiled;
    // read the authored modules from the compiler file list
    const root = compiled.directory;
    const inspection = new ProjectInspection(project, compiled, depth);
    const fileNames = [...(await project.program.getSourceFileNames())].toSorted();
    const authored = await readAuthored(project, root, fileNames);
    const runtime = await collectRuntimeFiles(project, entries, authored);

    // read the modules' directory references, and the authored modules' tests and declarations
    const collected = await Promise.all(
        fileNames
            .filter((file) => authored.has(file) || !isDeclarationPath(file))
            .map((file) => inspection.collect(file, authored.get(file), runtime.has(file))),
    );

    // leave test modules' examples and scenarios to their runner, as only it can import them
    const tested = new Set(
        collected.flatMap((entry) => (entry.tests.length > 0 ? [entry.file] : [])),
    );

    // index the directory references of each module reading any
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

    // describe each module and the declarations it queues, once per package
    if (described === undefined) {
        await Promise.all(
            [...authored].map(([file, source]) =>
                inspection.describe(file, source, tested.has(file)),
            ),
        );
        if (depth === "index") {
            await inspection.describeQueued();
        }
    }
    // describe what each runtime decides, taking the rest from another runtime's description
    else {
        await Promise.all(
            [...authored].map(([file, source]) => inspection.describeRuntime(file, source)),
        );
        inspection.adopt(await described);
    }

    // parse compiler descriptions before returning them to executable inspectors
    const modules = [...inspection.modules.values()].map((module) =>
        ModuleDescription.parse(module),
    );

    return {
        files: fileNames,
        modules,
        sources: inspection.sources,
        tests: collected.flatMap((entry) => entry.tests),
        examples: inspection.examples,
        scenarios: inspection.scenarios,
        declarations: collected.flatMap((entry) => entry.declarations),
        directories,
    };
}

/** What a source module has beside its description: directory references, tests, declarations. */
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

/** One inspection of a compiler project: its modules, bytes and queued declarations. */
class ProjectInspection {
    /** The export table of each module, by the compiler's path, built once per inspection. */
    readonly #exports = new Map<string, Promise<ExportTable>>();
    /** The modules each module's value star exports name, by the compiler's path. */
    readonly #stars = new Map<string, Promise<readonly SourceFile[]>>();
    /** The modules whose star exports lead to no cycle, by the compiler's path. */
    readonly #acyclic = new Set<string>();
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
    readonly examples: Omit<ExampleDeclaration, "properties">[] = [];
    /** The scenarios of the described modules. */
    readonly scenarios: ScenarioDeclaration[] = [];
    /** The package's declarations queued for description, by symbol. */
    readonly #pending = new Map<number, TypeScriptSymbol>();
    /** The reference of each located symbol. */
    readonly #locations = new Map<number, Promise<SymbolReference>>();
    /** How much the inspection reads: what a build needs, or every symbol the exports lead to. */
    readonly depth: InspectionDepth;

    /** Start inspecting a project of a compiled package, completely or as a build describes it. */
    constructor(
        project: Project,
        compiled: Pick<PackageSource, "directory" | "versions">,
        depth: InspectionDepth,
    ) {
        // store the project, the package's directory and its catalog, then inspect its symbols
        this.project = project;
        this.root = compiled.directory;
        this.catalog = new ConstructorCatalog(compiled.versions);
        this.depth = depth;
        this.inspector = new SymbolInspector(
            project,
            (symbol) => this.reference(symbol),
            this.root,
        );
    }

    /** Locate a symbol once by its module and name. */
    locate(symbol: TypeScriptSymbol): Promise<SymbolReference> {
        let result = this.#locations.get(symbol.id);
        if (result === undefined) {
            result = locateDeclaration(
                symbol,
                this.inspector,
                this.root,
                this.catalog,
                this.modules,
            );
            this.#locations.set(symbol.id, result);
        }

        return result;
    }

    /** Locate a symbol, queueing a declaration of the package for description when complete. */
    async reference(symbol: TypeScriptSymbol): Promise<SymbolReference> {
        const located = await this.locate(symbol);
        if (this.depth === "index" && "module" in located) {
            this.#pending.set(symbol.id, symbol);
        }

        return located;
    }

    /** Read a module's directory references, and an authored one's tests and declarations. */
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
        const references = await DirectoryReference.collect(source, project);
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

    /** Register an authored module by package path, requiring bytes equal to the snapshot. */
    async register(file: string, source: SourceFile): Promise<void> {
        // require source bytes to match the compiler snapshot
        const path = relativePath(this.root, file);
        const bytes = new Uint8Array(await readFile(file));
        const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
        if (text !== source.text) {
            throw BuildError.of("INSPECTION_FAILED", `source changed during inspection: ${path}`);
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

    /** Build a module's export table once, folding in the tables of the modules it star-exports. */
    exports(source: SourceFile): Promise<ExportTable> {
        let table = this.#exports.get(source.path);
        if (table === undefined) {
            table = this.#requireAcyclic(source, [source]).then(() => this.#exportTable(source));
            this.#exports.set(source.path, table);
        }

        return table;
    }

    /** Read the modules a module's value star exports name, once, refusing an unresolved one. */
    #starred(source: SourceFile): Promise<readonly SourceFile[]> {
        let starred = this.#stars.get(source.path);
        if (starred === undefined) {
            starred = this.#resolveStars(source);
            this.#stars.set(source.path, starred);
        }

        return starred;
    }

    /** Resolve the modules a module's value star exports name. */
    async #resolveStars(source: SourceFile): Promise<readonly SourceFile[]> {
        // read the module specifiers of its value star exports
        const specifiers = source.statements.flatMap((statement) =>
            isExportDeclaration(statement) &&
            !statement.exportClause &&
            !statement.isTypeOnly &&
            statement.moduleSpecifier !== undefined
                ? [statement.moduleSpecifier]
                : [],
        );
        if (specifiers.length === 0) {
            return [];
        }

        // resolve each to its module, refusing a star export of no module
        const targets = await this.project.checker.getSymbolAtLocation(specifiers);

        return Promise.all(
            targets.map(async (target, index) => {
                const declaration = target?.declarations[0];
                if (declaration === undefined) {
                    const text = nth(specifiers, index).getText();
                    throw BuildError.of("INSPECTION_FAILED", `unresolved export module: ${text}`);
                }

                return (await this.inspector.node(declaration)).getSourceFile();
            }),
        );
    }

    /** Refuse value star exports that lead back into a module on the chain, as a cycle. */
    async #requireAcyclic(source: SourceFile, chain: readonly SourceFile[]): Promise<void> {
        // pass a module whose star exports were checked before
        if (this.#acyclic.has(source.path)) {
            return;
        }

        // refuse a star export back into the chain, then check each target's exports
        for (const starred of await this.#starred(source)) {
            const start = chain.findIndex((each) => each.path === starred.path);
            if (start >= 0) {
                // list the cycle from its first path, the same from whichever module it starts
                const cycle = chain
                    .slice(start)
                    .map((each) => relativePath(this.root, each.fileName));
                const first = cycle.indexOf(
                    nth(
                        cycle.toSorted((left, right) => Text.compare(left, right)),
                        0,
                    ),
                );
                const ordered = [...cycle.slice(first), ...cycle.slice(0, first)];
                const paths = [...ordered, nth(ordered, 0)].join(" -> ");
                throw BuildError.of("INSPECTION_FAILED", `export * cycle: ${paths}`);
            }
            await this.#requireAcyclic(starred, [...chain, starred]);
        }
        this.#acyclic.add(source.path);
    }

    /** Read a module's exports by name, each with its original and whether it exists at runtime. */
    async #exportTable(source: SourceFile): Promise<ExportTable> {
        // read the module's exports, none for a module without a symbol
        const { checker } = this.project;
        const module = await checker.getSymbolAtLocation(source);
        const exported = module === undefined ? [] : await checker.getExportsOfModule(module);

        // read the table of each module a star export lists, which no cycle leads to
        const stars = await Promise.all(
            (await this.#starred(source)).map((starred) => this.exports(starred)),
        );

        // add each export with its original
        const entries = await Promise.all(
            exported.map(async (symbol) => {
                // decide whether the export exists at runtime here or through a star export
                const original = await this.inspector.original(symbol);
                const isValue = (original.flags & SymbolFlags.Value) !== 0;
                const isDeclared = await this.#isDeclaredValue(symbol, source);
                const isStarred = stars.some((star) => {
                    // match the star export's entry of the same original
                    const entry = star.get(symbol.name);

                    return entry?.isRuntime === true && entry.original.id === original.id;
                });
                const entry = { symbol, original, isRuntime: isValue && (isDeclared || isStarred) };

                return [symbol.name, entry] as const;
            }),
        );

        return new Map(entries);
    }

    /** Report whether a module declares an export without a type-only modifier. */
    async #isDeclaredValue(symbol: TypeScriptSymbol, source: SourceFile): Promise<boolean> {
        const local = symbol.declarations.filter((handle) => handle.path === source.path);
        const nodes = await Promise.all(local.map((handle) => this.inspector.node(handle)));

        return nodes.some((node) => !isTypeOnly(node));
    }

    /** Describe a module's exports by name, each by its original declaration. */
    async describeExports(source: SourceFile, path: string): Promise<ModuleDescription["exports"]> {
        // resolve each export to its located original declaration, refusing an unresolved one
        const table = await this.exports(source);
        const exports = await Promise.all(
            [...table.values()].map(async ({ symbol, original, isRuntime }) => {
                if (await this.project.checker.isUnknownSymbol(original)) {
                    throw BuildError.of(
                        "INSPECTION_FAILED",
                        `unresolved export: ${path}#${symbol.name}`,
                    );
                }

                return {
                    name: symbol.name,
                    symbol: await this.inspector.reference(original),
                    isTypeOnly: !isRuntime,
                };
            }),
        );

        return exports.toSorted((left, right) => Text.compare(left.name, right.name));
    }

    /** Describe a module's imports, globals, errors, exports, and outside tests its examples. */
    async describe(file: string, source: SourceFile, isTest: boolean): Promise<void> {
        // describe the module's imports, globals, errors, exports, examples and scenarios together
        const module = found(this.modules, relativePath(this.root, file));
        let examples: Omit<ExampleDeclaration, "properties">[];
        let scenarios: ScenarioDeclaration[];
        [module.imports, module.globals, module.errors, module.exports, examples, scenarios] =
            await Promise.all([
                describeImports(source, this.inspector, this.root, this.catalog),
                collectGlobals(source, this.project, module.path, (symbol) =>
                    this.reference(symbol),
                ),
                this.depth === "index" ? inspectErrors(source, this.inspector) : [],
                this.describeExports(source, module.path),
                isTest ? [] : ExampleDeclaration.collect(source, module.path, this.inspector),
                isTest ? [] : ScenarioDeclaration.collect(source, module.path, this.inspector),
            ]);
        this.examples.push(...examples);
        this.scenarios.push(...scenarios);
    }

    /** Describe a module's imports, globals and errors, and the exports runtimes agree on. */
    async describeRuntime(file: string, source: SourceFile): Promise<void> {
        const module = found(this.modules, relativePath(this.root, file));
        [module.imports, module.globals, module.errors, module.exports] = await Promise.all([
            describeImports(source, this.inspector, this.root, this.catalog),
            collectGlobals(source, this.project, module.path, (symbol) => this.reference(symbol)),
            this.depth === "index" ? inspectErrors(source, this.inspector) : [],
            this.describeExports(source, module.path),
        ]);
    }

    /** Take the symbols, examples and scenarios another runtime's program describes. */
    adopt(described: TypeScriptInspection): void {
        // require the modules among the described ones, which read the package's whole source
        const modules = new Map(described.modules.map((module) => [module.path, module]));
        const unknown = [...this.modules.keys()].filter((path) => !modules.has(path));
        if (unknown.length > 0) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `the runtime programs read modules that the first one does not have: ${unknown.join(", ")}`,
            );
        }

        // require each module's bytes and exports to agree, then take its symbols
        for (const [path, module] of this.modules) {
            const other = found(modules, path);
            const exported = new Set(other.exports.map((each) => stringifyInspection(each)));
            const differing = module.exports
                .filter((each) => !exported.has(stringifyInspection(each)))
                .map((each) => each.name);
            if (other.source.digest !== module.source.digest) {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `${path} changed between runtime programs`,
                );
            } else if (differing.length > 0 || other.exports.length !== module.exports.length) {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `the runtime programs disagree on the exports of ${path}: ${differing.join(", ")}`,
                );
            }
            module.symbols.push(...other.symbols);
        }

        // take the examples and scenarios
        this.examples.push(...described.examples);
        this.scenarios.push(...described.scenarios);
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
            module.symbols.sort((left, right) => Text.compare(left.name, right.name));
        }
    }

    /** Describe a queued declaration of one of the package's modules. */
    async #describeSymbol(
        symbol: TypeScriptSymbol,
    ): Promise<{ module: string; symbol: SymbolDescription }> {
        const located = await this.reference(symbol);
        if (!("module" in located)) {
            throw BuildError.of("INVARIANT_VIOLATED", "queued an external declaration");
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
                throw BuildError.of("INSPECTION_FAILED", `missing compiler source: ${file}`);
            }

            return [file, source] as const;
        }),
    );

    return new Map(sources);
}

/** Collect the package modules the entries import through runtime imports and reexports. */
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

/** Report whether a declaration or one of its ancestors has a type-only modifier. */
function isTypeOnly(declaration: Node): boolean {
    for (let node = declaration; node.kind !== SyntaxKind.SourceFile; node = node.parent) {
        if ("isTypeOnly" in node && node.isTypeOnly === true) {
            return true;
        }
    }

    return false;
}

/** Locate a symbol in its source package, refusing a declaration of no known module. */
async function locateDeclaration(
    symbol: TypeScriptSymbol,
    inspector: SymbolInspector,
    root: string,
    catalog: ConstructorCatalog,
    modules: Map<string, ModuleDescription>,
): Promise<SymbolReference> {
    // qualify the first declaration within its source module
    const [declaration] = symbol.declarations;
    if (declaration === undefined) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `export has no source declaration: ${symbol.name}`,
        );
    }
    const name = declaration.kind === SyntaxKind.SourceFile ? "*" : await declarationName(symbol);
    const reference = await locateSymbol(declaration, name, inspector, root, catalog);

    // require a declaration of the package to belong to one of its modules
    if ("module" in reference && !modules.has(reference.module)) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `declaration belongs to an unknown module: ${reference.module}`,
        );
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
            throw BuildError.of("INSPECTION_FAILED", `unsupported import in ${source.fileName}`);
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
        throw BuildError.of("INSPECTION_FAILED", `undeclared source file: ${declaration.path}`);
    }
    if (metadata.isDefaultLibrary) {
        return {
            compiler: { name: "typescript", version },
            symbol: { module: basename(declaration.path), name },
        };
    }

    // read the declaring file's path as the compiler keys it
    const file = await inspector.file(declaration);

    // key a dependency's module within its package
    if (!isAuthored(root, file)) {
        const location = await catalog.locate(dirname(file));

        return DependencySymbol.parse({
            package: await catalog.owner(location),
            symbol: { module: relativePath(location.directory, file), name },
        });
    }

    // key the package's module by its package-relative path
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
            throw BuildError.of(
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

/** Report whether a file has declarations only, by the compiler's `.d.ts` rule. */
export function isDeclarationPath(path: string): boolean {
    return /\.d\.[cm]?ts$/u.test(path) || (path.endsWith(".ts") && basename(path).includes(".d."));
}
