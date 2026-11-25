import type { FileSystem } from "@destack/environment";
import type {
    InputOptions,
    OutputAsset,
    OutputChunk,
    OutputOptions,
    Plugin,
    RolldownOutput,
} from "rolldown";
import { dev, type DevEngine } from "rolldown/experimental";
import { BuildError, bundleFailureOf } from "../error/index.ts";
import { present } from "@destack/schema";
import type { BundleManifest, BundleRecorder, ChunkStylesheets } from "./recorder.ts";

/** Vite's plugins that report on a whole build, which assume a full bundle a rebuild lacks. */
const BUILD_REPORT = /^(?:vite:license|vite:ssr-manifest|native:manifest|native:reporter)/u;

/** A bundle's output with the modules of the graph it was built from. */
export interface BundleOutput {
    /** The chunks and every asset they reference. */
    readonly generated: readonly (OutputChunk | OutputAsset)[];
    /** The modules of the graph, as the build that made the output left it. */
    readonly modules: readonly string[];
    /** The stylesheets each chunk imports, by the chunk's file name. */
    readonly stylesheets: ChunkStylesheets;
}

/** An engine build's output or failure, with its graph's modules and its chunks' stylesheets. */
interface EngineOutput {
    /** The output or failure. */
    readonly result: RolldownOutput | Error;
    /** The modules of the graph. */
    readonly modules: readonly string[];
    /** What the chunks import, absent for a bundle without Vite. */
    readonly imports: BundleManifest | undefined;
}

/** A warm bundle that rebuilds the modules its watcher sees change. */
export class Bundle implements AsyncDisposable {
    /** The engine that rebuilds the bundle on file changes. */
    readonly #engine: DevEngine;
    /** The files the bundle's modules come from. */
    readonly #files: FileSystem;
    /** The bytes of each package file when the bundle last loaded it, by path. */
    readonly #read: ReadonlyMap<string, Uint8Array>;
    /** The recorder of a Vite bundle's stylesheets, absent for a bundle without Vite. */
    readonly #recorder: BundleRecorder | undefined;
    /** The newest file of each asset outputs still reference, by file name. */
    readonly #assets = new Map<string, OutputAsset>();
    /** The number of outputs the engine produced, a full build or a rebuild each. */
    #builds = 0;
    /** The newest output or failure. */
    #output: EngineOutput | undefined;
    /** The wait for the next output. */
    #next = Promise.withResolvers<void>();

    /** Wrap a started engine, the bytes it read and a Vite bundle's recorder. */
    private constructor(
        engine: DevEngine,
        files: FileSystem,
        read: ReadonlyMap<string, Uint8Array>,
        recorder: BundleRecorder | undefined,
    ) {
        // wrap the engine, its files, the bytes it read and the recorder
        this.#engine = engine;
        this.#files = files;
        this.#read = read;
        this.#recorder = recorder;
    }

    /** Start a warm bundle reading each file it loads, with the recorder in its Vite options. */
    static async start(
        input: InputOptions,
        output: OutputOptions,
        directory: string,
        files: FileSystem,
        recorder: BundleRecorder | undefined,
    ): Promise<Bundle> {
        // read and watch each package file the bundle loads, leaving the load to the other plugins
        const read = new Map<string, Uint8Array>();
        const reading: Plugin = {
            name: "@destack/build/read",
            load: {
                filter: {
                    id: new RegExp(`^${RegExp.escape(directory)}/(?!.*/node_modules/)[^?]*$`, "u"),
                },
                async handler(id) {
                    this.addWatchFile(id);
                    read.set(id, await files.readFile(id));

                    return null;
                },
            },
        };

        // start the engine with its watcher on Vite's transforms, leaving out its build reports, handing each output to the bundle
        const received = { take: (_result: RolldownOutput | Error): void => undefined };
        const transforms = [input.plugins ?? []].flat().filter((plugin) => !isBuildReport(plugin));
        const plugins = [reading, ...transforms];
        const engine = await dev(
            { ...input, plugins, experimental: { ...input.experimental, incrementalBuild: true } },
            output,
            {
                onOutput: (result) => received.take(result),
                rebuildStrategy: "always",
                watch: { skipWrite: true },
            },
        );
        const bundle = new Bundle(engine, files, read, recorder);
        received.take = (result) => bundle.#take(result);
        await engine.run();

        return bundle;
    }

    /** The number of outputs the engine produced, a full build or a rebuild each. */
    get builds(): number {
        return this.#builds;
    }

