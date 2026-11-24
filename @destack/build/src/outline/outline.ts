import { dirname, join, relative } from "node:path";
import { Project } from "../source/index.ts";
import { ResolverFactory } from "rolldown/experimental";
import { type ESTree, parseSync } from "rolldown/utils";
import type { Environment } from "@destack/environment";
import { graph } from "@destack/package";
import type { SourceRange } from "@destack/package/source";
import { Digest, schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import { collectDefinitions, type Definition } from "./definition.ts";
import { children, qualifiedName } from "./node.ts";
import { collectTests, type TestDeclaration } from "./test.ts";

/** The longest signature, TypeScript's defaultMaximumTruncationLength. */
const SIGNATURE_LENGTH = 160;

/** The extensions a module specifier resolves to, sources before builds. */
const EXTENSIONS = [".ts", ".tsx", ".d.ts", ".mts", ".d.mts", ".js", ".jsx", ".mjs"];

/** The files a JavaScript specifier names, sources and declarations first, as in TypeScript. */
const EXTENSION_ALIASES = {
    ".js": [".ts", ".tsx", ".d.ts", ".js"],
    ".jsx": [".tsx", ".d.ts", ".jsx"],
    ".mjs": [".mts", ".d.mts", ".mjs"],
    ".cjs": [".cts", ".d.cts", ".cjs"],
};

/** The extensions of script modules, which hold code that runs. */
const SCRIPT_EXTENSION = /\.[cm]?[jt]sx?$/u;

/** The conditions a module specifier resolves under, types before code. */
const CONDITIONS = ["types", "import", "bun", "default"];

/** The fields of a package.json and a destack.json that name a package. */
const PackageNames = schema.looseObject({
    /** The npm name. */
    name: schema.string().min(1).exactOptional(),
    /** The Destack identifier. */
    id: schema.string().min(1).exactOptional(),
});

/** The symbol kind of each declaration node. */
const SYMBOL_KINDS: Readonly<Record<string, graph.SymbolDeclaration["kind"]>> = {
    FunctionDeclaration: "function",
    TSDeclareFunction: "function",
    ClassDeclaration: "class",
    TSInterfaceDeclaration: "interface",
    TSTypeAliasDeclaration: "typeAlias",
    TSEnumDeclaration: "enum",
    TSEnumMember: "enumMember",
    TSModuleDeclaration: "namespace",
    VariableDeclarator: "variable",
    ExportDefaultDeclaration: "variable",
    TSExportAssignment: "variable",
    TSPropertySignature: "property",
    PropertyDefinition: "property",
    AccessorProperty: "property",
    TSMethodSignature: "method",
    TSCallSignatureDeclaration: "method",
    TSConstructSignatureDeclaration: "method",
    TSIndexSignature: "method",
};

/** The name a module's `export =` value binds when it is no declared name. */
const ASSIGNED = "default";

/** A use of a name in a module: a call or a type reference, at a range, to a symbol. */
export interface OutlineUse {
    /** How it uses the symbol. */
    readonly kind: "calls" | "references";
    /** Where it sits in the module. */
    readonly source: SourceRange;
    /** The symbol it uses. */
    readonly to: graph.Moniker;
}

/** A module read from its syntax: its symbols, imports, exports and the names it uses. */
export interface Outline {
    /** The package-relative module path. */
    readonly path: string;
    /** The digest of the source bytes. */
    readonly digest: Digest;
    /** Each import specifier with the module and file it resolves to, absent when it does not. */
    readonly imports: readonly {
        readonly specifier: string;
        readonly target?: graph.Moniker;
        readonly file?: string;
    }[];
    /** The exports by name, each resolved to its original symbol. */
    readonly exports: graph.Module["exports"];
    /** The declared symbols and their members, each with every declaration of it. */
    readonly symbols: readonly graph.Symbol[];
    /** The heritage edges of its classes and interfaces. */
    readonly heritage: readonly graph.Edge[];
    /** The calls and type references to symbols it resolves. */
    readonly uses: readonly OutlineUse[];
    /** The tests and suites it declares. */
    readonly tests: readonly TestDeclaration[];
    /** The examples and scenarios it defines at its exported constants. */
    readonly definitions: readonly Definition[];
}

/** One declaration of a name: its node, whether it is a type, and a variable's keyword. */
interface Binding {
    /** The declaring node. */
    readonly node: ESTree.Node;
    /** Whether it declares a type and no value. */
    readonly isType: boolean;
    /** The keyword declaring a variable, absent for any other declaration. */
    readonly keyword?: string;
}

/** An export: a local name, another module's name, all its names, or its `export =` value. */
type Export =
    | {
          readonly kind: "local";
          readonly name: string;
          readonly local: string;
          readonly isType: boolean;
      }
    | {
          readonly kind: "from";
          readonly name: string;
          readonly file: string;
          readonly imported: string;
          readonly isType: boolean;
      }
    | { readonly kind: "all"; readonly file: string }
    | { readonly kind: "assigned"; readonly local: string };

/** The export providing a name: a module's local name, or another module's name or namespace. */
type Provided =
    | {
          readonly kind: "local";
          readonly file: string;
          readonly local: string;
          readonly isType: boolean;
      }
    | {
          readonly kind: "from";
          readonly target: string;
          readonly imported: string;
          readonly isType: boolean;
      };

/** A module's exports by name, and whether a star cycle left any out. */
interface ExportTable {
    /** The export providing each name, in statement order. */
    readonly provided: ReadonlyMap<string, Provided>;
    /** Whether every star export's names were read, none cut at a cycle. */
    readonly isComplete: boolean;
}

/** A comment of a module. */
interface Comment {
    /** Whether it is a line or a block comment. */
    readonly type: string;
    /** Its text between the delimiters. */
    readonly value: string;
    /** Where it starts. */
    readonly start: number;
    /** Where it ends. */
    readonly end: number;
}

/** The names a module's statements bind: its imports, aliases, declarations and exports. */
interface Names {
    /** The local name each import binds, to its module and imported name, `*` for a namespace. */
    readonly imports: Map<string, { readonly file: string; readonly name: string }>;
    /** The local name each `import A = B.C` binds, to the name it stands for by its parts. */
    readonly aliases: Map<string, string[]>;
    /** Each specifier with the file it resolves to. */
    readonly specifiers: { readonly specifier: string; readonly file?: string }[];
    /** The top-level declarations by name, in source order. */
    readonly declarations: Map<string, Binding[]>;
    /** The exports. */
    readonly exports: Export[];
}

/** What resolution reads of a module, kept while its bytes hold: its moniker, digest and names. */
export interface OutlineModule extends Omit<Names, "declarations"> {
    /** The module's moniker. */
    readonly moniker: graph.Moniker;
    /** The digest of its source bytes. */
    readonly digest: Digest;
    /** Whether each top-level name declares only types. */
    readonly declared: ReadonlyMap<string, boolean>;
}

/** A module's syntax, held only while it is read: its program, text, comments and declarations. */
interface Source {
    /** The module. */
    readonly module: OutlineModule;
    /** The module's program. */
    readonly program: ESTree.Program;
    /** The module's source text. */
    readonly text: string;
    /** The module's comments. */
    readonly comments: readonly Comment[];
    /** The top-level declarations by name, in source order. */
    readonly declarations: ReadonlyMap<string, readonly Binding[]>;
}

/** A call or type reference by name, at a range of its module. */
interface Use {
    /** How it uses the name. */
    readonly kind: "calls" | "references";
    /** The name, by its parts. */
    readonly name: string[];
    /** Where it starts. */
    readonly start: number;
    /** Where it ends. */
    readonly end: number;
}

/** Outlines modules from their syntax, each file parsed once, its packages read once. */
export class Outliner {
    /** The files the modules are read from and the crypto digesting them. */
    readonly #environment: Environment<"files">;
    /** The resolver of import specifiers. */
    readonly #resolver = new ResolverFactory({
        extensions: EXTENSIONS,
        conditionNames: CONDITIONS,
        mainFields: ["types", "module", "main"],
        extensionAlias: EXTENSION_ALIASES,
    });
    /** Each file's module this outliner read, by absolute path. */
    readonly #modules = new Map<string, Promise<OutlineModule>>();
    /** The modules earlier outliners read, by absolute path, reused while a file's digest holds. */
    readonly #kept: Map<string, OutlineModule>;
    /** Each module's complete export table, by absolute path. */
    readonly #tables = new Map<string, ExportTable>();
    /** The modules whose star exports were checked to lead back into no module of the chain. */
    readonly #acyclic = new Set<string>();
    /** The package each directory belongs to: its root and its moniker prefix. */
    readonly #packages = new Map<string, Promise<{ readonly root: string; readonly id: string }>>();

    /** Outline modules from files, reusing and adding to the modules earlier outliners kept. */
    constructor(environment: Environment<"files">, kept: Map<string, OutlineModule>) {
        this.#environment = environment;
        this.#kept = kept;
    }

    /** List a package's own script modules its entries import, directly or through each other. */
    async modules(directory: string, entries: readonly string[]): Promise<string[]> {
        // follow each specifier into a module of the package, each module once
        const imported = new Set<string>();
        const pending = [...entries];
        for (const file of pending) {
            if (imported.has(file)) {
                continue;
            }
            imported.add(file);
            for (const { file: target } of (await this.#parse(file)).specifiers) {
                const isOwn =
                    target !== undefined &&
                    target.startsWith(`${directory}/`) &&
                    !target.includes("/node_modules/") &&
                    SCRIPT_EXTENSION.test(target) &&
                    !Project.isDeclaration(target);
                if (isOwn) {
                    pending.push(target);
                }
            }
        }

        return [...imported];
    }

    /** Outline one module: its symbols, imports, exports and the names it uses. */
    async outline(file: string): Promise<Outline> {
        // read the module's syntax and its exports
        const source = await this.#source(file);
        const { module } = source;
        const path = graph.Moniker.module(module.moniker);
        const exported = await this.#exportsOf(file);
        const local = new Set(
            module.exports.flatMap((each) => (each.kind === "local" ? [each.local] : [])),
        );

        // describe each top-level name and its members with every declaration, and what each extends
        const symbols: graph.Symbol[] = [];
        const heritage: graph.Edge[] = [];
        for (const [name, bindings] of source.declarations) {
            const moniker = this.#symbol(module, name);
            const isExported = local.has(name);
            symbols.push(describe(source, moniker, bindings, isExported));
            for (const [member, nested] of membersOf(bindings)) {
                const memberMoniker = this.#symbol(module, name, member);
                symbols.push(describe(source, memberMoniker, nested, isExported));
            }
            for (const target of bindings.flatMap((binding) => heritageOf(binding.node))) {
                const to = await this.#resolve(module, target);
                if (to !== undefined) {
                    heritage.push({ from: moniker, to, kind: "implements" });
                }
            }
        }

        // resolve each call and type reference to a symbol, leaving globals and unresolved names
        const uses: OutlineUse[] = [];
        for (const use of usesOf(source.program)) {
            const to = await this.#resolve(module, use.name);
            if (to !== undefined) {
                const range = { file: path, start: use.start, end: use.end };
                uses.push({ kind: use.kind, source: range, to });
            }
        }

        // list the imports, and the exports by their original symbols
        const imports = await Promise.all(
            module.specifiers.map(async ({ specifier, file: target }) => ({
                specifier,
                ...(target === undefined
                    ? {}
                    : { target: await this.#module(target), file: target }),
            })),
        );
        const exports = await Promise.all(
            [...exported].map(async ([name, symbol]) => ({
                name,
                symbol,
                isTypeOnly: await this.#isType(file, name, new Set()),
            })),
        );

        // read its tests, and outside test modules, whose runners import their own, its examples and scenarios
        const tests = collectTests(source.program, path);
        const resolve = (name: readonly string[]) => this.#resolve(module, name);
        const definitions =
            tests.length > 0 ? [] : collectDefinitions(source.program, path, local, resolve);

        return {
            path,
            digest: module.digest,
            imports,
            exports,
            symbols,
            heritage,
            uses,
            tests,
            definitions,
        };
    }

    /** Resolve a name a module uses: an alias, its own declaration, else an import. */
    async #resolve(
        module: OutlineModule,
        name: readonly string[],
        visiting: ReadonlySet<string> = new Set(),
    ): Promise<graph.Moniker | undefined> {
        // take the name an alias stands for, else the module's own declaration
        const [head, member] = name;
        const alias = head === undefined ? undefined : module.aliases.get(head);
        if (head === undefined) {
            return undefined;
        } else if (alias !== undefined) {
            return await this.#resolve(module, [...alias, ...name.slice(1)], visiting);
        } else if (module.declared.has(head)) {
            return this.#symbol(module, head);
        }

        // follow an import: a named export, or a namespace import's member
        const imported = module.imports.get(head);
        if (imported === undefined) {
            return undefined;
        } else if (imported.name === "*") {
            return member === undefined
                ? await this.#module(imported.file)
                : await this.#original(imported.file, member, visiting);
        }

        // follow a member of a namespace the module re-exports, else the export itself
        const namespace = await this.#namespace(imported.file, imported.name, visiting);

        return namespace !== undefined && member !== undefined
            ? await this.#original(namespace, member, visiting)
            : await this.#original(imported.file, imported.name, visiting);
    }

    /** Follow one export of a module to its original symbol, absent past a cycle or for none. */
    async #original(
        file: string,
        name: string,
        visiting: ReadonlySet<string>,
    ): Promise<graph.Moniker | undefined> {
        // stop at an export the walk already follows
        const key = `${file}#${name}`;
        if (visiting.has(key)) {
            return undefined;
        }
        const inner = new Set([...visiting, key]);

        // follow the export providing the name: a local declaration, a namespace, or another module's
        const provided = (await this.#table(file, new Set())).provided.get(name);
        if (provided?.kind === "local") {
            return await this.#resolve(await this.#parse(provided.file), [provided.local], inner);
        } else if (provided?.kind === "from") {
            return provided.imported === "*"
                ? await this.#module(provided.target)
                : await this.#original(provided.target, provided.imported, inner);
        }

        // take a member of the module's `export =` value as its name
        const module = await this.#parse(file);
        const assigned = module.exports.find((each) => each.kind === "assigned");
        if (assigned === undefined) {
            return undefined;
        } else if (module.declared.has(assigned.local)) {
            return this.#symbol(module, assigned.local, name);
        }

        return await this.#resolve(module, [assigned.local, name], inner);
    }

    /** Report whether an export of a module is only a type, following it to its declarations. */
    async #isType(file: string, name: string, visiting: ReadonlySet<string>): Promise<boolean> {
        // take a value past a cycle, as the export resolves no type there
        const key = `${file}#${name}`;
        if (visiting.has(key)) {
            return false;
        }
        const inner = new Set([...visiting, key]);

        // take an export marked as a type, else follow it to its declarations
        const provided = (await this.#table(file, new Set())).provided.get(name);
        if (provided === undefined) {
            return false;
        } else if (provided.isType) {
            return true;
        } else if (provided.kind === "local") {
            return await this.#isLocalType(await this.#parse(provided.file), provided.local, inner);
        }

        return (
            provided.imported !== "*" &&
            (await this.#isType(provided.target, provided.imported, inner))
        );
    }

    /** Report whether a module's local name declares only types, following an import of it. */
    async #isLocalType(
        module: OutlineModule,
        name: string,
        visiting: ReadonlySet<string>,
    ): Promise<boolean> {
        // take its declarations, else follow its import
        const isType = module.declared.get(name);
        const imported = module.imports.get(name);
        if (isType !== undefined) {
            return isType;
        } else if (imported !== undefined && imported.name !== "*") {
            return await this.#isType(imported.file, imported.name, visiting);
        }

        return false;
    }

    /** Find the module an export of a module names as a namespace, absent for any other export. */
    async #namespace(
        file: string,
        name: string,
        visiting: ReadonlySet<string>,
    ): Promise<string | undefined> {
        // stop at an export the walk already follows
        const key = `${file}#${name}`;
        if (visiting.has(key)) {
            return undefined;
        }
        const inner = new Set([...visiting, key]);

        // follow the name through re-exports to a namespace export
        const provided = (await this.#table(file, new Set())).provided.get(name);
        if (provided?.kind !== "from") {
            return undefined;
        }

        return provided.imported === "*"
            ? provided.target
            : await this.#namespace(provided.target, provided.imported, inner);
    }

    /** Resolve a module's exports to their symbols, refusing a star cycle or a dead end. */
    async #exportsOf(file: string): Promise<ReadonlyMap<string, graph.Moniker>> {
        // list its export names, refusing star exports that lead back into a module of its package
        const { packageId, module: path } = await this.#location(file);
        await this.#requireAcyclic(file, [file], packageId);
        const { provided } = await this.#table(file, new Set());

        // resolve each name, refusing one that reaches no declaration
        const exports = new Map<string, graph.Moniker>();
        for (const name of provided.keys()) {
            const symbol = await this.#original(file, name, new Set());
            if (symbol === undefined) {
                throw BuildError.of(
                    "INSPECTION_INVALID",
                    `export '${name}' of '${path}' is invalid: it reaches no declaration`,
                );
            }
            exports.set(name, symbol);
        }

        return exports;
    }

    /** Read a module's export table, once unless a star cycle cut it short. */
    async #table(file: string, chain: ReadonlySet<string>): Promise<ExportTable> {
        // read nothing new past a star cycle, and a table read before
        const known = this.#tables.get(file);
        if (chain.has(file)) {
            return { provided: new Map(), isComplete: false };
        } else if (known !== undefined) {
            return known;
        }

        // read it, keeping a complete table
        const table = await this.#readTable(file, new Set([...chain, file]));
        if (table.isComplete) {
            this.#tables.set(file, table);
        }

        return table;
    }

    /** Read each name a module exports and the export providing it, own names over stars'. */
    async #readTable(file: string, chain: ReadonlySet<string>): Promise<ExportTable> {
        // take its own exports, a local name a type only when every export of it is
        const module = await this.#parse(file);
        const own = new Map<string, Provided>();
        for (const each of module.exports) {
            const previous = each.kind === "all" ? undefined : own.get(nameOf(each));
            if (each.kind === "local" && previous?.kind === "local") {
                own.set(each.name, { ...previous, isType: previous.isType && each.isType });
            } else if (each.kind === "all" || previous !== undefined) {
                continue;
            } else if (each.kind === "local") {
                own.set(each.name, { kind: "local", file, local: each.local, isType: each.isType });
            } else if (each.kind === "from") {
                const { name, imported, isType } = each;
                own.set(name, { kind: "from", target: each.file, imported, isType });
            } else {
                own.set("default", { kind: "local", file, local: each.local, isType: false });
            }
        }

        // list the names in statement order, each star adding the names it provides but its default
        const provided = new Map<string, Provided>();
        let isComplete = true;
        for (const each of module.exports) {
            const name = each.kind === "all" ? undefined : nameOf(each);
            const target = each.kind === "all" ? await this.#table(each.file, chain) : undefined;
            isComplete &&= target?.isComplete ?? true;
            for (const [starred, entry] of target?.provided ?? []) {
                if (starred !== "default" && !own.has(starred) && !provided.has(starred)) {
                    provided.set(starred, entry);
                }
            }
            const entry = name === undefined ? undefined : own.get(name);
            if (name !== undefined && entry !== undefined && !provided.has(name)) {
                provided.set(name, entry);
            }
        }

        return { provided, isComplete };
    }

    /** Refuse star exports that lead back into a module of the package on the chain, as a cycle. */
    async #requireAcyclic(
        file: string,
        chain: readonly string[],
        packageId: string,
    ): Promise<void> {
        // pass a module whose star exports were checked before
        if (this.#acyclic.has(file)) {
            return;
        }

        // refuse a star export back into the chain, then check each target in the package
        const module = await this.#parse(file);
        for (const each of module.exports) {
            if (each.kind !== "all") {
                continue;
            }
            const start = chain.indexOf(each.file);
            const target = await this.#location(each.file);
            if (start >= 0) {
                const cycle = await Promise.all(
                    chain.slice(start).map(async (step) => (await this.#location(step)).module),
                );
                throw BuildError.of(
                    "INSPECTION_INVALID",
                    `export * chain '${cycleOf(cycle)}' is invalid: it is a cycle`,
                );
            } else if (target.packageId === packageId) {
                await this.#requireAcyclic(each.file, [...chain, each.file], packageId);
            }
        }
        this.#acyclic.add(file);
    }

    /** Read a file's module once, keeping no syntax, reusing a kept module of the same bytes. */
    #parse(file: string): Promise<OutlineModule> {
        let module = this.#modules.get(file);
        if (module === undefined) {
            module = this.#keptOrRead(file);
            this.#modules.set(file, module);
        }

        return module;
    }

    /** Take the kept module of a file's bytes, else read it. */
    async #keptOrRead(file: string): Promise<OutlineModule> {
        // reuse the kept module of the same bytes
        const { bytes, digest } = await this.#bytes(file);
        const kept = this.#kept.get(file);

        return kept?.digest === digest ? kept : (await this.#read(file, bytes, digest)).module;
    }

    /** Read and parse a file's syntax, keeping its module. */
    async #source(file: string): Promise<Source> {
        // parse the file, noting its module for resolution
        const { bytes, digest } = await this.#bytes(file);
        const source = await this.#read(file, bytes, digest);
        if (!this.#modules.has(file)) {
            this.#modules.set(file, Promise.resolve(source.module));
        }

        return source;
    }

    /** Read a file's bytes and their digest. */
    async #bytes(file: string): Promise<{ bytes: Uint8Array<ArrayBuffer>; digest: Digest }> {
        const bytes = await this.#environment.files.readFile(file);

        return { bytes, digest: await Digest.of(bytes, this.#environment.crypto) };
    }

    /** Parse a file's bytes, keeping its module, refusing a file with syntax errors. */
    async #read(file: string, bytes: Uint8Array<ArrayBuffer>, digest: Digest): Promise<Source> {
        // parse the text, a JSON module as the default export of its value, refusing syntax errors
        const decoded = new TextDecoder().decode(bytes);
        const isJson = file.endsWith(".json");
        const text = isJson ? `export default ${decoded};` : decoded;
        const result = parseSync(file, text, {
            sourceType: "module",
            ...(isJson ? { lang: "js" as const } : {}),
        });
        const [error] = result.errors;
        if (error !== undefined) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `module '${file}' is invalid: ${error.message}`,
            );
        }

        // read the names of each statement, resolving each specifier
        const names: Names = {
            imports: new Map(),
            aliases: new Map(),
            specifiers: [],
            declarations: new Map(),
            exports: [],
        };
        const resolve = (specifier: string) => {
            const resolved = this.#resolver.resolveFileSync(file, specifier).path;
            names.specifiers.push({
                specifier,
                ...(resolved === undefined ? {} : { file: resolved }),
            });

            return resolved;
        };
        for (const statement of result.program.body) {
            readStatement(statement, names, resolve);
        }

        // name the module by its package and path, noting which names declare only types
        const moniker = await this.#module(file);
        const { declarations, ...rest } = names;
        const declared = new Map(
            [...declarations].map(([name, bindings]) => [
                name,
                bindings.every((binding) => binding.isType),
            ]),
        );
        const module = { moniker, digest, declared, ...rest };
        this.#kept.set(file, module);
        const { program, comments } = result;

        return { module, program, text, comments, declarations };
    }

    /** Name a module by its package and path. */
    async #module(file: string): Promise<graph.Moniker> {
        return graph.Moniker.of(await this.#location(file));
    }

    /** Name a symbol or a member of a module. */
    #symbol(module: OutlineModule, name: string, member?: string): graph.Moniker {
        // name it within the module's package and path
        const packageId = graph.Moniker.packageId(module.moniker);
        const path = graph.Moniker.module(module.moniker);
        const nested = member === undefined ? {} : { member };

        return graph.Moniker.of({ packageId, module: path, name, ...nested });
    }

    /** Locate a file by the package owning it and its path within. */
    async #location(file: string): Promise<{ packageId: string; module: string }> {
        const owner = await this.#package(dirname(file));

        return { packageId: owner.id, module: relative(owner.root, file) };
    }

    /** Find the package owning a directory once. */
    #package(directory: string): Promise<{ readonly root: string; readonly id: string }> {
        let found = this.#packages.get(directory);
        if (found === undefined) {
            found = this.#readPackage(directory);
            this.#packages.set(directory, found);
        }

        return found;
    }

    /** Read the package a directory belongs to from the nearest named package.json above it. */
    async #readPackage(directory: string): Promise<{ readonly root: string; readonly id: string }> {
        // read the directory's package.json, skipping one without a name, as a module type marker
        const manifest = join(directory, "package.json");
        const isManifest = await this.#environment.files.exists(manifest);
        const { name } = isManifest ? await this.#readNames(manifest) : {};

        // take the package above a directory without a named package.json, refusing the root
        if (name === undefined) {
            const parent = dirname(directory);
            if (parent === directory) {
                throw BuildError.of(
                    "INSPECTION_INVALID",
                    `module in '${directory}' is invalid: it has no package`,
                );
            }

            return await this.#package(parent);
        }

        // name it by its Destack identifier, else its npm name
        const definition = join(directory, "destack.json");
        const isDestack = await this.#environment.files.exists(definition);
        const { id } = isDestack ? await this.#readNames(definition) : {};

        return { root: directory, id: id ?? `npm:${name}` };
    }

    /** Read the names a package.json or a destack.json gives. */
    async #readNames(path: string): Promise<schema.Infer<typeof PackageNames>> {
        return PackageNames.parse(JSON.parse(await this.#environment.files.readTextFile(path)));
    }
}

