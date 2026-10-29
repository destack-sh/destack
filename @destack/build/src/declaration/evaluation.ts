import { Digest, present, schema } from "@destack/schema";
import { type FileSystem, FileSystemError } from "@destack/environment";
import { dirname, join, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { type Plugin, rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import {
    DeclarationDescription,
    DeclarationSite,
    graph,
    Package,
    PackageJson,
} from "@destack/package";
import { DECLARED_MODULE } from "@destack/package/transform";
import type { FunctionExport } from "./declaration.ts";
import type { ExampleDeclaration } from "../typescript/example.ts";
import type { ConstructorCatalog, ConstructorKind } from "./catalog.ts";
import type { PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/vite";
import type { ModuleTransformer } from "@destack/package/transform";
import { BuildError } from "../error/index.ts";
import type { DependencyTree } from "../dependency/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { isHostModule, type PluginOption, runtimeConditions } from "@destack/package/build";
import { HOST_RUNTIME, type Runtime, WORKERD_SCHEME } from "@destack/package/runtime";
import { selectExport } from "../source/source.ts";
import { loadExtensions, transformPlugins } from "../compile/extension.ts";
import { assetPlugin } from "../compile/asset.ts";

/** An installed JavaScript module, imported as the host loads it, without a query such as ?url. */
const INSTALLED_MODULE = /\.[cm]?js$/u;
/** A stylesheet, which declarations evaluate without. */
const STYLESHEET = /\.css(?:\?.*)?$/u;
/** The prefix of the module standing in for a workerd module, which the host lacks. */
const UNAVAILABLE_PREFIX = "\0destack-unavailable:";
/** The identifier of the generated entry. */
const ENTRY = "\0destack-declarations";
/** The resolved identifier of the declaration recorder inside the evaluation. */
const RECORDER = `\0${DECLARED_MODULE}`;

/** The recorder collecting each declaration with the site of its constructor's call. */
const RECORDER_SOURCE = [
    "export const declared = [];",
    "export const declare = (value, site) => (declared.push([value, site]), value);",
    "",
].join("\n");

/** The exports of the generated entry. */
const EvaluatedEntry = schema.object({
    /** The namespace of each export entry, in the order of the package's export paths. */
    entries: schema.array(schema.record(schema.string(), schema.unknown())),
    /** Each declaration the package's modules constructed, with its site. */
    recorded: schema.array(schema.tuple([schema.unknown(), schema.unknown()])),
    /** The functions of each constructor kind, aligned with the catalog's kinds. */
    functions: schema.array(
        schema.object({
            value: schema.unknown(),
            describe: schema.unknown(),
            vocabulary: schema.unknown(),
            symbols: schema.unknown(),
        }),
    ),
});
/** The exports of the generated entry. */
type EvaluatedEntry = schema.Infer<typeof EvaluatedEntry>;

/** The exports of the generated examples entry. */
const EvaluatedExamples = schema.object({
    /** The properties of each example, as its module evaluates them. */
    properties: schema.array(schema.jsonObject()),
});

/** The name a description gives its declaration. */
const DescriptionName = schema.looseObject({ name: schema.string() });

/** The terms a kind's vocabulary defines for a declaration, by name. */
const Terms = schema.record(schema.string(), schema.unknown());

/** The symbols a kind derives for a declaration. */
const Symbols = schema.array(graph.MemberSymbol);

/** A declaration a package exports, described by one of its constructor's kinds. */
export const EvaluatedDeclaration = schema.object({
    /** The description. */
    description: DeclarationDescription,
    /** The absolute module exporting it. */
    file: schema.string(),
    /** The name it is exported under. */
    export: schema.string(),
});
/** A declaration a package exports, described by one of its constructor's kinds. */
export type EvaluatedDeclaration = schema.Infer<typeof EvaluatedDeclaration>;

/** How an evaluation describes declarations: for a release, or with the symbols an index reads. */
export interface EvaluationOptions {
    /** Whether the evaluation derives each declaration's symbols for the module graph. */
    readonly isIndexing: boolean;
    /** The build's module transformer, which its compilations reuse. */
    readonly transformer: ModuleTransformer;
}

/** The declarations a package exports. */
export interface Evaluation {
    /** The declarations, one per kind their constructors describe them as. */
    readonly declarations: readonly EvaluatedDeclaration[];
}

/** A constructor kind with its place among the generated entry's functions. */
interface CatalogedKind extends ConstructorKind {
    /** The kind's index among the entry's functions. */
    readonly index: number;
}

/** The declarations a build evaluates in its worker. */
export const Evaluation = {
    /** Evaluate a package's exports, describing each declaration its constructors made. */
    async evaluate(
        projects: readonly PackageSource[],
        catalog: ConstructorCatalog,
        options: EvaluationOptions,
    ): Promise<Evaluation> {
        // list each constructor kind of the package and its dependencies, keyed by package and constructor
        const [first] = projects;
        if (first === undefined) {
            throw BuildError.of("INVARIANT_VIOLATED", "an evaluation needs a project");
        }
        const listed: { key: string; kind: CatalogedKind }[] = [];
        const directories = [
            first.directory,
            ...Object.values(first.tree.directories),
            ...(await installedDirectories(first)),
        ];
        for (const directory of new Set(directories)) {
            const location = await catalog.locate(directory);
            for (const [name, inspected] of await catalog.inspected(location)) {
                for (const kind of inspected.kinds) {
                    listed.push({
                        key: `${location.name}#${name}`,
                        kind: { ...kind, index: listed.length },
                    });
                }
            }
        }
        const kinds = Map.groupBy(listed, (entry) => entry.key);

        // evaluate each export entry once, in the first project exporting it
        const declarations: EvaluatedDeclaration[] = [];
        const described = new Set<string>();
        const evaluated = new Set<string>();
        const exported = new Set<string>();
        const unexported = new Map<string, string>();
        for (const project of projects) {
            const entries = Object.entries(project.exports).filter(
                ([, file]) => !evaluated.has(file),
            );
            for (const [, file] of entries) {
                evaluated.add(file);
            }
            if (entries.length === 0) {
                continue;
            }
            const found = await evaluateProject(
                project,
                entries,
                listed.map((entry) => entry.kind),
                kinds,
                options,
            );
            for (const site of found.exported) {
                exported.add(site);
            }
            for (const [site, name] of found.unexported) {
                unexported.set(site, name);
            }
            for (const declaration of found.declarations) {
                const { symbol, kind, name } = declaration.description;
                const key = JSON.stringify([symbol.symbol.module, symbol.symbol.name, kind, name]);
                if (!described.has(key)) {
                    described.add(key);
                    declarations.push(declaration);
                }
            }
        }

        // refuse a declaration no project exports
        const owner = first.packageJson.package;
        for (const [site, name] of unexported) {
            if (!exported.has(site)) {
                throw BuildError.of(
                    "DECLARATION_UNEXPORTED",
                    `declared ${name} is not exported by ${owner.name}`,
                );
            }
        }

        return { declarations };
    },

    /** Evaluate the properties each of a package's examples renders with. */
    async examples(
        project: PackageSource,
        examples: readonly Omit<ExampleDeclaration, "properties">[],
        transformer: ModuleTransformer,
    ): Promise<ExampleDeclaration[]> {
        // import each example and export its properties
        if (examples.length === 0) {
            return [];
        }
        const source = [
            ...examples.map(
                (example, index) =>
                    `import { ${JSON.stringify(example.symbol)} as example${index} } from ${JSON.stringify(join(project.directory, example.source.file))};`,
            ),
            `export const properties = [${examples.map((_, index) => `example${index}.properties`).join(", ")}];`,
        ].join("\n");
        const { properties } = EvaluatedExamples.parse(
            await evaluateEntry(source, project, transformer),
        );

        return examples.map((example, index) => ({
            ...example,
            properties: present(properties[index], `the properties of ${example.symbol}`),
        }));
    },
};

/** Evaluate one output project's exports, describing each declaration its constructors made. */
async function evaluateProject(
    project: PackageSource,
    entries: readonly (readonly [string, string])[],
    listed: readonly CatalogedKind[],
    kinds: ReadonlyMap<string, readonly { readonly kind: CatalogedKind }[]>,
    options: EvaluationOptions,
): Promise<{
    readonly declarations: EvaluatedDeclaration[];
    readonly exported: ReadonlySet<string>;
    readonly unexported: ReadonlyMap<string, string>;
}> {
    // evaluate the export entries beside the kinds' functions
    const exports = entries.toSorted(([left], [right]) =>
        left < right ? -1 : Number(left > right),
    );
    const source = generateEntry(
        exports.map(([, file]) => file),
        listed,
    );
    const evaluated = EvaluatedEntry.parse(
        await evaluateEntry(source, project, options.transformer),
    );

    // find each exported value a constructor made, once, at its first export
    const sites = new Map<unknown, DeclarationSite>(
        evaluated.recorded.map(([value, site]) => [value, DeclarationSite.parse(site)]),
    );
    const seen = new Set<unknown>();
    const declarations: EvaluatedDeclaration[] = [];
    const owner = project.packageJson.package;
    for (const [index, [, file]] of exports.entries()) {
        const namespace = present(evaluated.entries[index], `the namespace of ${file}`);
        for (const [name, value] of Object.entries(namespace).toSorted(([left], [right]) =>
            left < right ? -1 : Number(left > right),
        )) {
            // leave the declarations in an exported list, which no export of their own declares
            if (Array.isArray(value)) {
                for (const element of value) {
                    seen.add(element);
                }
                continue;
            }

            // leave a value no constructor made, and one found before
            const site = sites.get(value);
            if (site === undefined || seen.has(value)) {
                continue;
            }
            seen.add(value);

            // describe it as each kind its constructor describes, refusing another package's
            requireDeclaringPackage(value, owner, name);
            for (const { kind } of constructorKinds(kinds, site)) {
                const located = { owner, name, site, kind };
                const description = await describeDeclaration(
                    value,
                    located,
                    evaluated,
                    project,
                    options,
                );
                declarations.push({ description, file, export: name });
            }
        }
    }

    // name each module-level declaration its module does not export, by its first kind's description
    const exported = new Set(
        [...seen].flatMap((value) => {
            const site = sites.get(value);

            return site === undefined ? [] : [siteKey(site)];
        }),
    );
    const unexported = new Map<string, string>();
    for (const [value, site] of sites) {
        if (!seen.has(value) && site.binding === "unexported") {
            const [first] = constructorKinds(kinds, site);
            const kind = present(first, `a kind of ${site.package}#${site.constructor}`).kind;
            const located = { owner, name: site.constructor, site, kind };
            const { name } = await describeDeclaration(value, located, evaluated, project, options);
            unexported.set(siteKey(site), name);
        }
    }

    return { declarations, exported, unexported };
}

/** List the kinds of a recorded declaration's constructor, which its package declares. */
function constructorKinds(
    kinds: ReadonlyMap<string, readonly { readonly kind: CatalogedKind }[]>,
    site: DeclarationSite,
): readonly { readonly kind: CatalogedKind }[] {
    return present(
        kinds.get(`${site.package}#${site.constructor}`),
        `the kinds of constructor ${site.package}#${site.constructor}`,
    );
}

/** Key a declaration's site by its module and position. */
function siteKey(site: DeclarationSite): string {
    return `${site.module}:${site.line}:${site.column}`;
}

/** List the installed Destack packages a project depends on, outside its dependencies' builds. */
async function installedDirectories(project: PackageSource): Promise<string[]> {
    // find each runtime dependency without a build in the nearest node_modules above the project
    const { files } = project.environment;
    const directories: string[] = [];
    for (const name of PackageJson.runtimeDependencies(project.packageJson)) {
        if (project.tree.directories[name] !== undefined) {
            continue;
        }
        for (let directory = project.directory; ; directory = dirname(directory)) {
            const installed = join(directory, "node_modules", name);
            const isInstalled = await files.stat(join(installed, "destack.json")).then(
                () => true,
                (error: unknown) => {
                    if (FileSystemError.is(error, "NOT_FOUND")) {
                        return false;
                    }
                    throw error;
                },
            );
            if (isInstalled) {
                directories.push(await files.realPath(installed));
                break;
            } else if (dirname(directory) === directory) {
                break;
            }
        }
    }

    return directories;
}

/** Write the entry importing each export entry, the recorder and every kind's functions. */
function generateEntry(entries: readonly string[], kinds: readonly CatalogedKind[]): string {
    // select each inspecting module once, as the build worker runs it
    const conditions = new Set(["import", "default", ...runtimeConditions(HOST_RUNTIME)]);
    const modules = new Map<string, number>();
    const locate = (inspector: FunctionExport) => locateFunction(inspector, conditions, modules);
    const functions = kinds.map(({ inspector }) => {
        // name each function of the kind, absent ones as undefined
        const value = inspector.value && locate(inspector.value);
        const describe = locate(inspector.describe);
        const vocabulary = inspector.vocabulary && locate(inspector.vocabulary);
        const symbols = inspector.symbols && locate(inspector.symbols);

        return `{ value: ${value ?? "undefined"}, describe: ${describe}, vocabulary: ${vocabulary ?? "undefined"}, symbols: ${symbols ?? "undefined"} }`;
    });

    // import the recorder first, then the entries and the inspecting modules
    return [
        `import { declared } from ${JSON.stringify(DECLARED_MODULE)};`,
        ...entries.map((file, index) => `import * as entry${index} from ${JSON.stringify(file)};`),
        ...[...modules].map(
            ([specifier, index]) =>
                `import * as inspector${index} from ${JSON.stringify(specifier)};`,
        ),
        `export const entries = [${entries.map((_, index) => `entry${index}`).join(", ")}];`,
        "export const recorded = declared;",
        `export const functions = [${functions.join(",\n")}];`,
    ].join("\n");
}

/** Name a function in the generated entry, importing its module once. */
function locateFunction(
    inspector: FunctionExport,
    conditions: ReadonlySet<string>,
    modules: Map<string, number>,
): string {
    // require the module the runtime selects
    const path = selectExport(inspector.target, conditions);
    if (typeof path !== "string") {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `${inspector.directory} exports no ${HOST_RUNTIME} module at ${inspector.subpath}`,
        );
    }

    // import each module once by its package specifier
    const specifier = `${inspector.package}${inspector.subpath.slice(1)}`;
    const index = modules.get(specifier) ?? modules.size;
    modules.set(specifier, index);

    return `inspector${index}[${JSON.stringify(inspector.name)}]`;
}

/** Evaluate a generated entry, refusing a failed evaluation as an inspection failure. */
async function evaluateEntry(
    source: string,
    project: PackageSource,
    transformer: ModuleTransformer,
): Promise<unknown> {
    return await bundleAndEvaluate(source, project, transformer).catch((cause: unknown) => {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `declaration evaluation failed: ${messageOf(cause)}`,
            { cause },
        );
    });
}

/** Bundle a generated entry with the modules it imports, then evaluate it as one module. */
async function bundleAndEvaluate(
    source: string,
    project: PackageSource,
    transformer: ModuleTransformer,
): Promise<unknown> {
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    try {
        // bundle the entry with the modules it imports, recording each declaration
        bundle = await rolldown({
            input: ENTRY,
            cwd: project.directory,
            ...(project.configuration === undefined ? {} : { tsconfig: project.configuration }),
            platform: "node",
            external: isHostModule,
            resolve: { conditionNames: runtimeConditions(HOST_RUNTIME) },
            treeshake: { moduleSideEffects: true },
            plugins: [
                entryPlugin(source),
                assetPlugin(project.environment.files),
                treePlugin(project.tree, project.runtime, project.environment.files),
                modulePlugin(transformer),
                ...transformPlugins(
                    (
                        await loadExtensions(
                            project.directory,
                            project.packageJson,
                            project.tree,
                            project.environment.files,
                        )
                    ).map((loaded) => loaded.extension),
                    {
                        directory: project.directory,
                        runtime: HOST_RUNTIME,
                        server: true,
                        options: {},
                        dependencies: project.tree.directories,
                        environment: project.environment,
                    },
                ).map((option) => serverTransform(option)),
            ],
        });
        const generated = await bundle.generate({ format: "esm", codeSplitting: false });
        const chunks = generated.output.filter((file) => file.type === "chunk");
        const [chunk] = chunks;
        if (chunks.length !== 1 || chunk === undefined) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                "declaration evaluation requires one JavaScript module",
            );
        }

        // evaluate the entry, linking its externals as the host loads them
        const module = new SourceTextModule(chunk.code);
        await module.link(linkHost);
        await module.evaluate();

        return module.namespace;
    } finally {
        await bundle?.close();
    }
}