    /** Take the newest output once it reflects the files on disk, unless a rebuild restyled it. */
    async latest(): Promise<BundleOutput> {
        // wait for the newest output
        await this.#settle();

        // fail with a broken bundle's failure, else the newest build's failure
        const { result, modules, imports } = present(this.#output, "the output");
        if (BuildError.is(result, "BUNDLE_BROKEN")) {
            throw result;
        } else if (result instanceof Error) {
            throw bundleFailureOf(result);
        }

        // complete the output with the assets its chunks import
        const stylesheets = imports?.stylesheets ?? new Map<string, readonly string[]>();

        return { generated: this.#complete(result.output, imports), modules, stylesheets };
    }

    /** Stop the engine and its watcher. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#engine.close();
    }

    /** Wait until the engine's newest output reflects the files as they read now. */
    async #settle(): Promise<void> {
        // wait for the build running now
        await this.#engineCall(() => this.#engine.ensureCurrentBuildFinish());
        while (this.#output === undefined || !(await this.#isCurrent(this.#output.modules))) {
            // wait for the next output, starting a full build when the watcher saw no change
            const next = this.#next.promise;
            const state = await this.#engineCall(() => this.#engine.getBundleState());
            if (this.#output !== undefined && !state.hasStaleOutput) {
                await this.#engineCall(() => this.#engine.triggerFullBuild());
            }
            await next;
        }
    }

    /** Call the engine, refusing the bundle once the engine itself fails apart from a build. */
    async #engineCall<Value>(call: () => Value | Promise<Value>): Promise<Value> {
        try {
            return await call();
        } catch (cause) {
            throw BuildError.of(
                "BUNDLE_BROKEN",
                "bundle is broken: its engine failed apart from a build",
                { cause },
            );
        }
    }

    /** Take an output with its modules and stylesheets, and wake the waits for it. */
    #take(result: RolldownOutput | Error): void {
        // record each asset an output emits, from the engine's first full build on
        if (!(result instanceof Error)) {
            for (const entry of result.output) {
                if (entry.type === "asset") {
                    this.#assets.set(entry.fileName, entry);
                }
            }
        }

        // count the output, take it with its modules and its build's stylesheets, a failed take breaking it
        this.#builds += 1;
        const modules = this.#engine.moduleGraph.getModuleIds();
        try {
            const imports = this.#recorder?.take();
            this.#output = { result, modules, imports };
        } catch (cause) {
            const broken = BuildError.of("BUNDLE_BROKEN", "bundle is broken: its recorder failed", {
                cause,
            });
            this.#output = { result: broken, modules, imports: undefined };
        }

        // wake the waits for it
        this.#next.resolve();
        this.#next = Promise.withResolvers<void>();
    }

    /** Add the assets an output's chunks import that its rebuild left out, and forget the rest. */
    #complete(
        generated: readonly (OutputChunk | OutputAsset)[],
        imports: BundleManifest | undefined,
    ): (OutputChunk | OutputAsset)[] {
        // take a bundle without Vite's records as it generated
        if (imports === undefined) {
            return [...generated];
        }

        // add each imported asset the rebuild left out
        const emitted = new Set(generated.map((entry) => entry.fileName));
        const restored = [...imports.assets]
            .filter((name) => !emitted.has(name))
            .map((name) => this.#restored(name));

        // forget the assets no chunk imports any more
        for (const name of this.#assets.keys()) {
            if (!imports.assets.has(name) && !emitted.has(name)) {
                this.#assets.delete(name);
            }
        }

        return [...generated, ...restored];
    }

    /** Read an asset an earlier build emitted, refusing the bundle when none did. */
    #restored(name: string): OutputAsset {
        const asset = this.#assets.get(name);
        if (asset === undefined) {
            throw BuildError.of(
                "BUNDLE_BROKEN",
                `cannot restore asset '${name}': the bundle never emitted it`,
            );
        }

        return asset;
    }

    /** Check that each file of an output's graph still reads as the bundle loaded it. */
    async #isCurrent(modules: readonly string[]): Promise<boolean> {
        const checks = modules.map(async (id) => {
            // compare a recorded file with the disk, a deleted one differing
            const bytes = this.#read.get(id);
            if (bytes === undefined) {
                return true;
            }
            const current = (await this.#files.exists(id))
                ? await this.#files.readFile(id)
                : undefined;

            return current !== undefined && Buffer.from(current).equals(bytes);
        });

        return (await Promise.all(checks)).every(Boolean);
    }
}

/** Report whether a plugin reports on a whole build, as Vite's manifest and license do. */
function isBuildReport(plugin: unknown): boolean {
    const name: unknown =
        typeof plugin === "object" && plugin !== null ? Reflect.get(plugin, "name") : undefined;

    return typeof name === "string" && BUILD_REPORT.test(name);
}