/** Read one top-level statement's imports, aliases, declarations and exports. */
function readStatement(
    statement: ESTree.Directive | ESTree.Statement,
    names: Names,
    resolve: (specifier: string) => string | undefined,
): void {
    // bind the names an import declares
    if (statement.type === "ImportDeclaration") {
        const target = resolve(statement.source.value);
        for (const specifier of statement.specifiers) {
            const name =
                specifier.type === "ImportNamespaceSpecifier"
                    ? "*"
                    : specifier.type === "ImportDefaultSpecifier"
                      ? "default"
                      : moduleName(specifier.imported);
            if (target !== undefined) {
                names.imports.set(specifier.local.name, { file: target, name });
            }
        }
    }
    // re-export all of another module's names, or as one namespace
    else if (statement.type === "ExportAllDeclaration") {
        const target = resolve(statement.source.value);
        const isType = statement.exportKind === "type";
        if (target !== undefined && statement.exported === null) {
            names.exports.push({ kind: "all", file: target });
        } else if (target !== undefined && statement.exported !== null) {
            const name = moduleName(statement.exported);
            names.exports.push({ kind: "from", name, file: target, imported: "*", isType });
        }
    }
    // declare and export names, or re-export another module's
    else if (statement.type === "ExportNamedDeclaration") {
        const isType = statement.exportKind === "type";
        for (const [name, binding] of statement.declaration === null
            ? []
            : declare(statement.declaration, names, resolve)) {
            const isTypeOnly = isType || binding.isType;
            names.exports.push({ kind: "local", name, local: name, isType: isTypeOnly });
        }
        const target = statement.source === null ? undefined : resolve(statement.source.value);
        for (const specifier of statement.specifiers) {
            const local = moduleName(specifier.local);
            const name = moduleName(specifier.exported);
            const isTypeOnly = isType || specifier.exportKind === "type";
            names.exports.push(
                target === undefined
                    ? { kind: "local", name, local, isType: isTypeOnly }
                    : { kind: "from", name, file: target, imported: local, isType: isTypeOnly },
            );
        }
    }
    // export a default function or class under its name, else its expression as the default
    else if (statement.type === "ExportDefaultDeclaration") {
        const { declaration } = statement;
        const isNamed =
            declaration.type === "FunctionDeclaration" || declaration.type === "ClassDeclaration";
        const name = isNamed && declaration.id !== null ? declaration.id.name : "default";
        addBinding(names, name, { node: isNamed ? declaration : statement, isType: false });
        names.exports.push({ kind: "local", name: "default", local: name, isType: false });
    }
    // export one value as the module, by its name or as the default
    else if (statement.type === "TSExportAssignment") {
        const name = qualifiedName(statement.expression);
        const local = name?.length === 1 && name[0] !== undefined ? name[0] : ASSIGNED;
        if (local === ASSIGNED) {
            addBinding(names, ASSIGNED, { node: statement, isType: false });
        }
        names.exports.push({ kind: "assigned", local });
    }
    // declare the module's own names
    else if (isDeclaration(statement)) {
        declare(statement, names, resolve);
    }
}