/** Link an external of the entry as the host loads it: a builtin, a build module or a file. */
async function linkHost(specifier: string): Promise<SyntheticModule> {
    // expose the host module's exports, which the host caches once per file
    const exports = schema.record(schema.string(), schema.unknown()).parse(await import(specifier));
    const names = Object.keys(exports);

    return new SyntheticModule(names, function () {
        for (const name of names) {
            this.setExport(name, exports[name]);
        }
    });
}

/** Describe one exported declaration as one kind, with its terms and symbols. */
async function describeDeclaration(
    declared: unknown,
    located: {
        readonly owner: Package;
        readonly name: string;
        readonly site: DeclarationSite;
        readonly kind: CatalogedKind;
    },
    evaluated: EvaluatedEntry,
    project: PackageSource,
    options: EvaluationOptions,
): Promise<DeclarationDescription> {
    // read the kind's value of the declaration and describe it, refusing a failure as the declaration's
    const { owner, name, site, kind } = located;
    const functions = present(
        evaluated.functions[kind.index],
        `the functions of kind ${kind.kind}`,
    );
    let value: unknown;
    let described: unknown;
    try {
        value =
            typeof functions.value === "function"
                ? Reflect.apply(functions.value, undefined, [declared])
                : declared;
        if (typeof functions.describe !== "function") {
            throw BuildError.of("INSPECTION_FAILED", `kind ${kind.kind} has no describe function`);
        }
        described = await Reflect.apply(functions.describe, undefined, [value]);
    } catch (cause) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `declaration evaluation failed: ${kind.kind} ${name} (${site.module}): ${messageOf(cause)}`,
            { cause },
        );
    }

    // serialize the domain description under the declaration's described name
    const description: unknown = JSON.parse(stringifyInspection(described));
    const parsed = DeclarationDescription.parse({
        name: nameOf(description, name),
        kind: kind.kind,
        package: kind.package,
        symbol: { package: owner, symbol: { module: site.module, name } },
        source: { file: site.module, line: site.line, column: site.column },
        description,
    });

    // digest each term's definition the kind's code returns for the declaration
    const terms =
        typeof functions.vocabulary === "function"
            ? Terms.parse(Reflect.apply(functions.vocabulary, undefined, [value]))
            : undefined;
    const vocabulary: Record<string, Digest> = {};
    for (const [term, definition] of Object.entries(terms ?? {})) {
        vocabulary[`${parsed.kind}/${term}`] = await Digest.json(
            definition,
            project.environment.crypto,
        );
    }

    // list the symbols the kind's code derives from the declaration, for an index
    const symbols =
        options.isIndexing && typeof functions.symbols === "function"
            ? Symbols.parse(Reflect.apply(functions.symbols, undefined, [value]))
            : undefined;

    return {
        ...parsed,
        ...(terms === undefined ? {} : { vocabulary }),
        ...(symbols === undefined ? {} : { symbols }),
    };
}

