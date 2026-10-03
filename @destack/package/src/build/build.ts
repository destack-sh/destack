import type { Manifest, Plugin, PluginOption } from "vite";
import { schema } from "@destack/schema";
import type { ModuleDescription } from "../code/index.ts";
import type { Capabilities } from "../definition/capability.ts";
import type { Package } from "../definition/package.ts";
import type { DeclarationDescription } from "../inspect/index.ts";
import type { PackageOutput } from "../manifest/index.ts";
import type { Runtime } from "../runtime/index.ts";

export type { Plugin, PluginOption };

/** A dependency's part in building the packages that use it, set in its destack.json build field. */
export interface BuildExtension {
    /** Return the plugins compiling one module output of a package that uses the dependency. */
    compile?(compilation: Compilation): readonly PluginOption[];
    /** Describe the workloads and views of one compiled module output. */
    describe?(compilation: Compilation, compiled: CompiledOutput): OutputDescription;
    /** The output kinds this dependency compiles in a pass of its own, such as web applications. */
    readonly outputs?: Readonly<Record<string, OutputKind>>;
}

/** An output kind compiled in one pass into several outputs, such as a web application's browser and server. */
export interface OutputKind {
    /** Expand a request into the outputs it compiles, by output name. */
    expand(name: string, request: OutputRequest): Readonly<Record<string, ExpandedOutput>>;
    /** Compile the expanded outputs together and describe each one's manifest output, workloads and views included. */
    compile(
        name: string,
        request: OutputRequest,
        compilations: Readonly<Record<string, Compilation>>,
        pass: Pass,
    ): Promise<Readonly<Record<string, PackageOutput>>>;
}

/** A requested output of a kind an extension compiles, such as a web application, with the kind's settings. */
export const OutputRequest = schema.looseObject({
    /** The output kind. */
    kind: schema.string().min(1),
});
/** A requested output of a kind an extension compiles, such as a web application, with the kind's settings. */
export type OutputRequest = schema.Infer<typeof OutputRequest>;

/** An output a kind expands a request into, inspected like a module output. */
export interface ExpandedOutput {
    /** The runtime the output runs on. */
    readonly runtime: Runtime;
    /** Package-relative modules compiled as the package's exports. */
    readonly entries: Readonly<Record<string, string>>;
    /** Package-relative modules inspected beside the entries, such as framework entries. */
    readonly files: readonly string[];
}

/** What a build lends an output kind's pass: its standard plugins, files and checks. */
export interface Pass {
    /** Return the plugins retaining sources, directories and module metadata for every output. */
    plugins(): Plugin[];
    /** Return the plugin recording what one Vite environment compiles, into an output when it has one. */
    record(environment: string, output?: string): Plugin;
    /** Map a generated file's source map source to its package path. */
    mapSource(source: string, map: string, generated: string): string;
    /** Describe a Vite asset manifest by package paths instead of paths below a root. */
    describeAssets(manifest: Manifest, root: string): Manifest;
    /** Check the modules of an output against its runtime, all of them unless named. */
    check(output: string, paths?: readonly string[]): Promise<void>;
    /** Stage the package in a temporary directory that resolves its dependencies. */
    stage(): Promise<AsyncDisposable & { readonly directory: string }>;
    /** Keep a generated file in the build. */
    file(path: string, bytes: Uint8Array<ArrayBuffer>): void;
    /** Copy a generated file into the build. */
    copy(path: string, source: string): Promise<void>;
    /** Keep a generated file's source map. */
    sourceMap(generated: string, map: string): void;
}

/** One output a package build compiles, as extensions compile and describe it. */
export interface Compilation {
    /** The package the build compiles. */
    readonly package: Package;
    /** The package's source directory. */
    readonly directory: string;
    /** The absolute source modules the output compiles, by export path, such as `./server`. */
    readonly exports: Readonly<Record<string, string>>;
    /** The runtime the output runs on. */
    readonly runtime: Runtime;
    /** What the package's workloads and views may reach. */
    readonly capabilities: Capabilities;
    /** The declarations of the package and of the dependencies its modules import. */
    readonly declarations: readonly DeclarationDescription[];
    /** The package's inspected modules. */
    readonly modules: readonly ModuleDescription[];
    /** Locate the module and export holding one of the package's own declarations. */
    locate(declaration: DeclarationDescription): DeclarationModule;
    /** Emit a module as a chunk the package exports under an entrypoint, such as `./view/notes`. */
    entry(entrypoint: string, module: string): void;
}

/** A compiled output, as extensions describe it. */
export interface CompiledOutput {
    /** The emitted chunk of each package entrypoint. */
    readonly exports: Readonly<Record<string, string>>;
    /** Select the declarations an entrypoint's modules hold, refusing imports its runtime lacks. */
    reach(entrypoint: string): DeclarationDescription[];
}

/** The workloads and views an extension describes in one output. */
export type OutputDescription = Partial<Pick<PackageOutput, "workloads" | "views">>;

/** A package's own declaration, located by the module exporting it. */
export interface DeclarationModule {
    /** The absolute path of the module exporting the declaration. */
    readonly file: string;
    /** The module's export holding the declaration. */
    readonly export: string;
}