/** Declare a declaration's names in a module, returning each name with its binding. */
function declare(
    declaration: ESTree.Declaration,
    names: Names,
    resolve: (specifier: string) => string | undefined,
): [string, Binding][] {
    // bind an `import x = require()` as a namespace, and an `import A = B.C` as an alias
    if (declaration.type === "TSImportEqualsDeclaration") {
        const { id, moduleReference } = declaration;
        const target =
            moduleReference.type === "TSExternalModuleReference"
                ? resolve(moduleReference.expression.value)
                : undefined;
        const alias = qualifiedName(moduleReference);
        if (target !== undefined) {
            names.imports.set(id.name, { file: target, name: "*" });
        } else if (alias !== undefined) {
            names.aliases.set(id.name, alias);
        }

        return [[id.name, { node: declaration, isType: declaration.importKind === "type" }]];
    }

    // declare each name with its binding
    const declared = bindingsOf(declaration);
    for (const [name, binding] of declared) {
        addBinding(names, name, binding);
    }

    return declared;
}

/** Add a declaration of a name, after any earlier one. */
function addBinding(names: Names, name: string, binding: Binding): void {
    const bindings = names.declarations.get(name) ?? [];
    bindings.push(binding);
    names.declarations.set(name, bindings);
}

/** Report whether a statement declares names. */
function isDeclaration(
    statement: ESTree.Directive | ESTree.Statement,
): statement is ESTree.Declaration {
    return (
        statement.type === "VariableDeclaration" ||
        statement.type === "FunctionDeclaration" ||
        statement.type === "TSDeclareFunction" ||
        statement.type === "ClassDeclaration" ||
        statement.type === "TSTypeAliasDeclaration" ||
        statement.type === "TSInterfaceDeclaration" ||
        statement.type === "TSEnumDeclaration" ||
        statement.type === "TSModuleDeclaration" ||
        statement.type === "TSImportEqualsDeclaration"
    );
}

