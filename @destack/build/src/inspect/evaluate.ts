import { fileURLToPath, pathToFileURL } from "node:url";
import { isBuiltin } from "node:module";
import { SourceTextModule, SyntheticModule } from "node:vm";
import { rolldown } from "rolldown";
import { transform } from "rolldown/utils";
import { DeclarationDescription } from "@destack/package/inspect";
import type { Declaration } from "./declaration.ts";
import type { PackageSource } from "../source/index.ts";
import { metadataPlugin } from "../compile/source.ts";
import { BuildError } from "../error/index.ts";
import { stringifyInspection } from "./inspection.ts";
import { runtimeConditions } from "../compile/runtime.ts";

/** Evaluate exported declarations together in the current build worker. */
export async function evaluateDeclarations(
    declarations: readonly Declaration[],
    project: PackageSource,
): Promise<DeclarationDescription[]> {
    if (!declarations.length) {
        return [];
    }

    // import declarations and inspectors into the same module graph
    const inspector = fileURLToPath(new URL("./inspector.ts", import.meta.url));
    const imports = declarations.map(
        (declaration, index) =>
            `import { ${JSON.stringify(declaration.export)} as declaration${index} } from ${JSON.stringify(
                declaration.file,
            )};`,
    );

    // call each registered inspector with its declaring package
    const descriptions = declarations.map(
        (declaration, index) =>
            `inspectDeclaration(${JSON.stringify(declaration.inspector)}, declaration${index}, ${JSON.stringify(
                declaration.description.symbol.package,
            )})`,
    );

    // collect all descriptions through one generated entry
    const source = [
        `import { inspectDeclaration } from ${JSON.stringify(inspector)};`,
        ...imports,
        `export default await Promise.all([${descriptions.join(",\n")}]);`,
    ].join("\n");
    const entry = "\0destack-declarations";

    // rebuild the complete graph so edits to imported helpers cannot leave stale values
    let bundle: Awaited<ReturnType<typeof rolldown>> | undefined;
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
                metadataPlugin(
                    project.directory,
                    project.declaration.package,
                    project.configuration,
                ),
            ],
        });

        // evaluate a fresh module without adding bundles to the process import cache
        const generated = await bundle.generate({ format: "esm", codeSplitting: false });
        if (generated.output.length !== 1 || generated.output[0].type !== "chunk") {
            throw new BuildError(
                "INSPECTION_FAILED",
                "Declaration evaluation requires one JavaScript module.",
            );
        }

        // link runtime builtins into the fresh module
        const module = new SourceTextModule(generated.output[0].code);
        await module.link(async (specifier) => {
            if (!isBuiltin(specifier)) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `Unbundled declaration import: ${specifier}`,
                );
            }

            const exports = await import(specifier);
            const names = Object.keys(exports);

            return new SyntheticModule(names, function () {
                for (const name of names) {
                    this.setExport(name, exports[name]);
                }
            });
        });

        // evaluate exported declarations before serializing descriptions
        await module.evaluate();
        const values = (module.namespace as { default: unknown[] }).default;

        // serialize domain descriptions while retaining compiler symbol ownership
        return declarations.map((declaration, index) => {
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
        throw new BuildError("INSPECTION_FAILED", "Declaration evaluation failed.", { cause });
    } finally {
        await bundle?.close();
    }
}
