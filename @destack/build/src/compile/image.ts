import { posix } from "node:path";
import MagicString, { type SourceMap } from "magic-string";
import { type ESTree, parseSync } from "rolldown/utils";
import { type Plugin, rolldown } from "rolldown";
import { Digest, present } from "@destack/schema";
import {
    BuildImage,
    BuildListCache,
    BuildReader,
    MODULE_TYPE,
    ModuleResolution,
    PackageManifest,
    type PackageOutput,
    parseImports,
} from "@destack/package/manifest";
import { children } from "@destack/package/vite";
import { DependencyName, SideEffects } from "@destack/package";
import { DESTACK_MODULE_SCHEME, isHostModule } from "@destack/package/build";
import { BuildError } from "../error/index.ts";
import { type Span, TelemetryScope } from "@destack/telemetry";
import { COMPILER_VERSION } from "../cache/fingerprint.ts";
import { BuildDirectory } from "../build/directory.ts";
import type { LocalBuildStore } from "../store/local.ts";

/** The suffix of a bundled CommonJS-named module's id: npm units ship ES modules under `.cjs`. */
const MODULE_SUFFIX = ".mjs";

/** The format of a release bundle's cache entry, bumped whenever release bundling changes. */
const RELEASE_FORMAT = 12;

/** The directory of a bundle's assets, below its browser output's. */
const ASSET_DIRECTORY = "asset";

/** The directory of a bundle's shared chunks, below its browser output's. */
const SHARED_DIRECTORY = "shared";

/** The URL prefixes a release image's modules are bundled under, as an origin serves them. */
const PREFIXES = { builds: "/.destack/build/", modules: "/.destack/module/" };

/** How a bundle reads modules, which it leaves to the page, and where it shares chunks. */
export interface ImageBundleOptions {
    /** The directory below the bundle its shared chunks go in. */
    readonly directory: string;
    /** Read a module's code by its module URL, absent for a URL naming no module of the image. */
    readonly load: (url: string) => Promise<string | undefined>;
    /** Read a module's source map by its module URL, absent for a module without one. */
    readonly map: (url: string) => Promise<string | undefined>;
    /** Read a file a module names by URL, by its module URL, absent for a file the image lacks. */
    readonly asset: (url: string) => Promise<Uint8Array<ArrayBuffer> | undefined>;
    /** Report whether a module runs code on import, as its build's manifest declares it. */
    readonly hasSideEffects: (url: string) => Promise<boolean>;
    /** Read the specifier an external module is imported by, absent to bundle the module. */
    readonly external: (url: string, specifier: string) => string | undefined;
    /** The packages the page links, whose imports the image leaves unresolved. */
    readonly linked: ReadonlySet<string>;
}

/** A chunk of a bundle: its path, its code and map, and the chunks and modules it imports. */
export interface BundleChunk {
    /** The chunk's path below the bundle. */
    readonly path: string;
    /** The chunk's code. */
    readonly code: string;
    /** The chunk's source map, chained to its modules' own maps, absent for an empty chunk. */
    readonly map: string | undefined;
    /** The chunks and external specifiers it imports statically. */
    readonly imports: readonly string[];
    /** The chunks and external specifiers it imports dynamically. */
    readonly dynamicImports: readonly string[];
}

/** A file a bundle emits beside its chunks, which a module names by URL. */
export interface BundleAsset {
    /** The asset's path below the bundle. */
    readonly path: string;
    /** The asset's bytes. */
    readonly source: Uint8Array<ArrayBuffer>;
}

/**
 * An image's modules bundled from entries: a chunk per entry at its build's path, the rest shared.
 *
 * The files modules name by URL go beside the chunks, their URLs rewritten to them.
 */
export class ImageBundle {
    /** The chunks, entries among them. */
    readonly chunks: readonly BundleChunk[];
    /** The files the chunks name by URL. */
    readonly assets: readonly BundleAsset[];
    /** The builds whose modules the chunks took in, their code rendered. */
    readonly builds: ReadonlySet<Digest>;