/** Read an import's or export's name, a string literal by its value. */
function moduleName(name: ESTree.ModuleExportName): string {
    return name.type === "Identifier" ? name.name : name.value;
}

/** List the names a declaration binds, each variable binding with its declaration's keyword. */
function bindingsOf(declaration: ESTree.Declaration): [string, Binding][] {
    // name each binding of a variable declaration
    if (declaration.type === "VariableDeclaration") {
        return declaration.declarations.flatMap((declarator) =>
            bindingNames(declarator.id).map((name): [string, Binding] => [
                name,
                { node: declarator, isType: false, keyword: declaration.kind },
            ]),
        );
    }
    // bind no name for a global augmentation
    else if (declaration.type === "TSModuleDeclaration" && declaration.global) {
        return [];
    }

    // name the declaration itself, absent for an anonymous one or a string-named module
    const isType =
        declaration.type === "TSInterfaceDeclaration" ||
        declaration.type === "TSTypeAliasDeclaration";
    const { id } = declaration;
    const name = id?.type === "Identifier" ? id.name : undefined;

    return name === undefined ? [] : [[name, { node: declaration, isType }]];
}

/** List the names a binding pattern binds. */
function bindingNames(pattern: ESTree.BindingPattern | ESTree.BindingRestElement | null): string[] {
    // name an identifier
    if (pattern === null) {
        return [];
    } else if (pattern.type === "Identifier") {
        return [pattern.name];
    }
    // name each property, element, default and rest of a pattern
    else if (pattern.type === "ObjectPattern") {
        return pattern.properties.flatMap((property) =>
            bindingNames(property.type === "RestElement" ? property : property.value),
        );
    } else if (pattern.type === "ArrayPattern") {
        return pattern.elements.flatMap((element) => bindingNames(element));
    } else if (pattern.type === "AssignmentPattern") {
        return bindingNames(pattern.left);
    } else {
        return bindingNames(pattern.argument);
    }
}

