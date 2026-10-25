import { Digest, type DigestCrypto, present, schema } from "@destack/schema";
import type { FileSystem } from "@destack/environment";
import { join, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { type Plugin, rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import { graph, Package, DeclarationDescription } from "@destack/package";
import type { DeclarationExport, FunctionExport } from "./declaration.ts";
import type { ExampleDeclaration } from "../typescript/example.ts";
import { type Comparator, Plan } from "@destack/resource";
import type { PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/vite";
import { BuildError } from "../error/index.ts";
import type { DependencyTree } from "../dependency/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { isHostModule, type PluginOption, runtimeConditions } from "@destack/package/build";
import { HOST_RUNTIME, type Runtime, WORKERD_SCHEME } from "@destack/package/runtime";
import { selectExport } from "../source/source.ts";
import { loadExtensions, transformPlugins } from "../compile/extension.ts";

/** An installed JavaScript module, imported as the host loads it, without a query such as ?url. */
const INSTALLED_MODULE = /\.[cm]?js$/u;
/** A stylesheet, which declarations evaluate without. */
const STYLESHEET = /\.css(?:\?.*)?$/u;
/** The prefix of the module standing in for a workerd module, which the host lacks. */
const UNAVAILABLE_PREFIX = "\0destack-unavailable:";
/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** The exports of the generated declaration module, aligned by declaration. */
const DeclarationExports = schema.object({
    /** The descriptions each kind's describe function returns. */
    default: schema.array(schema.unknown()),
    /** The exported declarations. */
    declared: schema.array(schema.unknown()),
    /** The value each declaration's kind inspects, read through the kind's value function. */
    values: schema.array(schema.unknown()),
    /** Each declaration's kind comparison, if it has one. */
    compare: schema.array(schema.unknown()),
    /** Each declaration's kind vocabulary, if it has one. */
    vocabulary: schema.array(schema.unknown()),
    /** Each declaration's kind symbols, if it lists them. */
    symbols: schema.array(schema.unknown()),
    /** The properties of each example, as its module evaluates them. */
    examples: schema.array(schema.jsonObject()),
});

/** The exports of the evaluated entry. */
type EvaluatedEntry = schema.Infer<typeof DeclarationExports>;

/** The identifier of the generated entry. */
const ENTRY = "\0destack-declarations";

/** The name a description gives its declaration. */
const DescriptionName = schema.looseObject({ name: schema.string() });

/** The terms a kind's vocabulary defines for a declaration, by name. */
const Terms = schema.record(schema.string(), schema.unknown());

/** The symbols a kind derives for a declaration. */
const Symbols = schema.array(graph.MemberSymbol);

/** Evaluated declarations and each kind's compare function. */
export interface Evaluation {
    /** The declarations' descriptions, with their terms. */
    readonly declarations: DeclarationDescription[];
    /** The function comparing two releases of a kind, by `kindKey`. */
    readonly compare: ReadonlyMap<string, Comparator>;
    /** The examples, with the properties their modules evaluate. */
    readonly examples: ExampleDeclaration[];
}

/** Key a kind by its declaring package and name. */
export function kindKey(declaration: Pick<graph.Declaration, "kind" | "kindPackageId">): string {
    return JSON.stringify([declaration.kindPackageId, declaration.kind]);
}

/** The declarations a build evaluates in its worker. */
export const Evaluation = {
    /** Evaluate exported declarations together in the current build worker. */
    async evaluate(
        declarations: readonly DeclarationExport[],
        examples: readonly Omit<ExampleDeclaration, "properties">[],
        project: PackageSource,
    ): Promise<Evaluation> {
        if (!declarations.length && !examples.length) {
            return { declarations: [], compare: new Map(), examples: [] };
        }

        // bundle the package's declaring and example modules
        const source = generateEntry(declarations, examples, project.directory);
        const namespace = await evaluateEntry(source, project).catch((cause: unknown) => {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `declaration evaluation failed: ${messageOf(cause)}`,
                { cause },
            );
        });

        // require every declaration to record the package declaring it
        for (const [index, declaration] of declarations.entries()) {
            requireDeclaringPackage(declaration, namespace.declared[index]);
        }

        // describe each declaration beside its kind's comparison
        const described: DeclarationDescription[] = [];
        const compare = new Map<string, Comparator>();
        for (const [index, declaration] of declarations.entries()) {
            const { kind, name } = declaration.description;
            const description = await describeDeclaration(
                declaration,
                index,
                namespace,
                compare,
                project.environment.crypto,
            ).catch((cause: unknown) => {
                throw BuildError.of(
                    "INSPECTION_FAILED",
                    `declaration evaluation failed: ${kind} ${name} (${declaration.file}): ${messageOf(cause)}`,
                    { cause },
                );
            });
            described.push(description);
        }

        // give each example the properties its module evaluates
        const evaluated = examples.map((example, index) => ({
            ...example,
            properties: present(namespace.examples[index], `the properties of ${example.symbol}`),
        }));

        return { declarations: described, compare, examples: evaluated };
    },
};

/** Write the entry importing each declaration and inspector, exporting the kinds' results. */
function generateEntry(
    declarations: readonly DeclarationExport[],
    examples: readonly Omit<ExampleDeclaration, "properties">[],
    directory: string,
): string {
    // select each inspecting module once, as the build worker runs it
    const conditions = new Set(["import", "default", ...runtimeConditions(HOST_RUNTIME)]);
    const modules = new Map<string, number>();
    const locate = (inspector: FunctionExport) => locateFunction(inspector, conditions, modules);
    const functions = declarations.map(({ inspector }) => ({
        value: inspector.value && locate(inspector.value),
        describe: locate(inspector.describe),
        compare: inspector.compare && locate(inspector.compare),
        vocabulary: inspector.vocabulary && locate(inspector.vocabulary),
        symbols: inspector.symbols && locate(inspector.symbols),
    }));

    // import declarations and their inspecting modules into the same module graph
    const imports = [
        ...declarations.map(
            (declaration, index) =>
                `import { ${JSON.stringify(declaration.export)} as declaration${index} } from ${JSON.stringify(declaration.file)};`,
        ),
        ...examples.map(
            (example, index) =>
                `import { ${JSON.stringify(example.symbol)} as example${index} } from ${JSON.stringify(join(directory, example.source.file))};`,
        ),
        ...[...modules].map(
            ([specifier, index]) =>
                `import * as inspector${index} from ${JSON.stringify(specifier)};`,
        ),
    ];

    // list each kind's functions, read each kind's value, and describe it
    const list = (name: "compare" | "vocabulary" | "symbols") =>
        functions.map((entry) => entry[name] ?? "undefined").join(",\n");
    const names = functions.map((_, index) => `declaration${index}`);
    const values = functions.map((entry, index) =>
        entry.value === undefined ? `declaration${index}` : `${entry.value}(declaration${index})`,
    );
    const calls = functions.map((entry, index) => `${entry.describe}(values[${index}])`);

    return [
        ...imports,
        `export const declared = [${names.join(", ")}];`,
        `export const values = [${values.join(", ")}];`,
        `export const compare = [${list("compare")}];`,
        `export const vocabulary = [${list("vocabulary")}];`,
        `export const symbols = [${list("symbols")}];`,
        `export const examples = [${examples.map((_, index) => `example${index}.properties`).join(", ")}];`,
        `export default await Promise.all([${calls.join(",\n")}]);`,
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

/** Bundle the generated entry and evaluate it as a fresh module, past the import cache. */
async function evaluateEntry(source: string, project: PackageSource): Promise<EvaluatedEntry> {
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    try {
        // bundle the entry with the declarations it imports
        bundle = await rolldown({
            input: ENTRY,
            cwd: project.directory,
            tsconfig: project.configuration,
            platform: "node",
            external: isHostModule,
            resolve: { conditionNames: runtimeConditions(HOST_RUNTIME) },
            treeshake: { moduleSideEffects: true },
            plugins: [
                entryPlugin(source),
                treePlugin(project.tree, project.runtime, project.environment.files),
                modulePlugin(project.environment.files, project.versions),
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

        return DeclarationExports.parse(module.namespace);
    } finally {
        await bundle?.close();
    }
}

/** Link an external of the entry as the host loads it: a builtin, a build module or a file. */
async function linkHost(specifier: string): Promise<SyntheticModule> {
    // expose the host module's exports, which the host caches once per file
    const exports = Exports.parse(await import(specifier));
    const names = Object.keys(exports);

    return new SyntheticModule(names, function () {
        for (const name of names) {
            this.setExport(name, exports[name]);
        }
    });
}

/** Mark dependencies external: Destack ones at their builds' modules, npm ones at their files. */
function treePlugin(tree: DependencyTree, runtime: Runtime, files: FileSystem): Plugin {
    return {
        name: "destack-dependency-builds",
        async resolveId(specifier, importer, options) {
            // import a Destack dependency's module from its build, else the output runtime's
            const modules = tree.export(specifier);
            if (modules !== undefined) {
                const module = modules[HOST_RUNTIME] ?? modules[runtime];
                if (module === undefined) {
                    throw BuildError.of(
                        "INSPECTION_FAILED",
                        `${specifier} has no ${[...new Set([HOST_RUNTIME, runtime])].join(" or ")} module`,
                    );
                }

                return { id: module, external: true };
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
            // resolve the generated entry, and a workerd module as one failing once touched
            if (specifier === ENTRY) {
                return ENTRY;
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
            // load the entry, a stand-in for a workerd module, and a stylesheet as nothing
            if (id === ENTRY) {
                return source;
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

/** Describe one evaluated declaration with its terms and symbols, with its kind's comparison. */
async function describeDeclaration(
    declaration: DeclarationExport,
    index: number,
    namespace: EvaluatedEntry,
    compare: Map<string, Comparator>,
    crypto: DigestCrypto,
): Promise<DeclarationDescription> {
    // serialize the domain description under the declaration's described name
    const description: unknown = JSON.parse(stringifyInspection(namespace.default[index]));
    const parsed = DeclarationDescription.parse({
        ...declaration.description,
        name: nameOf(description, declaration.description.name),
        description,
    });

    // digest each term's definition the kind's code returns for the declaration
    const declared = namespace.values[index];
    const vocabularyOf = namespace.vocabulary[index];
    const terms =
        typeof vocabularyOf === "function"
            ? Terms.parse(Reflect.apply(vocabularyOf, undefined, [declared]))
            : undefined;
    const vocabulary: Record<string, Digest> = {};
    for (const [term, definition] of Object.entries(terms ?? {})) {
        vocabulary[`${parsed.kind}/${term}`] = await Digest.json(definition, crypto);
    }

    // list the symbols the kind's code derives from the declaration
    const symbolsOf = namespace.symbols[index];
    const symbols =
        typeof symbolsOf === "function"
            ? Symbols.parse(Reflect.apply(symbolsOf, undefined, [declared]))
            : undefined;

    // take the kind's comparison and check each plan it returns
    const comparison = namespace.compare[index];
    if (typeof comparison === "function") {
        compare.set(
            kindKey({ kind: parsed.kind, kindPackageId: parsed.package.id }),
            (before, after) => Plan.parse(Reflect.apply(comparison, undefined, [before, after])),
        );
    }

    return {
        ...parsed,
        ...(terms === undefined ? {} : { vocabulary }),
        ...(symbols === undefined ? {} : { symbols }),
    };
}

/** Read the name a description gives, else the declaration's symbol name. */
function nameOf(description: unknown, fallback: string): string {
    const name = DescriptionName.safeParse(description);

    return name.success ? name.data.name : fallback;
}

/** Require an evaluated declaration's package to be the one the compiler located. */
function requireDeclaringPackage(declaration: DeclarationExport, value: unknown): void {
    const stamped = Package.declaring(value);
    const owner = declaration.description.symbol.package;

    // refuse a declaration without a package
    if (stamped === undefined) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `${declaration.export} declaration has no package: set it in ${declaration.description.constructor.symbol.name}`,
        );
    }
    // refuse a declaration of another package
    else if (
        stamped.id !== owner.id ||
        stamped.name !== owner.name ||
        stamped.version !== owner.version
    ) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `${declaration.export} declaration belongs to a different package`,
        );
    }
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