/** Read the name a description gives, else the declaration's export name. */
function nameOf(description: unknown, fallback: string): string {
    const name = DescriptionName.safeParse(description);

    return name.success ? name.data.name : fallback;
}

/** Require an exported declaration to record the package that exports it. */
function requireDeclaringPackage(value: unknown, owner: Package, name: string): void {
    const stamped = Package.declaring(value);
    // refuse a declaration without a package
    if (stamped === undefined) {
        throw BuildError.of("INSPECTION_FAILED", `${name} declaration has no package`);
    }
    // refuse a declaration of another package
    else if (
        stamped.id !== owner.id ||
        stamped.name !== owner.name ||
        stamped.version !== owner.version
    ) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `${name} declaration belongs to a different package`,
        );
    }
}

/** Mark dependencies external: Destack ones at their builds' modules, npm ones at their files. */
function treePlugin(tree: DependencyTree, runtime: Runtime, files: FileSystem): Plugin {
    return {
        name: "destack-dependency-builds",
        async resolveId(specifier, importer, options) {
            // import a Destack dependency's host module from its build, else bundle the output runtime's
            const modules = tree.export(specifier);
            const host = modules?.[HOST_RUNTIME];
            const own = modules?.[runtime];
            if (host !== undefined) {
                return { id: host, external: true };
            } else if (own !== undefined) {
                return { id: own };
            } else if (modules !== undefined) {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `${specifier} has no ${[...new Set([HOST_RUNTIME, runtime])].join(" or ")} module`,
                );
            }

            // load an installed or built package's module from its file
            if (specifier.startsWith(".") || specifier.startsWith("\0")) {
                return null;
            }
            const resolved = await this.resolve(specifier, importer, {
                ...options,
                skipSelf: true,
            });
            const isModule =
                resolved !== null &&
                resolved.id.split(sep).includes("node_modules") &&
                INSTALLED_MODULE.test(resolved.id);

            return isModule ? { id: await files.realPath(resolved.id), external: true } : null;
        },
    };
}