/** Group the members of a name's classes, interfaces and enums by name, in source order. */
function membersOf(bindings: readonly Binding[]): Map<string, Binding[]> {
    const members = new Map<string, Binding[]>();
    for (const { node } of bindings) {
        // take a class's or interface's body, and an enum's members
        const nodes: ESTree.Node[] =
            node.type === "ClassDeclaration" || node.type === "TSInterfaceDeclaration"
                ? node.body.body
                : node.type === "TSEnumDeclaration"
                  ? node.body.members
                  : [];

        // group each named member with its other declarations, such as overloads and accessors
        for (const member of nodes) {
            const name = memberName(member);
            if (name !== undefined) {
                members.set(name, [...(members.get(name) ?? []), { node: member, isType: false }]);
            }
        }
    }

    return members;
}

/** Read a member's name, absent for a computed one or one without a key. */
function memberName(member: ESTree.Node): string | undefined {
    // name a constructor, and an enum member by its identifier or string
    if (member.type === "MethodDefinition" && member.kind === "constructor") {
        return "constructor";
    } else if (member.type === "TSEnumMember") {
        return literalName(member.id);
    }

    // name any other keyed member by its literal key
    const isKeyed =
        member.type === "MethodDefinition" ||
        member.type === "PropertyDefinition" ||
        member.type === "AccessorProperty" ||
        member.type === "TSPropertySignature" ||
        member.type === "TSMethodSignature";

    return !isKeyed || member.computed ? undefined : literalName(member.key);
}

