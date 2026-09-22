import { relative, sep } from "node:path";
import { mergeCompute } from "@destack/package";
import { type DeclarationReference, WorkloadDescription } from "@destack/package/workload";
import type { ModuleDescription } from "@destack/package/code";
import type { BuildDescription, DeclarationDescription } from "@destack/package/inspect";
import { ScheduleDeclaration } from "@destack/service/schedule";
import { ServiceInspection } from "@destack/service/inspect";
import type { PackageSource } from "../source/index.ts";
import { BuildError } from "../error/index.ts";
import { isRuntimeModule } from "../compile/runtime.ts";

/** Validate workload references, handlers and runtime imports against a compiled output. */
export async function describeWorkloads(
    declarations: DeclarationDescription[],
    modules: ModuleDescription[],
    project: PackageSource,
    exports: Record<string, string>,
    build: BuildDescription,
    generatedHandlers: readonly string[] = [],
): Promise<Record<string, WorkloadDescription>> {
    const result: Record<string, WorkloadDescription> = {};
    if (project.runtime === "browser") {
        return result;
    }

    // index domain declarations and reject ambiguous names
    const declared = new Map<string, DeclarationDescription>();
    for (const declaration of declarations) {
        if (!["resource", "secret", "service", "schedule"].includes(declaration.kind)) {
            continue;
        }
        const key = `${declaration.symbol.package.id}:${declaration.kind}:${declaration.name}`;
        if (declared.has(key)) {
            throw invalid(`Duplicate declaration: ${key}`);
        }
        declared.set(key, declaration);
    }

    // inspect workloads selected by this output
    for (const [name, definition] of Object.entries(
        project.declaration.definition.workloads ?? {},
    )) {
        const entry = project.exports[definition.entrypoint];
        if (!entry) {
            continue;
        }
        if (!exports[definition.entrypoint]) {
            throw invalid(`Workload ${name} has no compiled entrypoint: ${definition.entrypoint}`);
        }

        // locate the authored module and its emitted entry
        const path = relative(project.directory, entry).split(sep).join("/");
        const module = modules.find((entry) => entry.path === path);
        const emitted = build.outputs[exports[definition.entrypoint]];
        if (!module || !emitted) {
            throw invalid(`Missing workload module: ${name}`);
        }

        // resolve service and schedule handlers
        const handlers = Object.values(definition.lifecycle ?? {});
        for (const [kind, names] of [
            ["service", definition.services ?? []],
            ["schedule", definition.schedules ?? []],
        ] as const) {
            const keys = names.map((name) => `${project.declaration.package.id}:${kind}:${name}`);
            if (new Set(keys).size !== keys.length) {
                throw invalid(`Duplicate ${kind} reference in workload: ${name}`);
            }

            // collect handlers from the referenced declarations
            for (const reference of keys) {
                const declaration = declared.get(reference);
                if (!declaration) {
                    throw invalid(`Unknown ${kind}: ${reference} in workload ${name}`);
                }
                if (kind === "schedule") {
                    handlers.push(ScheduleDeclaration.parse(declaration.description).handler);
                }
                if (kind === "service") {
                    handlers.push(ServiceInspection.parse(declaration.description).handler);
                }
            }
        }

        // require at least one callable workload handler
        if (!handlers.length) {
            throw invalid(`Workload has no service, schedule or lifecycle handler: ${name}`);
        }
        for (const handler of handlers) {
            // require adapter-generated handlers to remain exported by the emitted module
            if (generatedHandlers.includes(handler)) {
                if (!emitted.exports.includes(handler)) {
                    throw invalid(`Missing generated handler: ${name}#${handler}`);
                }
                continue;
            }

            // resolve authored handlers through the source module graph
            const exported = module.exports.find(
                (entry) => entry.name === handler && !entry.isTypeOnly,
            );
            if (!exported || !("module" in exported.symbol)) {
                throw invalid(`Handler must resolve to package source: ${name}#${handler}`);
            }

            // require a callable symbol retained in the compiled output
            const reference = exported.symbol;
            const symbol = modules
                .find((entry) => entry.path === reference.module)
                ?.symbols.find((entry) => entry.name === reference.name);
            if (
                !symbol?.signatures.some((signature) => signature.kind === "call") ||
                !emitted.exports.includes(handler)
            ) {
                throw invalid(`Handler is not an emitted callable export: ${name}#${handler}`);
            }
        }

        // inspect the transitive emitted inputs before selecting a runtime
        const pending: string[] = [];
        const chunks = [exports[definition.entrypoint]];
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
        if (!roots.length && !generatedHandlers.length) {
            throw invalid(`Missing workload input: ${name}`);
        }

        // traverse each source module once
        const reachable = new Set<string>();
        const sources = generatedHandlers.length ? [...pending, ...roots] : [...roots];
        const paths = new Map<string, Set<string>>();
        const resources: DeclarationReference[] = [];
        const secrets: DeclarationReference[] = [];
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
        for (const declaration of declared.values()) {
            if (declaration.kind !== "resource" && declaration.kind !== "secret") {
                continue;
            }

            // select declarations present in reachable modules
            const owner = declaration.symbol.package;
            const used = paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file);
            if (!used) {
                continue;
            }

            const reference = { packageId: owner.id, name: declaration.name };
            if (declaration.kind === "resource") {
                resources.push(reference);
            } else {
                secrets.push(reference);
            }
        }

        // reject lifetime and capacity guarantees unavailable in worker isolates
        const compute = mergeCompute(project.declaration.definition.compute, definition.compute);
        if (
            project.runtime === "workerd" &&
            (Object.keys(definition.lifecycle ?? {}).length > 0 ||
                Object.keys(compute.requests ?? {}).length ||
                Object.keys(compute.limits ?? {}).length ||
                Object.keys(compute.scaling ?? {}).length ||
                compute.idleTimeout !== undefined ||
                compute.shutdownTimeout !== undefined)
        ) {
            throw invalid(`Workload ${name} requires instance management unavailable in workerd.`);
        }

        result[name] = WorkloadDescription.parse({ ...definition, resources, secrets, compute });
    }

    return result;
}

/** Report an invalid workload. */
function invalid(message: string): BuildError {
    return new BuildError("BUILD_FAILED", message);
}
