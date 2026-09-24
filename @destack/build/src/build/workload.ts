import { relative, sep } from "node:path";
import { mergeCompute } from "@destack/package";
import { WorkloadDefinition, WorkloadDescription } from "@destack/package/workload";
import { type DeclarationReference, reference } from "@destack/package/declare";
import type { ModuleDescription } from "@destack/package/code";
import type { BuildDescription, DeclarationDescription } from "@destack/package/inspect";
import type { PackageSource } from "../source/index.ts";
import { BuildError } from "../error/index.ts";
import { isRuntimeModule } from "../compile/runtime.ts";

/** Describe declared workloads with the declarations and runtime imports their code reaches. */
export async function describeWorkloads(
    declarations: DeclarationDescription[],
    modules: ModuleDescription[],
    project: PackageSource,
    exports: Record<string, string>,
    build: BuildDescription,
    isServerRendered: boolean,
): Promise<Record<string, WorkloadDescription>> {
    // describe no workloads for browser packages
    const result: Record<string, WorkloadDescription> = {};
    if (project.runtime === "browser") {
        return result;
    }

    // index domain declarations and reject ambiguous names
    const declared = new Map<string, DeclarationDescription>();
    for (const declaration of declarations) {
        if (
            !["resource", "secret", "service", "schedule", "service-connection"].includes(
                declaration.kind,
            )
        ) {
            continue;
        }
        const key = `${declaration.symbol.package.id}:${declaration.kind}:${declaration.name}`;
        if (declared.has(key)) {
            throw invalid(`Duplicate declaration: ${key}`);
        }
        declared.set(key, declaration);
    }

    // describe workloads this package declares and this output exports
    for (const workload of declarations) {
        const owner = workload.symbol.package;
        if (workload.kind !== "workload" || owner.id !== project.declaration.package.id) {
            continue;
        }
        const located = locateExport(workload, modules, project, exports);
        if (!located) {
            continue;
        }
        const { entrypoint, name: exportName, path } = located;
        const { name } = workload;
        if (Object.hasOwn(result, name)) {
            throw invalid(`Duplicate workload: ${name}`);
        }
        const emitted = build.outputs[exports[entrypoint]];
        if (!emitted) {
            throw invalid(`Workload ${name} has no compiled entrypoint: ${entrypoint}`);
        }

        // inspect the transitive emitted inputs before selecting a runtime
        const pending: string[] = [];
        const chunks = [exports[entrypoint]];
        const visitedChunks = new Set<string>();
        for (const path of chunks) {
            if (visitedChunks.has(path)) {
                continue;
            }
            visitedChunks.add(path);
            const chunk = build.outputs[path];
            if (!chunk) {
                throw invalid(`Unknown emitted chunk: ${path}`);
            }

            // collect source inputs and follow emitted imports
            pending.push(...chunk.inputs);
            for (const imported of chunk.imports) {
                if (!imported.external) {
                    chunks.push(imported.path);
                } else if (!isRuntimeModule(imported.path, project.runtime)) {
                    throw invalid(
                        `Unsupported external import for ${project.runtime}: ${imported.path}`,
                    );
                }
            }
        }

        // follow source imports from this entrypoint, including tree-shaken declarations
        const roots = Object.entries(build.inputs)
            .filter(([, input]) => !input.package && input.path.split("?")[0] === path)
            .map(([id]) => id);
        if (!roots.length && !isServerRendered) {
            throw invalid(`Missing workload input: ${name}`);
        }

        // traverse each source module once
        const reachable = new Set<string>();
        const sources = isServerRendered ? [...pending, ...roots] : [...roots];
        const paths = new Map<string, Set<string>>();
        const resources: DeclarationReference[] = [];
        const secrets: DeclarationReference[] = [];
        const connections: DeclarationReference[] = [];
        const services: DeclarationReference[] = [];
        const schedules: DeclarationReference[] = [];
        for (const id of sources) {
            if (reachable.has(id)) {
                continue;
            }
            reachable.add(id);
            const input = build.inputs[id];
            if (!input) {
                throw invalid(`Unknown source module: ${id}`);
            }

            // index reachable source paths by their declaring package
            const owner = input.package
                ? build.packages[input.package]?.package
                : project.declaration.package;
            if (input.package && !owner) {
                throw invalid(`Unknown source package: ${input.package}`);
            }
            if (owner) {
                const key = `${owner.name}@${owner.version}`;
                const files = paths.get(key) ?? new Set<string>();
                files.add(input.path.split("?")[0]);
                paths.set(key, files);
            }

            // follow internal source imports
            for (const imported of input.imports) {
                if (!imported.external) {
                    sources.push(imported.path);
                }
            }
        }

        // retain qualified references once per declaration
        const selections: Partial<Record<string, DeclarationReference[]>> = {
            resource: resources,
            secret: secrets,
            "service-connection": connections,
            service: services,
            schedule: schedules,
        };
        for (const declaration of declared.values()) {
            // select declarations present in reachable modules
            const owner = declaration.symbol.package;
            const used = paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file);
            if (!used) {
                continue;
            }

            // implement only services and schedules this package declares
            const isImplemented = declaration.kind === "service" || declaration.kind === "schedule";
            if (isImplemented && owner.id !== project.declaration.package.id) {
                continue;
            }
            selections[declaration.kind]?.push(
                reference({ package: owner, name: declaration.name }),
            );
        }

        // reject lifetime and capacity guarantees unavailable in worker isolates
        const compute = mergeCompute(WorkloadDefinition.parse(workload.description).compute);
        if (
            project.runtime === "workerd" &&
            (Object.keys(compute.requests ?? {}).length ||
                Object.keys(compute.limits ?? {}).length ||
                Object.keys(compute.scaling ?? {}).length ||
                compute.idleTimeout !== undefined ||
                compute.shutdownTimeout !== undefined)
        ) {
            throw invalid(`Workload ${name} requires instance management unavailable in workerd.`);
        }

        result[name] = WorkloadDescription.parse({
            entrypoint,
            export: exportName,
            services,
            schedules,
            resources,
            secrets,
            connections,
            compute,
        });
    }

    return result;
}

/** Find the first package export in this output exposing a workload declaration. */
function locateExport(
    workload: DeclarationDescription,
    modules: readonly ModuleDescription[],
    project: PackageSource,
    exports: Record<string, string>,
): { entrypoint: string; name: string; path: string } | undefined {
    const { module, name } = workload.symbol.symbol;
    for (const [entrypoint, file] of Object.entries(project.exports)) {
        // skip exports this output does not compile
        if (!exports[entrypoint]) {
            continue;
        }

        // match exports resolving to the declared constant
        const path = relative(project.directory, file).split(sep).join("/");
        const exported = modules
            .find((entry) => entry.path === path)
            ?.exports.find(
                (entry) =>
                    !entry.isTypeOnly &&
                    "module" in entry.symbol &&
                    entry.symbol.module === module &&
                    entry.symbol.name === name,
            );
        if (exported) {
            return { entrypoint, name: exported.name, path };
        }
    }

    return undefined;
}

/** Report an invalid workload. */
function invalid(message: string): BuildError {
    return new BuildError("BUILD_FAILED", message);
}