/** Read an identifier's name or a string literal's value, absent for any other node. */
function literalName(node: ESTree.Node): string | undefined {
    if (node.type === "Identifier") {
        return node.name;
    } else if (node.type === "Literal" && typeof node.value === "string") {
        return node.value;
    }

    return undefined;
}

/** List the names a class or interface extends or implements. */
function heritageOf(node: ESTree.Node): string[][] {
    // take a class's base and implemented interfaces, and an interface's extended ones
    const expressions: ESTree.Node[] = [];
    if (node.type === "ClassDeclaration") {
        if (node.superClass !== null) {
            expressions.push(node.superClass);
        }
        expressions.push(...(node.implements ?? []).map((clause) => clause.expression));
    } else if (node.type === "TSInterfaceDeclaration") {
        expressions.push(...node.extends.map((clause) => clause.expression));
    }

    return expressions.flatMap((expression) => {
        const name = qualifiedName(expression);

        return name === undefined ? [] : [name];
    });
}

/** Walk a program for its calls and type references by name, in source order. */
function usesOf(program: ESTree.Program): Use[] {
    // visit the nodes depth first
    const uses: Use[] = [];
    const pending: ESTree.Node[] = [program];
    for (let node = pending.pop(); node !== undefined; node = pending.pop()) {
        // take a call's or construction's callee, and a type reference's name
        const isCall = node.type === "CallExpression" || node.type === "NewExpression";
        const name =
            node.type === "CallExpression" || node.type === "NewExpression"
                ? qualifiedName(node.callee)
                : node.type === "TSTypeReference"
                  ? qualifiedName(node.typeName)
                  : undefined;
        if (name !== undefined) {
            const kind = isCall ? "calls" : "references";
            uses.push({ kind, name, start: node.start, end: node.end });
        }

        // visit its children in source order
        pending.push(...children(node).toReversed());
    }

    return uses;
}

