import { Digest, schema } from "@destack/schema";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { isBuiltin } from "node:module";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { type Plugin, rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import { graph, Package, DeclarationDescription } from "@destack/package";
import type { DeclarationExport, FunctionExport } from "./declaration.ts";
import { type Comparator, Plan } from "@destack/resource";
import type { PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/vite";
import { BuildError } from "../error/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { runtimeConditions } from "@destack/package/build";
import { compiledPackage, selectExport } from "../source/source.ts";
import { loadExtensions, transformPlugins } from "../compile/extension.ts";

/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** The exports of the generated declaration module, aligned by declaration. */
const DeclarationExports = schema.object({
    /** The descriptions each kind's describe function returned. */
    default: schema.array(schema.unknown()),
    /** The exported declarations. */
    declared: schema.array(schema.unknown()),
    /** Each declaration's kind comparison, if it has one. */
    compare: schema.array(schema.unknown()),
    /** Each declaration's kind vocabulary, if it has one. */
    vocabulary: schema.array(schema.unknown()),
    /** Each declaration's kind symbols, if it lists them. */
    symbols: schema.array(schema.unknown()),
});

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
}

/** Key a kind by its declaring package and name. */
export function kindKey(declaration: Pick<graph.Declaration, "kind" | "package">): string {
    return JSON.stringify([declaration.package, declaration.kind]);
}

/** Evaluate exported declarations together in the current build worker. */
export async function evaluateDeclarations(
    declarations: readonly DeclarationExport[],
    project: PackageSource,
): Promise<Evaluation> {
    if (!declarations.length) {
        return { declarations: [], compare: new Map() };
    }

    // rebuild the complete graph so edits to imported modules cannot leave stale values
    const source = generateEntry(declarations, project);
    let declared: unknown[];
    const described: DeclarationDescription[] = [];
    const compare = new Map<string, Comparator>();
    try {
        // evaluate the declarations, then describe each beside its kind's comparison
        const namespace = await evaluateEntry(source, project);
        declared = namespace.declared;
        for (const [index, declaration] of declarations.entries()) {
            described.push(await describeDeclaration(declaration, index, namespace, compare));
        }
    } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        throw new BuildError("INSPECTION_FAILED", `declaration evaluation failed: ${message}`, {
            cause,
        });
    }

    // require every declaration to name the package declaring it
    for (const [index, declaration] of declarations.entries()) {
        requireDeclaringPackage(declaration, declared[index]);
    }

    return { declarations: described, compare };
}

/** Write the entry importing each declaration and its inspecting modules, exporting what the kinds' functions return. */
function generateEntry(declarations: readonly DeclarationExport[], project: PackageSource): string {
    // select each inspecting module once
    const conditions = new Set(["import", "default", ...runtimeConditions(project.runtime)]);
    const modules = new Map<string, number>();
    const locate = (inspector: FunctionExport | undefined) =>
        inspector === undefined
            ? "undefined"
            : locateFunction(inspector, conditions, modules, project);
    const functions = declarations.map(({ inspector }) => ({
        describe: locate(inspector.describe),
        compare: locate(inspector.compare),
        vocabulary: locate(inspector.vocabulary),
        symbols: locate(inspector.symbols),
    }));

    // import declarations and their inspecting modules into the same module graph
    const imports = [
        ...declarations.map(
            (declaration, index) =>
                `import { ${JSON.stringify(declaration.export)} as declaration${index} } from ${JSON.stringify(declaration.file)};`,
        ),
        ...[...modules].map(
            ([file, index]) => `import * as inspector${index} from ${JSON.stringify(file)};`,
        ),
    ];

    // list each kind's functions, or call them on the declarations
    const list = (name: keyof (typeof functions)[number]) =>
        functions.map((entry) => entry[name]).join(",\n");
    const calls = functions.map((entry, index) => `${entry.describe}(declaration${index})`);
    const names = declarations.map((_, index) => `declaration${index}`);

    return [
        ...imports,
        `export const declared = [${names.join(", ")}];`,
        `export const compare = [${list("compare")}];`,
        `export const vocabulary = [${list("vocabulary")}];`,
        `export const symbols = [${list("symbols")}];`,
        `export default await Promise.all([${calls.join(",\n")}]);`,
    ].join("\n");
}