    /** Hold a bundle's chunks, assets and the builds they took in. */
    private constructor(
        chunks: readonly BundleChunk[],
        assets: readonly BundleAsset[],
        builds: ReadonlySet<Digest>,
    ) {
        this.chunks = chunks;
        this.assets = assets;
        this.builds = builds;
    }

    /** Bundle an image's modules from entries, each named by its module URL. */
    static async of(
        resolution: ModuleResolution,
        entries: readonly string[],
        options: ImageBundleOptions,
    ): Promise<ImageBundle> {
        // name each entry by its path within its build
        const pathOf = (url: string) => present(resolution.locate(url), `module '${url}'`).path;
        const named = new Map(entries.map((url) => [url, pathOf(url)]));

        // bundle the entries minified, the modules they name by URL emitted beside them
        const build = await rolldown({
            input: Object.fromEntries(
                [...named].map(([url, path]) => [path, ImageBundle.#id(url)]),
            ),
            platform: "browser",
            logLevel: "silent",
            plugins: [ImageBundle.#plugin(resolution, options)],
        });
        try {
            const generated = await build.generate({
                format: "esm",
                minify: true,
                sourcemap: true,
                entryFileNames: (chunk) =>
                    present(
                        named.get(
                            ImageBundle.#url(present(chunk.facadeModuleId, "the entry's module")),
                        ),
                        "the entry's path",
                    ),
                chunkFileNames: `${options.directory}/${SHARED_DIRECTORY}/[hash].js`,
                assetFileNames: `${options.directory}/${ASSET_DIRECTORY}/[name]-[hash][extname]`,
            });

            // take the chunks with the imports their code keeps, as rolldown also lists dropped external ones
            const outputs = generated.output.filter((file) => file.type === "chunk");
            const paths = new Set(outputs.map((chunk) => chunk.fileName));
            const chunks = await Promise.all(
                outputs.map(async (chunk) => {
                    const [parsed] = await parseImports(chunk.code);
                    const kept = new Set(parsed.map((imported) => imported.specifier));

                    return {
                        path: chunk.fileName,
                        code: chunk.code,
                        map: chunk.map?.toString(),
                        imports: chunk.imports.filter((each) => paths.has(each) || kept.has(each)),
                        dynamicImports: chunk.dynamicImports,
                    };
                }),
            );

            const assets = generated.output
                .filter((file) => file.type === "asset")
                .map((asset) => ({
                    path: asset.fileName,
                    source:
                        typeof asset.source === "string"
                            ? new TextEncoder().encode(asset.source)
                            : Uint8Array.from(asset.source),
                }));

            // note the builds whose modules render code into a chunk
            const builds = new Set(
                outputs.flatMap((chunk) =>
                    Object.entries(chunk.modules).flatMap(([id, module]) => {
                        const located =
                            module.renderedLength > 0 && !id.startsWith("\0")
                                ? resolution.locate(ImageBundle.#url(id))
                                : undefined;

                        return located === undefined ? [] : [located.build];
                    }),
                ),
            );

            return new ImageBundle(chunks, assets, builds);
        } finally {
            await build.close();
        }
    }

    /** Bundle a release image's browser output into its root build, answering the release image. */
    static async release(
        image: Digest,
        store: LocalBuildStore,
        linked: {
            /** The builtin packages the page links, whose imports the image leaves unresolved. */
            readonly builtins: ReadonlySet<string>;
            /** The builds several builtins link, which stay their own builds. */
            readonly shared: ReadonlySet<Digest>;
            /** The npm units the platform links, by name, with the subpaths each exports. */
            readonly platform: ReadonlyMap<string, ReadonlySet<string>>;
            /** The exports to bundle: every one, or the subpaths the build's importers import. */
            readonly exports: "every" | ReadonlySet<string>;
            /** The host's builtin images the bundle inlines, by package name. */
            readonly hosted: ReadonlyMap<string, BuildImage>;
        },
    ): Promise<Digest> {
        // bundle in a span naming the root's package, logged with its duration
        const value = await store.image(image);
        const root = await store.contents(value.root);
        const scope = TelemetryScope.of(import.meta.destack.package, store.environment.telemetry);
        const attributes = { "destack.package.name": root.manifest.package.name };

        // bundle it in its span
        const released = await scope.span(
            "destack.bundle.release",
            attributes,
            (span) => ImageBundle.#release(image, value, root, store, linked, { scope, span }),
            { level: "info" },
        );

        // give the bundled root the source root's graph, as both declare the same modules
        const bundled = (await store.image(released)).root;
        if (bundled !== value.root && (await store.isIndexed(value.root))) {
            await store.attachGraph(bundled, await store.graph(value.root));
        }

        return released;
    }

    /** Bundle a release image's browser output, recording each step in its own span. */
    static async #release(
        image: Digest,
        value: BuildImage,
        root: BuildReader,
        store: LocalBuildStore,
        linked: Parameters<typeof ImageBundle.release>[2],
        telemetry: { readonly scope: TelemetryScope; readonly span: Span },
    ): Promise<Digest> {
        // take the root's browser output, leaving an image without one as it is
        const { scope, span } = telemetry;
        const [output] = PackageManifest.outputs(root.manifest, "browser");
        if (output === undefined) {
            return image;
        }

        // take the bundled build the cache keeps for the image and what it links
        const key = await Digest.json(
            {
                kind: "release",
                format: RELEASE_FORMAT,
                compiler: COMPILER_VERSION,
                image,
                builtins: [...linked.builtins].toSorted(),
                shared: [...linked.shared].toSorted(),
                platform: [...linked.platform.keys()]
                    .toSorted()
                    .map((name) => [name, [...(linked.platform.get(name) ?? [])].toSorted()]),
                exports: linked.exports === "every" ? "every" : [...linked.exports].toSorted(),
                hosted: [...linked.hosted.values()].map((hosted) => hosted.root).toSorted(),
            },
            store.environment.crypto,
        );
        const cached = await store.cache.get(key);
        span.setAttribute("destack.bundle.cached", cached?.kind === "build");
        if (cached?.kind === "build") {
            return await store.putImage(
                await ImageBundle.#rooted(value, cached.manifest, linked, store),
            );
        }

        // resolve the image's browser modules, its builtins left to the page that links them
        const { crypto } = store.environment;
        const read = (digest: Digest) => store.get(digest);
        const resolution = await scope.span("destack.bundle.resolve", {}, async () => {
            const map = await BuildImage.imports(
                value,
                "browser",
                PREFIXES.builds,
                linked.hosted,
                read,
                "export",
            );

            return await ModuleResolution.of(
                map,
                value.root,
                PREFIXES,
                async (build) => PackageManifest.dependencies(await read(build), "browser"),
                crypto,
            );
        });
        // read the image's modules, their maps and the files they name
        const reading = ImageBundle.#reading(resolution, store);

        // bundle the kept exports and the views, each from its module
        const kept = Object.fromEntries(
            Object.entries(output.exports).filter(
                ([subpath]) => linked.exports === "every" || linked.exports.has(subpath),
            ),
        );
        const paths = [
            ...Object.values(kept).map((exported) => exported.path),
            ...Object.values(output.views).map((view) => view.entrypoint),
        ];
        const entries = [...new Set(paths)].map((path) => resolution.url(value.root, path));
        const bundle = await scope.span("destack.bundle.rolldown", {}, async (bundling) => {
            const bundled = await ImageBundle.#bundle(resolution, entries, output, linked, {
                ...reading,
                root: value.root,
            });
            bundling.setAttributes({
                "destack.bundle.modules": reading.count(),
                "destack.bundle.chunks": bundled.chunks.length,
            });

            return bundled;
        });

        // write the root's build again with the bundle for its kept exports' modules, keeping it under the key
        const requires = await ImageBundle.#ranges(value, bundle, root, store);
        const inlined = await ImageBundle.#inlined(bundle, store);
        const bundled = await scope.span("destack.bundle.write", {}, () =>
            ImageBundle.#write(
                root,
                { ...output, exports: kept, inlined },
                bundle,
                requires,
                store,
            ),
        );
        await store.cache.set(key, { kind: "build", manifest: bundled });

        return await store.putImage(await ImageBundle.#rooted(value, bundled, linked, store));
    }

    /** Tell whether a build's path holds a module of an output or its map, outside its assets. */
    static #isModule(path: string, output: PackageOutput): boolean {
        const below = `${output.directory}/`;

        return (
            path.startsWith(below) &&
            !path.startsWith(`${below}${ASSET_DIRECTORY}/`) &&
            (/\.[cm]?js$/u.test(path) || path.endsWith(".js.map"))
        );
    }