/** Describe a name or member as a symbol with each of its declarations and the first comment. */
function describe(
    source: Source,
    moniker: graph.Moniker,
    bindings: readonly Binding[],
    isExported: boolean,
): graph.Symbol {
    // describe each declaration by its kind, range and written signature
    const file = graph.Moniker.module(source.module.moniker);
    const declarations = bindings.map((binding) => ({
        kind: kindOf(binding.node, moniker),
        source: { file, start: binding.node.start, end: binding.node.end },
        signature: signatureOf(source.text, binding, moniker),
    }));

    // take the first declaration's documentation that has one, as overloads document the first
    const comment = bindings
        .map((binding) => commentOf(source, binding.node))
        .find((each) => each !== undefined);

    return { moniker, declarations, ...(comment === undefined ? {} : { comment }), isExported };
}

/** Read a declaration's or member's symbol kind. */
function kindOf(node: ESTree.Node, moniker: graph.Moniker): graph.SymbolDeclaration["kind"] {
    // kind a class method by what it defines
    if (node.type === "MethodDefinition") {
        return node.kind === "constructor"
            ? "constructor"
            : node.kind === "get"
              ? "getter"
              : node.kind === "set"
                ? "setter"
                : "method";
    }

    // kind any other declaration by its node, refusing one it does not know
    const kind = SYMBOL_KINDS[node.type];
    if (kind === undefined) {
        throw BuildError.of(
            "INSPECTION_INVALID",
            `declaration '${moniker}' is invalid: its kind ${node.type} is unknown`,
        );
    }

    return kind;
}

