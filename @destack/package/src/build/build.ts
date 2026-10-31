import type { Environment } from "@destack/environment";
import type { Plugin, PluginOption } from "vite";
import type { Digest } from "@destack/schema";
import type { Sandbox } from "../definition/sandbox.ts";
import type { Package } from "../definition/package.ts";
import type { DeclarationDescription } from "../definition/declaration.ts";
import type { OutputContent, PackageOutput } from "../manifest/index.ts";
import type { Runtime } from "../runtime/index.ts";

export type { Plugin, PluginOption };

/** A dependency's role in building the packages using it, set in its destack.json build field. */
export interface BuildExtension {
    /** Return the plugins transforming a dependent package's modules, wherever they compile. */
    readonly transform?: (context: TransformContext) => readonly PluginOption[];
    /** Return the plugins compiling one module output of a package that uses the dependency. */
    readonly compile?: (compilation: Compilation) => readonly PluginOption[];
    /** Describe the workloads and views of one compiled module output. */
    readonly describe?: (compilation: Compilation, compiled: CompiledOutput) => OutputDescription;
    /** Render a package's pages at build, whatever runtimes it ships. */
    readonly prerender?: Prerender;
}

/** How an extension renders a package's pages at build, as Astro prerenders on its server build. */
export interface Prerender {
    /** Report whether a package's own declarations have pages to render. */
    readonly isRendering: (declarations: readonly DeclarationDescription[]) => boolean;
    /** Render the pages once the module outputs compiled, returning their files by path. */
    readonly render: (
        context: PrerenderContext,
    ) => Promise<ReadonlyMap<string, Uint8Array<ArrayBuffer>>>;
}

/** A package build once its module outputs compiled, as extensions render pages from it. */
export interface PrerenderContext {
    /** The package the build compiles. */
    readonly package: Package;
    /** The package's source directory. */
    readonly directory: string;
    /** The build directory, its module outputs written and its dependency builds linked. */
    readonly build: string;
    /** The package's own declarations. */
    readonly declarations: readonly DeclarationDescription[];
    /** The compiled module outputs, by name. */
    readonly outputs: Readonly<Record<string, OutputContent>>;
    /** The clock, files and processes the pages render with. */
    readonly environment: Environment<"files" | "process">;
}

/** What modules compile for: package, runtime and server rendering. */
export interface TransformContext {
    /** The package directory. */
    readonly directory: string;
    /** The runtime the modules run on. */
    readonly runtime: Runtime;
    /** Whether a server renders the modules' pages before the browser hydrates them. */
    readonly server: boolean;
    /** The build's content-addressed cache of derived files, absent where builds keep none. */
    readonly cache?: TransformCache;
    /** The directory of each Destack dependency's build, by package name. */
    readonly dependencies: Readonly<Record<string, string>>;
    /** The clock and files the transforms run on. */
    readonly environment: Environment<"files">;
}

/** The build's content-addressed cache, keeping the files transforms derive under their keys. */
export interface TransformCache {
    /** Read the bytes kept under a key, a SHA-256 digest of what derived them, absent on a miss. */
    readonly read: (key: Digest) => Promise<Uint8Array<ArrayBuffer> | undefined>;
    /** Keep bytes under a key for later builds. */
    readonly write: (key: Digest, bytes: Uint8Array<ArrayBuffer>) => Promise<void>;
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
    /** What the package's workloads and views always use beyond the bare sandbox. */
    readonly sandbox: Sandbox;
    /** The declarations of the package and of the dependencies its modules import. */
    readonly declarations: readonly DeclarationDescription[];
    /** Locate the module and export with one of the package's own declarations. */
    readonly locate: (declaration: DeclarationDescription) => DeclarationModule;
    /** Emit a module as a chunk the package exports under an entrypoint, such as `./view/notes`. */
    readonly entry: (entrypoint: string, module: string) => void;
}

/** A compiled output, as extensions describe it. */
export interface CompiledOutput {
    /** The emitted chunk of each package entrypoint. */
    readonly exports: Readonly<Record<string, string>>;
    /** Select the declarations in an entrypoint's modules, refusing imports its runtime lacks. */
    readonly declarations: (entrypoint: string) => DeclarationDescription[];
    /** List the stylesheets an entrypoint's chunks import, statically or on demand. */
    readonly stylesheets: (entrypoint: string) => string[];
}

/** The workloads and views an extension describes in one output. */
export type OutputDescription = Partial<Pick<PackageOutput, "workloads" | "views">>;

/** A package's own declaration, located by the module exporting it. */
export interface DeclarationModule {
    /** The absolute path of the module exporting the declaration. */
    readonly file: string;
    /** The module's export with the declaration. */
    readonly export: string;
}
