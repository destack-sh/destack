import { join } from "node:path";
import type { FileSystem } from "@destack/environment";
import { present } from "@destack/schema";
import type { OutputBundle, OutputChunk } from "rolldown";
import type { Plugin } from "vite";
import { BuildError } from "../error/index.ts";

/** The stylesheet modules Vite's CSS plugin compiles, its own `CSS_LANGS_RE`. */
const STYLESHEET = /\.(?:css|less|sass|scss|styl|stylus|pcss|postcss|sss)(?:$|\?)/u;

/** The directories of a package without sources of its bundle: installs, outputs, tests. */
const OUTSIDE = new Set(["node_modules", "output", "target", "tests"]);

/** The test modules a package places beside its sources, which its bundle never reads. */
const TEST_MODULE = /\.test\.[cm]?[jt]sx?$/u;

/** The stylesheets each chunk of a build imports, by the chunk's file name. */
export type ChunkStylesheets = ReadonlyMap<string, readonly string[]>;

/** A build's manifest: the stylesheets of each chunk, and every asset the chunks reference. */
export interface BundleManifest {
    /** The stylesheets each chunk imports, by the chunk's file name. */
    readonly stylesheets: ChunkStylesheets;
    /** The file names of every stylesheet and other asset the chunks reference. */
    readonly assets: ReadonlySet<string>;
}

/** A recorder of what a warm bundle's builds reference: Vite's records and the plugins' assets. */
export class BundleRecorder {
    /** The package's directory. */
    readonly #directory: string;
    /** The files the bundle reads. */
    readonly #files: FileSystem;
    /** The builds started and not yet taken, oldest first, each filled once rendered. */
    readonly #builds: Render[];
    /** The references each module emitted in the build running now, by the module. */
    readonly #emitted: Map<string, string[]>;
    /** The file names of the assets each module's newest load or transform emitted, by module. */
    readonly #assets: Map<string, readonly string[]>;
    /** The file names any build resolved a file URL to, which Vite's records keep across builds. */
    readonly #fileUrls: Set<string>;
    /** The modules each parsed module imports, as its newest parse found them, by the module. */
    readonly #imports: Map<string, readonly string[]>;
    /** The modules the newest rendered build reaches. */
    #modules: ReadonlySet<string>;
    /** The package's source files as the newest scan before the first take found them. */
    #scanned: ReadonlyMap<string, string>;
    /** The first taken build and the source files at its render, absent before the first take. */
    #first: { build: Render; files: ReadonlyMap<string, string> } | undefined;
    /** Whether a build after the first changed a file only a new bundle renders right. */
    #isStale: boolean;

    /** Start before any build of a package's directory. */
    constructor(directory: string, files: FileSystem) {
        // start with no build
        this.#directory = directory;
        this.#files = files;
        this.#builds = [];
        this.#emitted = new Map();
        this.#assets = new Map();
        this.#fileUrls = new Set();
        this.#imports = new Map();
        this.#modules = new Set();
        this.#scanned = new Map();
        this.#first = undefined;
        this.#isStale = false;
    }

    /** Record an asset a module emits in the build running now, as the package's plugins do. */
    emit(module: string, reference: string): void {
        const references = this.#emitted.get(module) ?? [];
        references.push(reference);
        this.#emitted.set(module, references);
    }

    /** Create the plugin starting each build and naming the assets its modules emitted. */
    plugin(): Plugin {
        // start each build, and reset each module a load or transform reads anew
        const start = () => {
            this.#builds.push(new Render());
        };
        const reset = (id: string) => {
            this.#emitted.set(id, this.#emitted.get(id) ?? []);
        };
        const name = (chunks: readonly OutputChunk[], fileName: (reference: string) => string) =>
            this.#name(chunks, fileName);

        return {
            name: "@destack/build/record",
            buildStart: start,
            load: {
                order: "pre",
                handler(id) {
                    reset(id);
                },
            },
            transform: {
                order: "pre",
                handler(_code, id) {
                    reset(id);
                },
            },
            generateBundle(_options, bundle) {
                const chunks = Object.values(bundle).filter(
                    (entry): entry is OutputChunk => entry.type === "chunk",
                );
                name(chunks, (reference) => this.getFileName(reference));
            },
        };
    }