/** Write a declaration's signature on one line as written, its body and value cut. */
function signatureOf(text: string, binding: Binding, moniker: graph.Moniker): string {
    // write a default-exported expression and an `export =` as the export
    const { node } = binding;
    if (node.type === "ExportDefaultDeclaration") {
        return "export default";
    } else if (node.type === "TSExportAssignment") {
        return "export =";
    }

    // end a function, class or interface before its body, a variable or property before its value
    const isBodied =
        node.type === "FunctionDeclaration" ||
        node.type === "ClassDeclaration" ||
        node.type === "TSInterfaceDeclaration";
    const end =
        node.type === "MethodDefinition"
            ? (node.value.body?.start ?? node.end)
            : node.type === "VariableDeclarator"
              ? node.id.end
              : node.type === "PropertyDefinition" || node.type === "AccessorProperty"
                ? (node.typeAnnotation?.end ?? node.key.end)
                : isBodied
                  ? (node.body?.start ?? node.end)
                  : node.end;
    const written = text
        .slice(node.start, end)
        .replaceAll(/\s+/gu, " ")
        .replace(/[\s;,={]+$/u, "")
        .trim();

    // prefix a variable with its declaration's keyword, and cut a long signature
    const prefixed = binding.keyword === undefined ? written : `${binding.keyword} ${written}`;
    const signature = prefixed === "" ? (graph.Moniker.name(moniker) ?? moniker) : prefixed;

    return signature.length > SIGNATURE_LENGTH
        ? `${signature.slice(0, SIGNATURE_LENGTH - 1)}…`
        : signature;
}

/** Read the first sentence of the documentation comment right above a declaration. */
function commentOf(source: Source, node: ESTree.Node): string | undefined {
    // find the last documentation comment before the declaration, only keywords between them
    const documentation = source.comments.findLast(
        (comment) =>
            comment.type === "Block" &&
            comment.value.startsWith("*") &&
            comment.end <= node.start &&
            /^\s*(export\s+(default\s+)?)?(declare\s+)?((const|let|var|async)\s+)?$/u.test(
                source.text.slice(comment.end, node.start),
            ),
    );
    if (documentation === undefined) {
        return undefined;
    }

    // take its text up to the first tag, then its first sentence
    const lines = documentation.value.split("\n").map((line) => line.replace(/^\s*\*+\s?/u, ""));
    const text = lines.join(" ").split(/\s@\w/u)[0]?.trim() ?? "";
    const sentence = /^[\s\S]*?[.!?](?=\s|$)/u.exec(text)?.[0] ?? text;

    return sentence === "" ? undefined : sentence.replaceAll(/\s+/gu, " ");
}

/** Write a cycle of module paths from its least path, the same from any start. */
function cycleOf(cycle: readonly string[]): string {
    // rotate the cycle to start at its least path
    const least = cycle.toSorted((left, right) => left.localeCompare(right))[0];
    const first = least === undefined ? 0 : cycle.indexOf(least);
    const ordered = [...cycle.slice(first), ...cycle.slice(0, first)];

    return [...ordered, ordered[0]].join(" -> ");
}

/** Name the export a statement gives: its exported name, or the default for an `export =`. */
function nameOf(each: Exclude<Export, { readonly kind: "all" }>): string {
    return each.kind === "assigned" ? "default" : each.name;
}
