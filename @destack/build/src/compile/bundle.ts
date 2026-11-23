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
import { bundleFailureOf } from "../error/index.ts";

/** A bundle's output with the modules of the graph it was built from. */
export interface BundleOutput {
    /** The chunks and assets. */
    readonly generated: readonly (OutputChunk | OutputAsset)[];
    /** The modules of the graph, as the build that made the output left it. */
    readonly modules: readonly string[];
}

/** A warm bundle that rebuilds the modules its watcher sees change. */
export class Bundle implements AsyncDisposable {
    /** The engine that rebuilds the bundle on file changes. */
    readonly #engine: DevEngine;
    /** The files the bundle's modules come from. */
    readonly #files: FileSystem;
    /** The bytes of each package file when the bundle last loaded it, by path. */
    readonly #read: ReadonlyMap<string, Uint8Array>;
    /** The newest output or failure, with the modules of the graph its build left. */
    #output:
        | { readonly result: RolldownOutput | Error; readonly modules: readonly string[] }
        | undefined;
    /** The wait for the next output. */
    #next = Promise.withResolvers<void>();

    /** Wrap a started engine and the bytes its recorder keeps. */
    private constructor(
        engine: DevEngine,
        files: FileSystem,
        read: ReadonlyMap<string, Uint8Array>,
    ) {
        this.#engine = engine;
        this.#files = files;
        this.#read = read;
    }

    /** Start a warm bundle, recording the bytes of each file under a directory it loads. */
    static async start(
        input: InputOptions,
        output: OutputOptions,
        directory: string,
        files: FileSystem,
    ): Promise<Bundle> {
        // record and watch each package file the bundle loads, leaving the load to the other plugins
        const read = new Map<string, Uint8Array>();
        const recorder: Plugin = {
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

        // start the engine with its watcher, handing each output to the bundle
        const received = { take: (_result: RolldownOutput | Error): void => undefined };
        const plugins = [recorder, ...[input.plugins ?? []].flat()];
        const engine = await dev(
            { ...input, plugins, experimental: { ...input.experimental, incrementalBuild: true } },
            output,
            {
                onOutput: (result) => received.take(result),
                rebuildStrategy: "always",
                watch: { skipWrite: true },
            },
        );
        const bundle = new Bundle(engine, files, read);
        received.take = (result) => bundle.#take(result);
        await engine.run();

        return bundle;
    }

    /** Take the newest output once it reflects the disk's bytes of every file it read. */
    async latest(): Promise<BundleOutput> {
        // wait for the build running now
        await this.#engine.ensureCurrentBuildFinish();
        while (this.#output === undefined || !(await this.#isCurrent(this.#output.modules))) {
            // wait for the next output, starting a full build when the watcher saw no change
            const next = this.#next.promise;
            const state = await this.#engine.getBundleState();
            if (this.#output !== undefined && !state.hasStaleOutput) {
                this.#engine.triggerFullBuild();
            }
            await next;
        }

        // fail with the newest build's failure
        const { result, modules } = this.#output;
        if (result instanceof Error) {
            throw bundleFailureOf(result);
        }

        return { generated: result.output, modules };
    }

    /** Stop the engine and its watcher. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#engine.close();
    }

    /** Keep an output with its graph's modules, and wake the waits for it. */
    #take(result: RolldownOutput | Error): void {
        this.#output = { result, modules: this.#engine.moduleGraph.getModuleIds() };
        this.#next.resolve();
        this.#next = Promise.withResolvers<void>();
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