/** Resolve and load the generated entry, setting module URLs and leaving lazy imports. */
function entryPlugin(source: string): Plugin {
    return {
        name: "destack-declarations",
        resolveId(specifier, _importer, options) {
            // resolve the generated entry, the recorder, and a workerd module as one failing once touched
            if (specifier === ENTRY) {
                return ENTRY;
            } else if (specifier === DECLARED_MODULE) {
                return RECORDER;
            } else if (WORKERD_SCHEME.test(specifier)) {
                return `${UNAVAILABLE_PREFIX}${specifier}`;
            }
            // leave lazily imported modules unloaded
            else if (options.kind === "dynamic-import") {
                return { id: specifier, external: true };
            }
            // resolve other modules as usual
            else {
                return null;
            }
        },
        load: (id) => {
            // load the entry, the recorder, a stand-in for a workerd module, and a stylesheet as nothing
            if (id === ENTRY) {
                return source;
            } else if (id === RECORDER) {
                return RECORDER_SOURCE;
            } else if (id.startsWith(UNAVAILABLE_PREFIX)) {
                const code = unavailableModule(id.slice(UNAVAILABLE_PREFIX.length));

                return { code, moduleType: "js" };
            } else if (STYLESHEET.test(id)) {
                return { code: "export {};\n", moduleType: "js" };
            }

            return null;
        },
        transform: {
            filter: { id: /\.[cm]?[jt]sx?$/u, code: /import\.meta\.url/u },
            async handler(code, id) {
                // define the module's URL as its source file's
                const result = await transform(id, code, {
                    define: { "import.meta.url": JSON.stringify(pathToFileURL(id).href) },
                    sourcemap: true,
                });
                if (result.errors.length) {
                    throw BuildError.of("INSPECTION_FAILED", JSON.stringify(result.errors));
                }

                return {
                    code: result.code,
                    ...(result.map === undefined ? {} : { map: result.map }),
                };
            },
        },
    };
}

