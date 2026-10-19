import {
    Sandbox,
    ComputeDefinition,
    HOST_SANDBOX,
    type DeclarationDescription,
} from "@destack/package";
import type {
    BuildExtension,
    Compilation,
    CompiledOutput,
    DeclarationModule,
    OutputDescription,
    Plugin,
} from "@destack/package/build";
import { DeclarationReference } from "@destack/package/declare";
import { WorkloadDefinition, WorkloadDescription } from "@destack/package/manifest";
import { WORKLOAD_EXPORT } from "@destack/runtime/cloudflare";
import { PackageDescription } from "@destack/runtime";
import { StackError } from "../error/index.ts";
import { HOOK_WORKLOAD } from "../client/hook.ts";
import { SECRET_DECLARATION } from "../declare/binding.ts";
import { RESOURCE_DECLARATION } from "@destack/resource";

/** The prefix of the generated modules running each workload. */
const PREFIX = "virtual:@destack/stack/workload/";

/** The declaration kinds a workload description references. */
const REFERENCED_KINDS = [RESOURCE_DECLARATION, SECRET_DECLARATION, "package", "trigger"] as const;

/** Run each package workload in its own Bun process or Durable Object as its machine starts it. */
export const stackBuild: BuildExtension = {
    compile(compilation) {
        // emit one entry per workload
        const workloads = runnable(compilation);
        for (const workload of workloads) {
            compilation.entry(workload.entrypoint, `${PREFIX}${workload.name}`);
        }

        return [workloadPlugin(workloads)];
    },
    describe(compilation, compiled) {
        // describe each declared workload by the declarations its entry imports, and the hooks workload by its own
        const workloads: Record<string, WorkloadDescription> = {};
        for (const workload of runnable(compilation)) {
            workloads[workload.name] =
                workload.name === HOOK_WORKLOAD
                    ? describeHooks(workload, compilation)
                    : describe(workload, compilation, compiled);
        }

        return { workloads } satisfies OutputDescription;
    },
};

/** A workload an output runs, with the generated module running it. */
interface Runnable {
    /** The workload's name. */
    readonly name: string;
    /** The package entrypoint running it. */
    readonly entrypoint: string;
    /** The workload's definition, as its declaration describes it. */
    readonly definition: unknown;
    /** The generated module running it. */
    readonly code: string;
}

/** List the workloads a server output runs: the package's own, and one serving its hooks on Bun. */
function runnable(compilation: Compilation): Runnable[] {
    // run workloads only in Bun and workerd outputs
    const { runtime } = compilation;
    if (runtime !== "bun" && runtime !== "workerd") {
        return [];
    }

    // import every resource and secret the package declares
    const declarations = owned(compilation);
    const bound = declarations.filter(
        (declaration) =>
            declaration.kind === RESOURCE_DECLARATION || declaration.kind === SECRET_DECLARATION,
    );
    const resources = bound.map((declaration) => ({
        name: declaration.name,
        module: compilation.locate(declaration),
    }));

    // run each declared workload, refusing one under the hooks workload's name
    const declared = declarations.filter((declaration) => declaration.kind === "workload");
    if (declared.some((workload) => workload.name === HOOK_WORKLOAD)) {
        throw new StackError(
            "INVALID_DEFINITION",
            `workload ${HOOK_WORKLOAD} is the name of the workload serving a package's hooks`,
        );
    }
    const workloads = declared.map((workload) => ({
        name: workload.name,
        entrypoint: `./workload/${workload.name}`,
        definition: workload.description,
        code: runModule(
            [importModule(compilation.locate(workload), "workload")],
            "workload",
            resources,
            runtime,
        ),
    }));

    // serve the hooks of the types the package routes with code, in a Bun sandbox
    const hooks = runtime === "bun" ? hookRunnable(compilation, declarations) : undefined;

    return hooks === undefined ? workloads : [...workloads, hooks];
}

/** Derive the workload serving the hooks of the types a package routes, if any. */
function hookRunnable(
    compilation: Compilation,
    declarations: readonly DeclarationDescription[],
): Runnable | undefined {
    // derive none for a package routing no types with code
    const hooked = declarations.filter(hasHooks);
    if (hooked.length === 0) {
        return undefined;
    }

    // start the hooks' server over the package's routes
    const routing = hooked.map((_, index) => `routing${index}`).join(", ");
    const module = JSON.stringify({ package: compilation.package });
    const code = runModule(
        [
            'import { serveHooks } from "@destack/stack/client";',
            'import { defineWorkload } from "@destack/runtime/workload";',
            ...hooked.map((declaration, index) =>
                importModule(compilation.locate(declaration), `routing${index}`),
            ),
            `const workload = defineWorkload({ name: ${JSON.stringify(HOOK_WORKLOAD)}, start: serveHooks([${routing}]) }, ${module});`,
        ],
        "workload",
        [],
        "bun",
    );

    return {
        name: HOOK_WORKLOAD,
        entrypoint: `./workload/${HOOK_WORKLOAD}`,
        definition: { name: HOOK_WORKLOAD },
        code,
    };
}

