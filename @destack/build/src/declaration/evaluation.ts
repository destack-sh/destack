import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { isBuiltin } from "node:module";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import { DeclarationDescription } from "@destack/package/inspect";
import { Package } from "@destack/package";
import type { DeclarationExport, FunctionExport } from "./declaration.ts";
import type { Compare } from "@destack/resource";
import { digest } from "@destack/schema/json";
import type { PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/transform/vite";
import { BuildError } from "../error/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { runtimeConditions } from "@destack/package/build";
import { selectExport } from "../source/source.ts";

/** Evaluated declarations and each kind's compare function. */
export interface Evaluation {
    /** The declarations' descriptions, with their terms. */
    readonly declarations: DeclarationDescription[];
    /** The function comparing two releases of a kind, by `kindKey`. */
    readonly compare: ReadonlyMap<string, Compare>;
}

/** Key a kind by its declaring package and name. */
export function kindKey(declaration: Pick<DeclarationDescription, "kind" | "package">): string {
    return JSON.stringify([declaration.package.id, declaration.kind]);
}

/** Evaluate exported declarations together in the current build worker. */
export async function evaluateDeclarations(
    declarations: readonly DeclarationExport[],
    project: PackageSource,
): Promise<Evaluation> {
    if (!declarations.length) {
        return { declarations: [], compare: new Map() };
    }

    // select each inspecting module once
    const conditions = new Set(["import", "default", ...runtimeConditions(project.runtime)]);
    const modules = new Map<string, number>();
    const locate = (inspector: FunctionExport) => {
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
    };
    const functions = declarations.map(({ inspector }) => ({
        describe: locate(inspector.describe),
        compare: inspector.compare === undefined ? "undefined" : locate(inspector.compare),
        vocabulary: inspector.vocabulary === undefined ? "undefined" : locate(inspector.vocabulary),
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

    // call each kind's functions
    const list = (name: "describe" | "compare" | "vocabulary", call = false) =>
        functions
            .map((entry, index) => (call ? `${entry[name]}(declaration${index})` : entry[name]))
            .join(",\n");

    // collect all descriptions through one generated entry
    const source = [
        ...imports,
        `export const declared = [${declarations.map((_, index) => `declaration${index}`).join(", ")}];`,
        `export const compare = [${list("compare")}];`,
        `export const vocabulary = [${list("vocabulary")}];`,
        `export default await Promise.all([${list("describe", true)}]);`,
    ].join("\n");
    const entry = "\0destack-declarations";

    // rebuild the complete graph so edits to imported modules cannot leave stale values
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    let declared: unknown[];
    let described: DeclarationDescription[];
    const compare = new Map<string, Compare>();
    try {
        bundle = await rolldown({
            input: entry,
            cwd: project.directory,
            tsconfig: project.configuration,
            platform: "node",
            external: isBuiltin,
            resolve: { conditionNames: runtimeConditions(project.runtime) },
            treeshake: { moduleSideEffects: true },
            plugins: [
                {
                    name: "destack-declarations",
                    resolveId(source, _importer, options) {
                        // resolve the generated entry
                        if (source === entry) {
                            return entry;
                        }
                        // leave lazily imported modules, such as view components, unloaded
                        else if (options.kind === "dynamic-import") {
                            return { id: source, external: true };
                        }
                    },
                    load: { filter: { id: /^\0destack-declarations$/ }, handler: () => source },
                    transform: {
                        filter: { id: /\.[cm]?[jt]sx?$/, code: /import\.meta\.url/ },
                        async handler(code, id) {
                            const result = await transform(id, code, {
                                define: {
                                    "import.meta.url": JSON.stringify(pathToFileURL(id).href),
                                },
                                sourcemap: true,
                            });
                            if (result.errors.length) {
                                throw new BuildError(
                                    "INSPECTION_FAILED",
                                    JSON.stringify(result.errors),
                                );
                            }

                            return { code: result.code, map: result.map };
                        },
                    },
                },
                modulePlugin(),
            ],
        });

        // evaluate a fresh module without adding bundles to the process import cache
        const generated = await bundle.generate({ format: "esm", codeSplitting: false });
        if (generated.output.length !== 1 || generated.output[0].type !== "chunk") {
            throw new BuildError(
                "INSPECTION_FAILED",
                "declaration evaluation requires one JavaScript module",
            );
        }

        // link runtime builtins into the fresh module
        const module = new SourceTextModule(generated.output[0].code);
        await module.link(async (specifier) => {
            if (!isBuiltin(specifier)) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `unbundled declaration import: ${specifier}`,
                );
            }

            // expose the imported module's exports
            const exports = await import(specifier);
            const names = Object.keys(exports);

            return new SyntheticModule(names, function () {
                for (const name of names) {
                    this.setExport(name, exports[name]);
                }
            });
        });

        // evaluate exported declarations, remembering them beside their descriptions
        await module.evaluate();
        const namespace = module.namespace as {
            default: unknown[];
            declared: unknown[];
            compare: (Compare | undefined)[];
            vocabulary: (((description: unknown) => Record<string, unknown>) | undefined)[];
        };
        const values = namespace.default;
        declared = namespace.declared;

        // serialize domain descriptions while retaining compiler symbol ownership
        described = [];
        for (const [index, declaration] of declarations.entries()) {
            const description = JSON.parse(stringifyInspection(values[index]));
            const parsed = DeclarationDescription.parse({
                ...declaration.description,
                name:
                    typeof description.name === "string"
                        ? description.name
                        : declaration.description.name,
                description,
            });

            // digest each term's definition
            const terms = namespace.vocabulary[index]?.(parsed.description);
            const vocabulary: Record<string, string> = {};
            for (const [term, definition] of Object.entries(terms ?? {})) {
                vocabulary[`${parsed.kind}/${term}`] = await digest(definition);
            }
            described.push(terms === undefined ? parsed : { ...parsed, vocabulary });

            // keep the kind's comparison
            const comparison = namespace.compare[index];
            if (comparison !== undefined) {
                compare.set(kindKey(parsed), comparison);
            }
        }
    } catch (cause) {
        throw new BuildError("INSPECTION_FAILED", "declaration evaluation failed", { cause });
    } finally {
        await bundle?.close();
    }

    // require every declaration to carry the package declaring it
    for (const [index, declaration] of declarations.entries()) {
        const stamped = Package.declaring(declared[index]);
        const owner = declaration.description.symbol.package;
        if (stamped === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${declaration.export} declaration lacks its declaring package; stamp it in ${declaration.description.constructor.symbol.name}`,
            );
        } else if (
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

    return { declarations: described, compare };
}