/** Name a function in the generated entry, importing its module once. */
function locateFunction(
    inspector: FunctionExport,
    conditions: ReadonlySet<string>,
    modules: Map<string, number>,
    project: PackageSource,
): string {
    // require the module the runtime selects
    const path = selectExport(inspector.target, conditions);
    if (typeof path !== "string") {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${inspector.directory} exports no ${project.runtime} module at ${inspector.subpath}`,
        );
    }

    // import each module once, reading the function from its namespace
    const file = join(inspector.directory, path);
    const index = modules.get(file) ?? modules.size;
    modules.set(file, index);

    return `inspector${index}[${JSON.stringify(inspector.name)}]`;
}

/** Bundle the generated entry and evaluate it as a fresh module, outside the process import cache. */
async function evaluateEntry(
    source: string,
    project: PackageSource,
): Promise<schema.Infer<typeof DeclarationExports>> {
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    try {
        // bundle the entry with the declarations it imports
        bundle = await rolldown({
            input: ENTRY,
            cwd: project.directory,
            tsconfig: project.configuration,
            platform: "node",
            external: isBuiltin,
            resolve: { conditionNames: runtimeConditions(project.runtime) },
            treeshake: { moduleSideEffects: true },
            plugins: [
                entryPlugin(source),
                modulePlugin(compiledPackage(project)),
                ...transformPlugins(
                    (await loadExtensions(project.directory, project.declaration)).map(
                        (loaded) => loaded.extension,
                    ),
                    {
                        directory: project.directory,
                        runtime: project.runtime,
                        server: false,
                        options: {},
                    },
                ),
            ],
        });
        const generated = await bundle.generate({ format: "esm", codeSplitting: false });
        const chunks = generated.output.filter((file) => file.type === "chunk");
        const [chunk] = chunks;
        if (chunks.length !== 1 || chunk === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                "declaration evaluation requires one JavaScript module",
            );
        }

        // link runtime builtins into the fresh module and evaluate it, leaving emitted assets aside
        const module = new SourceTextModule(chunk.code);
        await module.link(linkBuiltin);
        await module.evaluate();

        return DeclarationExports.parse(module.namespace);
    } finally {
        await bundle?.close();
    }
}

/** Resolve and load the generated entry, defining each module's URL and leaving lazy imports unloaded. */
function entryPlugin(source: string): Plugin {
    return {
        name: "destack-declarations",
        resolveId(specifier, _importer, options) {
            // resolve the generated entry
            if (specifier === ENTRY) {
                return ENTRY;
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
        load: { filter: { id: /^\0destack-declarations$/u }, handler: () => source },
        transform: {
            filter: { id: /\.[cm]?[jt]sx?$/u, code: /import\.meta\.url/u },
            async handler(code, id) {
                // define the module's URL as its source file's
                const result = await transform(id, code, {
                    define: { "import.meta.url": JSON.stringify(pathToFileURL(id).href) },
                    sourcemap: true,
                });
                if (result.errors.length) {
                    throw new BuildError("INSPECTION_FAILED", JSON.stringify(result.errors));
                }

                return {
                    code: result.code,
                    ...(result.map === undefined ? {} : { map: result.map }),
                };
            },
        },
    };
}

/** Link a runtime builtin into a fresh module, refusing any other import. */
async function linkBuiltin(specifier: string): Promise<SyntheticModule> {
    if (!isBuiltin(specifier)) {
        throw new BuildError("INSPECTION_FAILED", `unbundled declaration import: ${specifier}`);
    }

    // expose the imported module's exports
    const exports = Exports.parse(await import(specifier));
    const names = Object.keys(exports);

    return new SyntheticModule(names, function () {
        for (const name of names) {
            this.setExport(name, exports[name]);
        }
    });
}

/** Describe one evaluated declaration with its terms and symbols, keeping its kind's comparison. */
async function describeDeclaration(
    declaration: DeclarationExport,
    index: number,
    namespace: schema.Infer<typeof DeclarationExports>,
    compare: Map<string, Comparator>,
): Promise<DeclarationDescription> {
    // serialize the domain description, naming the declaration as the description names it
    const description: unknown = JSON.parse(stringifyInspection(namespace.default[index]));
    const parsed = DeclarationDescription.parse({
        ...declaration.description,
        name: nameOf(description, declaration.description.name),
        description,
    });

    // digest each term's definition the kind's code returns
    const vocabularyOf = namespace.vocabulary[index];
    const terms =
        typeof vocabularyOf === "function"
            ? Terms.parse(Reflect.apply(vocabularyOf, undefined, [parsed.description]))
            : undefined;
    const vocabulary: Record<string, string> = {};
    for (const [term, definition] of Object.entries(terms ?? {})) {
        vocabulary[`${parsed.kind}/${term}`] = await Digest.json(definition);
    }

    // list the symbols the kind's code derives
    const symbolsOf = namespace.symbols[index];
    const symbols =
        typeof symbolsOf === "function"
            ? Symbols.parse(Reflect.apply(symbolsOf, undefined, [parsed.description]))
            : undefined;

    // keep the kind's comparison and check each plan it returns
    const comparison = namespace.compare[index];
    if (typeof comparison === "function") {
        compare.set(kindKey({ kind: parsed.kind, package: parsed.package.id }), (before, after) =>
            Plan.parse(Reflect.apply(comparison, undefined, [before, after])),
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

/** Require an evaluated declaration to name the package declaring it, the one the compiler located. */
function requireDeclaringPackage(declaration: DeclarationExport, value: unknown): void {
    const stamped = Package.declaring(value);
    const owner = declaration.description.symbol.package;

    // refuse a declaration naming no package
    if (stamped === undefined) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${declaration.export} declaration lacks its declaring package; stamp it in ${declaration.description.constructor.symbol.name}`,
        );
    }
    // refuse a declaration naming another package
    else if (
        stamped.id !== owner.id ||
        stamped.name !== owner.name ||
        stamped.version !== owner.version
    ) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${declaration.export} declaration belongs to a different package`,
        );
    }
}