    /** Create the Vite plugin filling each build with what Vite records of its chunks. */
    vitePlugin(): Plugin {
        // mark and fill the started build from Vite's hooks
        const transform = () => this.#started().transform();
        const resolve = (fileName: string) => {
            this.#fileUrls.add(fileName);
            this.#started().resolve(fileName);
        };
        const parse = (id: string, imports: readonly string[]) => this.#imports.set(id, imports);
        const render = (bundle: OutputBundle) => this.#render(bundle);

        return {
            name: "@destack/build/record-vite",
            enforce: "post",
            // record each file URL before Vite resolves it, leaving the resolution to Vite
            resolveFileUrl: {
                order: "pre",
                handler({ fileName }) {
                    resolve(fileName);

                    return null;
                },
            },
            transform: {
                filter: { id: STYLESHEET },
                handler() {
                    transform();

                    return null;
                },
            },
            moduleParsed(info) {
                parse(info.id, [...info.importedIds, ...info.dynamicallyImportedIds]);
            },
            async generateBundle(_options, bundle) {
                await render(bundle);
            },
        };
    }

    /** Forget the builds of Vite's own build, which ran before the engine. */
    clear(): void {
        this.#builds.length = 0;
    }

    /** Take the oldest build's imports for its output, marking the bundle stale by it. */
    take(): BundleManifest {
        // take the oldest build, unrendered when it failed
        const taken = present(this.#builds.shift(), "a started build for the output");
        const first = this.#first;

        // compare a later build with the first, which transformed every stylesheet
        if (first === undefined && taken.modules !== undefined) {
            this.#first = { build: taken, files: this.#scanned };
        } else if (first !== undefined && taken.modules !== undefined) {
            const isRegraphed = !isSameSet(first.build.stylesheets(), taken.stylesheets());
            this.#isStale ||= taken.isTransformed || isRegraphed;
        } else if (first !== undefined) {
            this.#isStale ||= taken.isTransformed;
        }

        return { stylesheets: taken.imported, assets: taken.assets };
    }

    /** Report whether a build changed a stylesheet, or a file outside the graph changed since. */
    async isStale(): Promise<boolean> {
        // scan the source files again unless a build changed a stylesheet already
        if (this.#isStale) {
            return true;
        }
        const first = present(this.#first, "the first build");
        const scanned = await this.#scan();

        // compare the files outside both the first and the newest graph
        const graph = new Set([...present(first.build.modules, "its modules"), ...this.#modules]);
        const before = [...first.files].filter(([path]) => !graph.has(path));
        const after = [...scanned].filter(([path]) => !graph.has(path));

        return !isSameMap(new Map(before), new Map(after));
    }

    /** Name the assets each module read anew emitted, adding the chunks' modules' to the build. */
    #name(chunks: readonly OutputChunk[], fileName: (reference: string) => string): void {
        // name what each module read anew emitted, replacing its earlier assets
        for (const [module, references] of this.#emitted) {
            this.#assets.set(module, references.map(fileName));
        }
        this.#emitted.clear();

        // add the assets of the modules each chunk renders to the started build
        const modules = chunks.flatMap((chunk) => chunk.moduleIds);
        this.#started().include(modules.flatMap((module) => this.#assets.get(module) ?? []));
    }

    /** Return the build the newest start began. */
    #started(): Render {
        return present(this.#builds.at(-1), "a started build");
    }

    /** Fill the started build with its chunks' stylesheets and the modules it reaches. */
    async #render(bundle: OutputBundle): Promise<void> {
        // read each chunk's stylesheets, which Vite records for its own plugins alone
        const build = this.#started();
        const chunks = Object.values(bundle).filter(
            (entry): entry is OutputChunk => entry.type === "chunk",
        );
        const imported = chunks.map((chunk) => {
            const metadata = chunk.viteMetadata;
            if (metadata === undefined) {
                throw BuildError.of(
                    "INVARIANT_BROKEN",
                    `chunk '${chunk.fileName}' is invalid: the recorder runs outside Vite's plugins`,
                );
            }

            return [chunk.fileName, metadata] as const;
        });
        const stylesheets = imported.map(
            ([file, metadata]) => [file, [...metadata.importedCss]] as const,
        );

        // read each imported asset, leaving the file URLs of earlier builds Vite's records keep
        const resolved = build.fileUrls;
        const isEarlier = (file: string) => this.#fileUrls.has(file) && !resolved.has(file);
        const assets = imported.flatMap(([, metadata]) => [
            ...metadata.importedCss,
            ...[...metadata.importedAssets].filter((file) => !isEarlier(file)),
        ]);

        // reach the modules from the chunks', as the engine's graph lists the modules a rebuild dropped
        const reached = new Set(chunks.flatMap((chunk) => chunk.moduleIds));
        for (const module of reached) {
            for (const imports of this.#imports.get(module) ?? []) {
                reached.add(imports);
            }
        }
        this.#modules = reached;
        build.render(new Map(stylesheets), reached);
        build.include(assets);

        // scan the source files before the first take, whose build later scans compare with
        if (this.#first === undefined) {
            this.#scanned = await this.#scan();
        }
    }

    /** Read the package's source files, each at its size and modification time, by path. */
    async #scan(): Promise<Map<string, string>> {
        // walk the source directories, skipping installs, outputs, tests and hidden directories
        const files = new Map<string, string>();
        const pending = [this.#directory];
        for (let directory = pending.pop(); directory !== undefined; directory = pending.pop()) {
            const entries = await this.#files.readDirectory(directory);
            const isSource = (name: string) => !OUTSIDE.has(name) && !name.startsWith(".");
            for (const entry of entries) {
                if (entry.kind === "directory" && isSource(entry.name)) {
                    pending.push(join(directory, entry.name));
                }
            }

            // stamp each file of the directory but its tests
            const sources = entries.filter(
                (entry) => entry.kind === "file" && !TEST_MODULE.test(entry.name),
            );
            const stamped = await Promise.all(
                sources.map(async (entry) => {
                    const path = join(directory, entry.name);
                    const status = await this.#files.stat(path);

                    return [path, `${status.size}:${status.modifiedAt}`] as const;
                }),
            );
            for (const [path, stamp] of stamped) {
                files.set(path, stamp);
            }
        }

        return files;
    }
}

/** One render of the bundle, started with its build and filled once Vite renders it. */
class Render {
    /** Whether it transformed a stylesheet. */
    isTransformed: boolean;
    /** The stylesheets each chunk imports, empty until rendered. */
    imported: ChunkStylesheets;
    /** The file names of the assets the chunks reference. */
    readonly assets: Set<string>;
    /** The file names its chunks resolved file URLs to. */
    readonly fileUrls: Set<string>;
    /** The modules it reaches, absent until rendered. */
    modules: ReadonlySet<string> | undefined;

    /** Start unrendered. */
    constructor() {
        // start with nothing rendered
        this.isTransformed = false;
        this.imported = new Map();
        this.assets = new Set();
        this.fileUrls = new Set();
        this.modules = undefined;
    }

    /** Read the stylesheet modules it reaches. */
    stylesheets(): Set<string> {
        return new Set([...(this.modules ?? [])].filter((id) => STYLESHEET.test(id)));
    }

    /** Add assets the build's chunks reference. */
    include(assets: readonly string[]): void {
        for (const asset of assets) {
            this.assets.add(asset);
        }
    }

    /** Record a file URL a chunk of the build resolved. */
    resolve(fileName: string): void {
        this.fileUrls.add(fileName);
    }

    /** Mark that the build transformed a stylesheet module. */
    transform(): void {
        this.isTransformed = true;
    }

    /** Fill the build with what Vite rendered. */
    render(imported: ChunkStylesheets, modules: ReadonlySet<string>): void {
        this.imported = imported;
        this.modules = modules;
    }
}

/** Report whether two sets hold the same values. */
function isSameSet(left: ReadonlySet<string>, right: ReadonlySet<string>): boolean {
    return left.size === right.size && [...left].every((value) => right.has(value));
}

/** Report whether two maps hold the same entries. */
function isSameMap(left: ReadonlyMap<string, string>, right: ReadonlyMap<string, string>): boolean {
    return left.size === right.size && [...left].every(([key, value]) => right.get(key) === value);
}
