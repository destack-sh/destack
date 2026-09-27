import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { isBuiltin } from "node:module";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import { DeclarationDescription } from "@destack/package/inspect";
import type { Package } from "@destack/package";
import type { DeclarationExport } from "./declaration.ts";
import type { PackageSource } from "../source/index.ts";
import { modulePlugin } from "@destack/package/transform/vite";
import { BuildError } from "../error/index.ts";
import { stringifyInspection } from "../build/serialization.ts";
import { runtimeConditions } from "../compile/runtime.ts";
import { selectExport } from "../source/source.ts";

/** Evaluate exported declarations together in the current build worker. */
export async function evaluateDeclarations(
    declarations: readonly DeclarationExport[],
    project: PackageSource,
): Promise<DeclarationDescription[]> {
    if (!declarations.length) {
        return [];
    }

    // select each describing module under the build's runtime conditions
    const conditions = new Set(["import", "default", ...runtimeConditions(project.runtime)]);
    const inspectors = declarations.map(({ inspector }) => {
        const path = selectExport(inspector.target, conditions);
        if (typeof path !== "string") {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${inspector.directory} exports no ${project.runtime} module at ${inspector.subpath}`,
            );
        }

        return join(inspector.directory, path);
    });

    // import declarations and their describing modules into the same module graph
    const imports = declarations.flatMap((declaration, index) => [
        `import { ${JSON.stringify(declaration.export)} as declaration${index} } from ${JSON.stringify(declaration.file)};`,
        `import * as inspector${index} from ${JSON.stringify(inspectors[index])};`,
    ]);

    // describe each declaration with its constructor's function
    const descriptions = declarations.map(
        (declaration, index) =>
            `inspector${index}[${JSON.stringify(declaration.inspector.name)}](declaration${index})`,
    );

    // collect all descriptions through one generated entry
    const source = [
        ...imports,
        `export const declared = [${declarations.map((_, index) => `declaration${index}`).join(", ")}];`,
        `export default await Promise.all([${descriptions.join(",\n")}]);`,
    ].join("\n");
    const entry = "\0destack-declarations";

    // rebuild the complete graph so edits to imported modules cannot leave stale values
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
    let declared: unknown[];
    let described: DeclarationDescription[];
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
                    resolveId: { filter: { id: /^\0destack-declarations$/ }, handler: () => entry },
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
        const namespace = module.namespace as { default: unknown[]; declared: unknown[] };
        const values = namespace.default;
        declared = namespace.declared;

        // serialize domain descriptions while retaining compiler symbol ownership
        described = declarations.map((declaration, index) => {
            const description = JSON.parse(stringifyInspection(values[index]));

            return DeclarationDescription.parse({
                ...declaration.description,
                name:
                    typeof description.name === "string"
                        ? description.name
                        : declaration.description.name,
                description,
            });
        });
    } catch (cause) {
        throw new BuildError("INSPECTION_FAILED", "declaration evaluation failed", { cause });
    } finally {
        await bundle?.close();
    }

    // require every declaration to carry the package declaring it
    for (const [index, declaration] of declarations.entries()) {
        const stamped = (declared[index] as { package?: Package } | null)?.package;
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

    return described;
}