    /** Read an image's modules, their maps and the files they name, counting the modules. */
    static #reading(
        resolution: ModuleResolution,
        store: LocalBuildStore,
    ): Pick<ImageBundleOptions, "load" | "map" | "asset" | "hasSideEffects"> & {
        readonly count: () => number;
    } {
        // read each build of the image once
        const readers = new Map<Digest, Promise<BuildReader>>();
        const contents = (url: string) => {
            // locate the URL's build, reading it once
            const located = present(resolution.locate(url), `module '${url}'`);
            const reading = readers.get(located.build) ?? store.contents(located.build);
            readers.set(located.build, reading);

            return { reading, path: located.path };
        };
        let loaded = 0;

        return {
            load: async (url) => {
                // read a module's code, absent for a URL naming no listed module
                loaded += 1;
                const { reading, path } = contents(url);
                const build = await reading;
                const file = (await build.has(path)) ? await build.file(path) : undefined;

                return file?.mediaType === MODULE_TYPE
                    ? new TextDecoder().decode(await build.load(file.path))
                    : undefined;
            },
            map: async (url) => {
                // read a module's source map beside it, absent for a module without one
                const { reading, path } = contents(url);
                const build = await reading;
                const mapped = `${path}.map`;

                return (await build.has(mapped))
                    ? new TextDecoder().decode(await build.load(mapped))
                    : undefined;
            },
            asset: async (url) => {
                // read a file a module names by URL
                const { reading, path } = contents(url);
                const build = await reading;

                return (await build.has(path)) ? await build.load(path) : undefined;
            },
            hasSideEffects: async (url) => {
                // match the module's path below its output against the build's declared side effects
                const { reading, path } = contents(url);
                const { manifest } = await reading;
                const output = present(
                    Object.values(manifest.outputs).find((each) =>
                        path.startsWith(`${each.directory}/`),
                    ),
                    `the output of module '${url}'`,
                );

                return SideEffects.has(
                    manifest.sideEffects,
                    path.slice(output.directory.length + 1),
                );
            },
            count: () => loaded,
        };
    }

    /** Root an image at its bundled build, linking the builds its bundle leaves external. */
    static async #rooted(
        value: BuildImage,
        bundled: Digest,
        linked: Parameters<typeof ImageBundle.release>[2],
        store: LocalBuildStore,
    ): Promise<BuildImage> {
        // read the packages the bundled build's browser modules import
        const reader = await store.contents(bundled);
        const imported = new Set(
            [...(await reader.specifiers("browser"))].map((specifier) =>
                DependencyName.of(specifier),
            ),
        );

        // link each to the one build the builds taken in link it to, beside the root's own links for every runtime
        const taken = BuildImage.builds(value).filter((build) => !linked.shared.has(build));
        const carried: Record<string, Digest> = {};
        for (const name of imported) {
            const targets = new Set(taken.flatMap((build) => value.builds[build]?.[name] ?? []));
            if (targets.size > 1) {
                throw BuildError.of(
                    "DEPENDENCY_INVALID",
                    `dependency '${name}' of '${reader.manifest.package.name}' is invalid: its bundle takes in builds linking ${targets.size} builds of it`,
                );
            }
            const [target] = targets;
            if (target !== undefined) {
                carried[name] = target;
            }
        }
        const rooted = BuildImage.rooted(value, bundled);
        const own = value.builds[value.root] ?? {};

        return BuildImage.of(rooted.root, {
            ...rooted.builds,
            [bundled]: { ...own, ...carried },
        });
    }

    /** Bundle a release's entries, leaving shared builds and platform units to their builds. */
    static #bundle(
        resolution: ModuleResolution,
        entries: readonly string[],
        output: PackageOutput,
        linked: Parameters<typeof ImageBundle.release>[2],
        read: Pick<ImageBundleOptions, "load" | "map" | "asset" | "hasSideEffects"> & {
            readonly root: Digest;
        },
    ): Promise<ImageBundle> {
        const { load, map, asset, hasSideEffects, root } = read;

        return ImageBundle.of(resolution, entries, {
            directory: output.directory,
            load,
            map,
            asset,
            hasSideEffects,
            external: (url, specifier) => {
                // leave a module of another shared build, or a subpath a platform unit exports, to that build
                const { build } = present(resolution.locate(url), `module '${url}'`);
                const isShared = build !== root && linked.shared.has(build);
                const exported = linked.platform.get(DependencyName.of(specifier));
                const isPlatform =
                    build !== root && exported?.has(DependencyName.subpath(specifier)) === true;

                return isShared || isPlatform ? specifier : undefined;
            },
            linked: linked.builtins,
        });
    }

    /** Read the ranges the root requires with those of the packages its bundle leaves external. */
    static async #ranges(
        value: BuildImage,
        bundle: ImageBundle,
        root: BuildReader,
        store: LocalBuildStore,
    ): Promise<Readonly<Record<string, string>>> {
        // take each external package's range from a build of the image that requires it
        const names = new Set(
            bundle.chunks
                .flatMap((chunk) => bundle.externals(chunk.path))
                .map((specifier) => DependencyName.of(specifier)),
        );
        const ranges: Record<string, string> = {};
        for (const digest of BuildImage.builds(value)) {
            const { requires } = await store.get(digest);
            for (const name of names) {
                const range = requires[name];
                if (range !== undefined) {
                    ranges[name] ??= range;
                }
            }
        }

        return { ...ranges, ...root.manifest.requires };
    }

    /** Write a build's files again, its browser output's modules replaced by a bundle's chunks. */
    static async #write(
        root: BuildReader,
        output: PackageOutput,
        bundle: ImageBundle,
        requires: Readonly<Record<string, string>>,
        store: LocalBuildStore,
    ): Promise<Digest> {
        // keep every file the bundle leaves: all but the output's modules and maps
        const isReplaced = (path: string) => ImageBundle.#isModule(path, output);
        const { environment } = store;
        const directory = await environment.files.makeTemporaryDirectory("destack-bundle-");
        const written = new BuildDirectory(directory, environment);
        for (const file of await root.files()) {
            if (!isReplaced(file.path)) {
                written.add(file);
            }
        }

        // write the chunks, each export importing the builtins its chunks reach
        const encoder = new TextEncoder();
        for (const asset of bundle.assets) {
            await written.write(asset.path, asset.source);
        }
        for (const chunk of bundle.chunks) {
            await written.write(chunk.path, encoder.encode(chunk.code));
            if (chunk.map !== undefined) {
                await written.write(`${chunk.path}.map`, encoder.encode(chunk.map));
            }
        }
        const exports = Object.fromEntries(
            Object.entries(output.exports).map(([name, exported]) => [
                name,
                { path: exported.path, imports: bundle.externals(exported.path) },
            ]),
        );

        // finish the build with the root's lists, its browser output importing builtins alone
        const { manifest } = root;
        const outputs = Object.fromEntries(
            Object.entries(manifest.outputs).map(([name, each]) => [
                name,
                each.directory === output.directory
                    ? { ...each, exports, inlined: output.inlined }
                    : each,
            ]),
        );
        await written.finish({
            package: manifest.package,
            role: manifest.role,
            sideEffects: manifest.sideEffects,
            destack: manifest.destack,
            outputs,
            requires,
            sourceMaps: [
                ...(await root.sourceMaps()).filter((each) => !isReplaced(each.generated)),
                ...bundle.chunks.flatMap((chunk) =>
                    chunk.map === undefined
                        ? []
                        : [{ generated: chunk.path, map: `${chunk.path}.map` }],
                ),
            ],
            declarations: await root.declarations(),
            relationships: await root.relationships(),
            tables: await root.tables(),
        });
        // read the kept files from the root build, which the store holds already
        const reader = await BuildReader.openDirectory(
            directory,
            environment.files,
            environment.crypto,
            new BuildListCache(),
            (path, signal) => root.stream(path, signal),
        );

        return await store.put(reader);
    }

    /** List the external specifiers a chunk reaches through its static and dynamic imports. */
    externals(path: string): string[] {
        // follow each chunk's imports once, taking the specifiers no chunk is
        const chunks = new Map(this.chunks.map((chunk) => [chunk.path, chunk]));
        const reached = new Set<string>();
        const external = new Set<string>();
        const pending = [path];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const chunk = chunks.get(next);
            for (const imported of [...(chunk?.imports ?? []), ...(chunk?.dynamicImports ?? [])]) {
                if (!chunks.has(imported)) {
                    external.add(imported);
                } else if (!reached.has(imported)) {
                    reached.add(imported);
                    pending.push(imported);
                }
            }
        }

        return [...external].toSorted();
    }

    /** Resolve and load an image's modules, emitting the modules entries name by URL as entries. */
    static #plugin(resolution: ModuleResolution, options: ImageBundleOptions): Plugin {
        const emitted = new Map<string, string>();

        return {
            name: "@destack/build/image",
            async resolveId(specifier, importer) {
                // take an entry's module as it is
                if (importer === undefined) {
                    return { id: specifier };
                }

                // resolve through the image, importing an external module by its specifier, refusing an unresolved one neither the page, its runtime nor its host links
                const resolved = resolution.resolve(specifier, ImageBundle.#url(importer));
                const isLinked =
                    specifier.startsWith(DESTACK_MODULE_SCHEME) ||
                    isHostModule(specifier) ||
                    options.linked.has(DependencyName.of(specifier));
                if (resolved === undefined && !isLinked) {
                    throw BuildError.of(
                        "DEPENDENCY_INVALID",
                        `import '${specifier}' of '${importer}' is invalid: the image has no module for it`,
                    );
                }
                const external =
                    resolved === undefined ? specifier : options.external(resolved, specifier);

                // let rolldown drop an unused import of a module without side effects
                const moduleSideEffects =
                    resolved === undefined || (await options.hasSideEffects(resolved));

                return external === undefined
                    ? {
                          id: ImageBundle.#id(present(resolved, "the resolved module")),
                          moduleSideEffects,
                      }
                    : { id: external, external: true, moduleSideEffects };
            },
            async load(loaded) {
                // read a module's code and map, loading it as an ES module whatever its extension
                const id = ImageBundle.#url(loaded);
                const code = present(await options.load(id), `module '${id}' of the image`);
                const map = await options.map(id);

                return { code, moduleType: "js", ...(map === undefined ? {} : { map }) };
            },
            async transform(code, loaded) {
                // emit each file a module names by URL once
                const id = ImageBundle.#url(loaded);
                const emit = async (url: string) => {
                    // tell a module outside the output's asset directory from any other file
                    const located = resolution.locate(url);
                    const directory = `${options.directory}/${ASSET_DIRECTORY}/`;
                    const isAsset = located?.path.startsWith(directory) === true;
                    const isModule =
                        located !== undefined &&
                        !isAsset &&
                        (await options.load(url)) !== undefined;
                    const source = isModule ? undefined : await options.asset(url);

                    // emit a module or a hashed asset at its build's path, any other file by its name
                    const chunk = () =>
                        this.emitFile({
                            type: "chunk",
                            id: ImageBundle.#id(url),
                            fileName: present(located, `module '${url}'`).path,
                        });
                    const file = (bytes: Uint8Array<ArrayBuffer>) =>
                        this.emitFile(
                            isAsset
                                ? { type: "asset", fileName: located.path, source: bytes }
                                : { type: "asset", name: posix.basename(url), source: bytes },
                        );
                    const reference =
                        emitted.get(url) ??
                        (isModule ? chunk() : source === undefined ? undefined : file(source));

                    // emit it once
                    if (reference !== undefined) {
                        emitted.set(url, reference);
                    }

                    return reference;
                };

                return await ImageBundle.#relocated(code, id, emit);
            },
        };
    }

    /** Name a module URL as an id the bundler parses as an ES module. */
    static #id(url: string): string {
        return url.endsWith(".cjs") ? `${url}${MODULE_SUFFIX}` : url;
    }

    /** Read the npm packages a bundle took in, by name, at their versions. */
    static async #inlined(
        bundle: ImageBundle,
        store: LocalBuildStore,
    ): Promise<Readonly<Record<string, string>>> {
        // take the npm units among the builds the bundle took in
        const manifests = await Promise.all([...bundle.builds].map((build) => store.get(build)));
        const units = manifests
            .filter((manifest) => !PackageManifest.isPackage(manifest))
            .map(({ package: { name, version } }) => [name, version] as const)
            .toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)));

        return Object.fromEntries(units);
    }

    /** Read the module URL of a bundler id. */
    static #url(id: string): string {
        return id.endsWith(`.cjs${MODULE_SUFFIX}`) ? id.slice(0, -MODULE_SUFFIX.length) : id;
    }

    /** Rewrite each file a module names by a literal URL to the file the bundle emits for it. */
    static async #relocated(
        code: string,
        url: string,
        emit: (url: string) => Promise<string | undefined>,
    ): Promise<{ readonly code: string; readonly map: SourceMap } | null> {
        // find each `new URL("…", import.meta.url)`
        const parsed = parseSync(url, code, { lang: "js", sourceType: "module" });
        const named: { readonly start: number; readonly end: number; readonly target: string }[] =
            [];
        const pending: ESTree.Node[] = [parsed.program];
        for (let node = pending.pop(); node !== undefined; node = pending.pop()) {
            // read the literal a `new URL(…, import.meta.url)` names, a quoted or a plain template one
            const [first, second] = node.type === "NewExpression" ? node.arguments : [];
            const literal =
                first?.type === "Literal" && typeof first.value === "string"
                    ? first.value
                    : first?.type === "TemplateLiteral" && first.expressions.length === 0
                      ? first.quasis[0]?.value.cooked
                      : undefined;
            const isNamed =
                node.type === "NewExpression" &&
                node.callee.type === "Identifier" &&
                node.callee.name === "URL" &&
                typeof literal === "string" &&
                second?.type === "MemberExpression" &&
                second.object.type === "MetaProperty" &&
                second.property.type === "Identifier" &&
                second.property.name === "url";

            // note the file it names, else look inside it
            if (isNamed && typeof literal === "string") {
                const target = new URL(literal, `file://${url}`).pathname;
                named.push({ start: node.start, end: node.end, target });
            } else {
                pending.push(...children(node));
            }
        }

        // emit each named file and rewrite its URL to the emitted one, leaving a module naming none as it is
        if (named.length === 0) {
            return null;
        }
        const edited = new MagicString(code);
        for (const each of named) {
            const reference = await emit(each.target);
            if (reference !== undefined) {
                edited.overwrite(
                    each.start,
                    each.end,
                    `new URL(import.meta.ROLLUP_FILE_URL_${reference})`,
                );
            }
        }

        return { code: edited.toString(), map: edited.generateMap({ hires: true }) };
    }
}
