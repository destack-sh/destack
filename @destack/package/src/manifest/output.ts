import { schema } from "@destack/schema";
import { DependencyName, DependencyRelease } from "../definition/index.ts";
import { PackagePath } from "../file/index.ts";
import { Runtime } from "../runtime/index.ts";
import { WorkloadDescription } from "./workload.ts";
import { ViewDescription } from "./view.ts";
import { DeclarationName } from "../definition/package.ts";

/** An output's export: its emitted file and the dependency specifiers its chunks import. */
export const OutputExport = schema.object({
    /** The emitted file of the export. */
    path: PackagePath,
    /** The dependency specifiers the export's chunks import, such as `@destack/object/client`. */
    imports: schema.array(schema.string().min(1)),
});
/** An output's export: its emitted file and the dependency specifiers its chunks import. */
export type OutputExport = schema.Infer<typeof OutputExport>;

/** A peer an output's package declares, which its dependents provide. */
export const OutputPeer = schema.object({
    /** Whether a dependent may leave it out, as npm's peerDependenciesMeta marks it. */
    isOptional: schema.boolean(),
});

/** A peer an output's package declares, which its dependents provide. */
export type OutputPeer = schema.Infer<typeof OutputPeer>;

/** Compiled files and dependencies for one runtime. */
export const PackageOutput = schema.object({
    /** The runtime the output is compiled for. */
    runtime: Runtime,
    /** Whether this output is distributed for execution. */
    emit: schema.boolean(),
    /** Validated workloads keyed by their declared names. */
    workloads: schema.record(DeclarationName, WorkloadDescription),
    /** The views this output mounts, keyed by their declared names. */
    views: schema.record(DeclarationName, ViewDescription),
    /** The output directory within the build. */
    directory: PackagePath,
    /** Generated entrypoints keyed by their exported names. */
    exports: schema.record(schema.string().min(1), OutputExport),
    /** Exact dependencies referenced by runtime code. */
    dependencies: schema.record(DependencyName, DependencyRelease),
    /** The peers its package declares, which each package depending on it provides beside it. */
    peers: schema.record(DependencyName, OutputPeer).default({}),
});

/** Compiled files for one execution target. */
export type PackageOutput = schema.Infer<typeof PackageOutput>;

/** An import map, as browsers read it: module specifiers to URLs, with scopes by URL prefix. */
export const ImportMap = schema.object({
    /** The URL of each specifier the page's own modules import. */
    imports: schema.record(schema.string().min(1), schema.string()),
    /** The URL of each specifier the modules below a URL prefix import, by prefix. */
    scopes: schema.record(
        schema.string().min(1),
        schema.record(schema.string().min(1), schema.string()),
    ),
});
/** An import map, as browsers read it: module specifiers to URLs, with scopes by URL prefix. */
export type ImportMap = schema.Infer<typeof ImportMap>;