/** Write the CommonJS module standing in for a runtime module the host lacks, failing once read. */
function unavailableModule(specifier: string): string {
    const message = JSON.stringify(`${specifier} is unavailable while declarations evaluate`);

    return [
        `const fail = () => { throw new Error(${message}); };`,
        'const failing = { get: (_target, key) => (key === "__esModule" ? undefined : fail()), ownKeys: () => [] };',
        "module.exports = new Proxy({}, { ...failing, getPrototypeOf: () => new Proxy({}, failing) });",
        "",
    ].join("\n");
}

/** Transform modules as Vite's server environment does, as the host runs what it evaluates. */
function serverTransform(option: PluginOption): PluginOption {
    // follow nested and pending plugins to each plugin
    if (Array.isArray(option)) {
        return option.map((each: PluginOption) => serverTransform(each));
    } else if (option instanceof Promise) {
        return option.then((each: PluginOption) => serverTransform(each));
    } else if (typeof option !== "object" || option === null || !("transform" in option)) {
        return option;
    }

    // pass each transform the server flag Vite gives server modules
    const { transform: hook } = option;
    if (hook === undefined) {
        return option;
    }
    const handler = typeof hook === "function" ? hook : hook.handler;
    const served: typeof handler = function (code, id, options) {
        return handler.call(this, code, id, {
            ...present(options, "the transform options rolldown passes"),
            ssr: true,
        });
    };

    return {
        ...option,
        transform: typeof hook === "function" ? served : { ...hook, handler: served },
    };
}

/** Read a failure's message. */
function messageOf(cause: unknown): string {
    return cause instanceof Error ? cause.message : String(cause);
}