/** Decide whether a declaration is a package handle routing types with code. */
function hasHooks(declaration: DeclarationDescription): boolean {
    if (declaration.kind !== "package") {
        return false;
    }
    const { objects } = PackageDescription.parse(declaration.description);

    return Object.values(objects).some((routed) => Object.keys(routed.hooks).length > 0);
}

/** Describe the hooks workload: no packages, triggers, bindings or sandbox abilities. */
function describeHooks(workload: Runnable, compilation: Compilation): WorkloadDescription {
    return WorkloadDescription.parse({
        entrypoint: workload.entrypoint,
        packages: [],
        triggers: [],
        resources: [],
        secrets: [],
        compute: ComputeDefinition.merge(),
        sandbox: Sandbox.select(compilation.sandbox, []),
    });
}

/** Describe a workload by the declarations its entry imports and its compute settings. */
function describe(
    workload: Runnable,
    compilation: Compilation,
    compiled: CompiledOutput,
): WorkloadDescription {
    // select each kind's references and implement only the package's own handle and triggers
    const selections: Record<(typeof REFERENCED_KINDS)[number], DeclarationReference[]> = {
        resource: [],
        secret: [],
        package: [],
        trigger: [],
    };
    const seen = new Set<string>();
    for (const declaration of compiled.declarations(workload.entrypoint)) {
        const owner = declaration.symbol.package;
        const kind = REFERENCED_KINDS.find((referenced) => referenced === declaration.kind);
        const isImplemented = kind === "package" || kind === "trigger";
        if (kind === undefined || (isImplemented && owner.id !== compilation.package.id)) {
            continue;
        }

        // refuse a declaration its package declares twice
        const key = `${owner.id}:${declaration.kind}:${declaration.name}`;
        if (seen.has(key)) {
            throw new StackError("INVALID_DEFINITION", `duplicate declaration: ${key}`);
        }
        seen.add(key);
        selections[kind].push(DeclarationReference.of({ package: owner, name: declaration.name }));
    }

    // grant the workload the package's sandbox abilities it lists
    const definition = WorkloadDefinition.parse(workload.definition);

    return WorkloadDescription.parse({
        entrypoint: workload.entrypoint,
        packages: selections.package,
        triggers: selections.trigger,
        resources: selections.resource,
        secrets: selections.secret,
        compute: ComputeDefinition.merge(definition.compute),
        sandbox: Sandbox.select(
            compilation.sandbox,
            definition.sandbox ??
                HOST_SANDBOX.filter((name) => compilation.sandbox[name] !== undefined),
        ),
    });
}

/** Resolve and load the generated module running each workload. */
function workloadPlugin(workloads: readonly Runnable[]): Plugin {
    return {
        name: "@destack/stack",
        resolveId: {
            filter: { id: new RegExp(`^${RegExp.escape(PREFIX)}`, "u") },
            handler: (id) => `\0${id}`,
        },
        load: {
            filter: { id: new RegExp(`^\\0${RegExp.escape(PREFIX)}`, "u") },
            handler: (id) =>
                workloads.find((workload) => `\0${PREFIX}${workload.name}` === id)?.code,
        },
    };
}

/** Write a module running a workload with its declarations and the package's resources. */
function runModule(
    lines: readonly string[],
    workload: string,
    resources: readonly { name: string; module: DeclarationModule }[],
    runtime: "bun" | "workerd",
): string {
    const bound = `{ ${resources.map(({ name }, index) => `${JSON.stringify(name)}: resource${index}`).join(", ")} }`;

    return [
        runtime === "bun"
            ? 'import { hostWorkload } from "@destack/stack/bun";'
            : 'import { hostWorkload } from "@destack/stack/cloudflare";',
        ...resources.map(({ module }, index) => importModule(module, `resource${index}`)),
        ...lines,
        runtime === "bun"
            ? `await hostWorkload(${workload}, ${bound});`
            : `export const ${WORKLOAD_EXPORT} = hostWorkload(${workload}, ${bound});`,
    ].join("\n");
}

/** Write an import of a located declaration under a local name. */
function importModule(module: DeclarationModule, local: string): string {
    return `import { ${module.export} as ${local} } from ${JSON.stringify(module.file)};`;
}

/** Select the package's own declarations. */
function owned(compilation: Compilation): DeclarationDescription[] {
    return compilation.declarations.filter(
        (declaration) => declaration.symbol.package.id === compilation.package.id,
    );
}
