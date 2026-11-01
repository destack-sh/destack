import type { Environment } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import {
    type DependencyRelease,
    graph,
    PackageDefinition,
    type PackageId,
    TEST_EXPORT,
} from "@destack/package";
import { PackageFile } from "@destack/package/file";
import type { Runtime } from "@destack/package/runtime";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, schema } from "@destack/schema";
import { ForgeError } from "../error/index.ts";
import { valid } from "semver";

/** The runtimes in the order their export conditions load. */
const CONDITION_ORDER = ["browser", "workerd", "bun"] as const satisfies readonly Runtime[];

/** The strict UTF-8 decoder of the build's declarations. */
const DECODER = new TextDecoder("utf-8", { fatal: true });

/** The TypeScript files of an export's types condition: sources and declarations. */
const TYPESCRIPT = /\.(?:d\.)?[mc]?tsx?$/u;

/** The source metadata the generated installation declaration copies. */
const Source = schema
    .object({
        name: schema.string(),
        version: schema.string(),
        description: schema.string().exactOptional(),
        license: schema.string().exactOptional(),
        sideEffects: schema
            .union([schema.boolean(), schema.array(schema.string())])
            .exactOptional(),
        bin: schema.record(schema.string().min(1), schema.string()).exactOptional(),
        peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
        peerDependenciesMeta: schema
            .record(schema.string(), schema.object({ optional: schema.boolean().exactOptional() }))
            .exactOptional(),
        optionalDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
        exports: schema
            .union([
                schema.string(),
                schema.record(
                    schema.string(),
                    schema.union([
                        schema.string(),
                        schema.record(schema.string(), schema.string()),
                    ]),
                ),
            ])
            .exactOptional(),
    })
    .strip();

/** The source metadata the installation declaration copies. */
type Source = schema.Infer<typeof Source>;

/** The npm archive installers download for a release. */
export const Distribution = schema.object({
    /** The archive's SHA-256 digest and storage key. */
    digest: Digest,
    /** The archive's SHA-1 checksum for npm clients. */
    shasum: schema.string(),
    /** The archive's SHA-512 Subresource Integrity expression. */
    integrity: schema.string(),
    /** The compressed size in bytes. */
    size: schema.number().int(),
    /** The total uncompressed size in bytes. */
    unpackedSize: schema.number().int(),
    /** The number of archived regular files. */
    fileCount: schema.number().int(),
});
/** The npm archive installers download for a release. */
export type Distribution = schema.Infer<typeof Distribution>;

/** The npm declaration of a release, generated from its build. */
export const PackageMetadata = schema.object({
    /** The scoped package name. */
    name: schema.string(),
    /** The immutable release version. */
    version: schema.string(),
    /** The module type of the compiled outputs: ECMAScript modules. */
    type: schema.literal("module"),
    /** The conditional compiled entrypoints, in priority order. */
    exports: schema.record(schema.string(), schema.record(schema.string(), schema.string())),
    /** The compiled module of each command the package installs, by command name. */
    bin: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The exact required dependency versions and npm aliases. */
    dependencies: schema.record(schema.string(), schema.string()),
    /** The exact optional dependency versions and npm aliases. */
    optionalDependencies: schema.record(schema.string(), schema.string()),
    /** The shared dependency version ranges consumers supply. */
    peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The optional peer declarations. */
    peerDependenciesMeta: schema
        .record(
            schema.string(),
            schema.object({
                optional: schema.boolean().exactOptional(),
            }),
        )
        .exactOptional(),
    /** The authored package description. */
    description: schema.string().exactOptional(),
    /** The commit the release was built from, as npm records it, absent for some prereleases. */
    gitHead: schema.string().exactOptional(),
    /** The authored SPDX license expression. */
    license: schema.string().exactOptional(),
    /** Whether importing a module can have side effects, or the files whose import can. */
    sideEffects: schema.union([schema.boolean(), schema.array(schema.string())]).exactOptional(),
});
/** The npm declaration of a release, generated from its build. */
export type PackageMetadata = schema.Infer<typeof PackageMetadata>;

/** A pushed build packed for its release, before the release's transaction. */
export const Pack = schema.object({
    /** The digest of the build's manifest. */
    manifest: Digest,
    /** The commit the build compiled, absent for a prerelease of uncommitted changes. */
    commit: schema.string().exactOptional(),
    /** The version the build declares. */
    version: schema.string(),
    /** The stored npm archive. */
    distribution: Distribution,
    /** The npm declaration of the archive. */
    metadata: PackageMetadata,
    /** The exact releases the archive requires. */
    dependencies: schema
        .array(schema.object({ name: schema.string(), version: schema.string() }))
        .readonly(),
    /** The manifests of the npm units the build depends on, directly or through other units. */
    units: schema.array(Digest).readonly(),
    /** The build's declarations, with their terms. */
    declarations: schema.array(graph.Declaration).readonly(),
});
/** A pushed build packed for its release, before the release's transaction. */
export type Pack = schema.Infer<typeof Pack>;

/** An npm archive with a complete Destack build beneath a generated package declaration. */
export class PackageArchive {
    /** The package identity of the build manifest. */
    readonly packageId: PackageId;
    /** The digest of the build manifest in the archive. */
    readonly manifest: Digest;
    /** The generated npm declaration. */
    readonly metadata: PackageMetadata;
    /** The exact releases the package requires, by package name, peers excluded. */
    readonly dependencies: Pack["dependencies"];
    /** The archive's digests, sizes and file count. */
    readonly distribution: Distribution;
    /** The compressed archive. */
    readonly #bytes: Blob;

    /** Take a packed archive and its description. */
    private constructor(
        fields: Pick<
            PackageArchive,
            "packageId" | "manifest" | "metadata" | "dependencies" | "distribution"
        >,
        bytes: Blob,
    ) {
        // take the description and the bytes
        this.packageId = fields.packageId;
        this.manifest = fields.manifest;
        this.metadata = fields.metadata;
        this.dependencies = fields.dependencies;
        this.distribution = fields.distribution;
        this.#bytes = bytes;
    }

    /** The archive as a file of the build store, named by its digest. */
    get file(): PackageFile {
        const { digest, size } = this.distribution;

        return { path: "package.tgz", digest, size, mediaType: "application/gzip" };
    }

    /** Open the compressed archive. */
    open(): ReadableStream<Uint8Array<ArrayBuffer>> {
        return this.#bytes.stream();
    }

    /** Pack a build of a commit, if any, with its compiled npm exports and exact dependencies. */
    static async pack(
        source: BuildReader,
        commit: string | undefined,
        environment: Pick<Environment, "crypto" | "compression">,
    ): Promise<PackageArchive> {
        // list the build's files, refusing duplicate paths
        const { manifest } = source;
        const list = await source.distributed();
        const paths = PackageArchive.#paths(list);

        // generate the npm declaration, and place the build manifest beneath it
        const declarations = new Map(
            await Promise.all(
                ["package.json", "destack.json"].map(
                    async (path) => [path, await source.load(path)] as const,
                ),
            ),
        );
        const { metadata, dependencies } = PackageArchive.#describe(
            manifest,
            declarations,
            paths,
            commit,
        );
        const encoder = new TextEncoder();
        const description = PackageManifest.serialize(manifest);
        const generated = new Map<string, Uint8Array<ArrayBuffer>>([
            ["package/package.json", encoder.encode(JSON.stringify(metadata))],
            ["package/build/manifest.json", description],
        ]);
        const built = list.map((file) => [`package/build/${file.path}`, file] as const);
        const entries = [...generated, ...built].toSorted(([left], [right]) =>
            left < right ? -1 : 1,
        );

        // archive the entries, and count the unpacked bytes of the declarations and of the build
        const { bytes, digests } = await PackageArchive.#archive(source, entries, environment);
        const declared = [...generated.values()].reduce(
            (sum, contents) => sum + contents.byteLength,
            0,
        );
        const unpacked = list.reduce((sum, file) => sum + file.size, declared);

        return new PackageArchive(
            {
                packageId: PackageManifest.package(manifest).id,
                manifest: await Digest.of(description, environment.crypto),
                metadata,
                dependencies,
                distribution: {
                    ...digests,
                    size: bytes.size,
                    unpackedSize: unpacked,
                    fileCount: entries.length,
                },
            },
            bytes,
        );
    }

    /** Collect a build's paths, refusing the reserved manifest path and duplicates. */
    static #paths(list: readonly PackageFile[]): Set<string> {
        const paths = new Set<string>();
        for (const file of list) {
            // refuse the reserved manifest path
            if (file.path === "manifest.json") {
                throw ForgeError.of(
                    "RESERVED_MANIFEST",
                    "manifest.json is reserved for the build manifest",
                );
            }
            // refuse a duplicate path
            else if (paths.has(file.path)) {
                throw ForgeError.of("PATH_DUPLICATE", `duplicate build path: ${file.path}`);
            }
            paths.add(file.path);
        }

        return paths;
    }

    /** Archive entries in path order, hashing the compressed bytes as they stream. */
    static async #archive(
        source: BuildReader,
        entries: readonly (readonly [string, Uint8Array<ArrayBuffer> | PackageFile])[],
        environment: Pick<Environment, "crypto" | "compression">,
    ): Promise<{
        readonly bytes: Blob;
        readonly digests: Pick<Distribution, "digest" | "shasum" | "integrity">;
    }> {
        // read each entry, verifying a build file's bytes
        const read = async function* (): AsyncGenerator<TarballEntry> {
            for (const [path, entry] of entries) {
                const contents =
                    entry instanceof Uint8Array ? entry : await source.load(entry.path);
                yield TarballEntry.of(path, contents);
            }
        };

        // gather the compressed chunks as they stream
        const chunks: Uint8Array<ArrayBuffer>[] = [];
        for await (const chunk of Tarball.stream(read(), environment.compression)) {
            chunks.push(chunk);
        }

        // hash the archive's bytes for its digest, its shasum and its integrity, as npm calls them
        const bytes = new Blob(chunks);
        const contents = new Uint8Array(await bytes.arrayBuffer());
        const { crypto } = environment;
        const [sha256, sha1, sha512] = await Promise.all([
            crypto.digest("SHA-256", contents),
            crypto.digest("SHA-1", contents),
            crypto.digest("SHA-512", contents),
        ]);

        return {
            bytes,
            digests: {
                digest: Digest.parse(new Uint8Array(sha256).toHex()),
                shasum: new Uint8Array(sha1).toHex(),
                integrity: `sha512-${new Uint8Array(sha512).toBase64()}`,
            },
        };
    }

    /** Describe the npm exports and exact dependencies of outputs the export conditions load. */
    static #describe(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        paths: ReadonlySet<string>,
        commit: string | undefined,
    ): {
        readonly metadata: PackageMetadata;
        readonly dependencies: Pack["dependencies"];
    } {
        // read the declarations matching the manifest
        const source = PackageArchive.#source(manifest, files);
        const conditions = PackageArchive.#conditions(manifest);

        // declare the source exports' TypeScript and each loaded output's exports and dependencies
        const metadata = PackageArchive.#metadata(source, commit);
        PackageArchive.#declareTypes(metadata, source, paths);
        PackageArchive.#declareBins(metadata, source, manifest, paths);
        const dependencies = PackageArchive.#declareOutputs(
            metadata,
            source,
            manifest,
            conditions,
            paths,
        );

        return { metadata, dependencies };
    }

    /** Read the build's package.json. */
    static #source(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): Source {
        // parse the declaration
        const bytes = files.get("package.json");
        if (!bytes) {
            throw ForgeError.of("PACKAGE_JSON_MISSING", "build has no package.json");
        }
        const source = Source.parse(JSON.parse(DECODER.decode(bytes)));

        // require the manifest's package id in destack.json
        const declaration = files.get("destack.json");
        if (!declaration) {
            throw ForgeError.of("DECLARATION_MISSING", "build has no destack.json");
        } else if (
            PackageDefinition.read(DECODER.decode(declaration)).id !==
            PackageManifest.package(manifest).id
        ) {
            throw ForgeError.of(
                "PACKAGE_ID_MISMATCH",
                "package ID differs from its source declaration",
            );
        }

        // require the declared npm name and the manifest's release version
        if (source.name !== manifest.package.name || source.version !== manifest.package.version) {
            throw ForgeError.of("NPM_NAME_INVALID", "invalid npm package name or release version");
        }

        return source;
    }

    /** Map each npm export condition to the emitted output it loads. */
    static #conditions(manifest: PackageManifest): ReadonlyMap<string, Runtime> {
        // order the emitted outputs named after their runtimes
        const emitted = new Set(
            Object.entries(manifest.outputs).flatMap(([name, output]) =>
                output.emit && name === output.runtime ? [output.runtime] : [],
            ),
        );
        const ordered = CONDITION_ORDER.filter((runtime) => emitted.has(runtime));
        const fallback = ordered.at(-1);
        if (fallback === undefined) {
            throw ForgeError.of("RUNTIME_OUTPUT_MISSING", "build emits no runtime output");
        }

        // give each runtime its own condition but the default's
        const conditions = new Map<string, Runtime>(
            ordered.filter((runtime) => runtime !== fallback).map((runtime) => [runtime, runtime]),
        );

        return conditions.set("default", fallback);
    }

    /** Start the npm declaration from the source's descriptive fields and peers. */
    static #metadata(source: Source, commit: string | undefined): PackageMetadata {
        return {
            name: source.name,
            version: source.version,
            type: "module",
            exports: {},
            dependencies: {},
            optionalDependencies: {},
            ...(commit === undefined ? {} : { gitHead: commit }),
            ...(source.peerDependencies === undefined
                ? {}
                : { peerDependencies: source.peerDependencies }),
            ...(source.peerDependenciesMeta === undefined
                ? {}
                : { peerDependenciesMeta: source.peerDependenciesMeta }),
            ...(source.description === undefined ? {} : { description: source.description }),
            ...(source.license === undefined ? {} : { license: source.license }),
            ...(source.sideEffects === undefined ? {} : { sideEffects: source.sideEffects }),
        };
    }

    /** Add each source export's TypeScript to the declaration's exports first. */
    static #declareTypes(
        metadata: PackageMetadata,
        source: Source,
        paths: ReadonlySet<string>,
    ): void {
        for (const [entry, target] of Object.entries(PackageArchive.#exports(source))) {
            // pass over the test layer, which builds leave out, and an export without a TypeScript file
            const declared =
                typeof target === "string" ? target : (target["types"] ?? target["default"]);
            if (entry === TEST_EXPORT || declared === undefined || !TYPESCRIPT.test(declared)) {
                continue;
            }

            // require the file in the build, or a file a pattern's target matches
            const path = declared.replace(/^\.\//u, "");
            const pattern = SubpathPattern.parse(path);
            const isBuilt =
                pattern === undefined
                    ? paths.has(path)
                    : [...paths].some((built) => pattern.stem(built) !== undefined);
            if (!isBuilt) {
                throw ForgeError.of("EXPORT_SOURCE_MISSING", `missing export source: ${entry}`);
            }
            metadata.exports[entry] = { types: `./build/${path}` };
        }
    }

    /** Point each command the source declares at its module compiled into the Bun output. */
    static #declareBins(
        metadata: PackageMetadata,
        source: Source,
        manifest: PackageManifest,
        paths: ReadonlySet<string>,
    ): void {
        for (const command of Object.keys(source.bin ?? {})) {
            // require the command compiled into the emitted Bun output
            const output = manifest.outputs["bun"];
            const compiled = output?.emit === true ? output.bins?.[command] : undefined;
            if (compiled === undefined || !paths.has(compiled.path)) {
                throw ForgeError.of("BIN_MISSING", `command has no compiled module: ${command}`);
            }
            metadata.bin = { ...metadata.bin, [command]: `./build/${compiled.path}` };
        }
    }

    /** Read a source's exports as a map of entries, the main entry for a bare string. */
    static #exports(source: Source): Exclude<Source["exports"], string | undefined> {
        // read no exports as none
        if (source.exports === undefined) {
            return {};
        }
        // read a bare string as the main entry
        else if (typeof source.exports === "string") {
            return { ".": source.exports };
        }
        // read a map as it is
        else {
            return source.exports;
        }
    }

    /** Add each loaded output's compiled exports and exact dependencies to the declaration. */
    static #declareOutputs(
        metadata: PackageMetadata,
        source: Source,
        manifest: PackageManifest,
        conditions: ReadonlyMap<string, Runtime>,
        paths: ReadonlySet<string>,
    ): Pack["dependencies"] {
        // collect the required releases and each dependency's resolution across outputs
        const required: Pack["dependencies"][number][] = [];
        const resolutions = new Map<string, string>();
        const patterns = Object.keys(PackageArchive.#exports(source)).flatMap((name) => {
            const pattern = SubpathPattern.parse(name);

            return pattern === undefined ? [] : [pattern];
        });
        for (const [condition, outputName] of conditions) {
            // require the condition's output to be built and emitted
            const output = manifest.outputs[outputName];
            if (!output) {
                throw ForgeError.of("OUTPUT_UNKNOWN", `unknown package output: ${outputName}`);
            }
            // refuse a build-only output
            else if (!output.emit) {
                throw ForgeError.of("BUILD_ONLY_OUTPUT", `output is build-only: ${outputName}`);
            }

            // load each compiled export under the condition, a pattern's entries under the pattern
            for (const [entry, { path }] of Object.entries(output.exports)) {
                if (!paths.has(path) || (entry !== "." && !entry.startsWith("./"))) {
                    throw ForgeError.of("EXPORT_INVALID", `invalid compiled export: ${entry}`);
                }
                const [name, target] = PackageArchive.#subpath(entry, path, patterns);
                const declared = metadata.exports[name]?.[condition];
                if (declared !== undefined && declared !== `./build/${target}`) {
                    throw ForgeError.of(
                        "EXPORT_INVALID",
                        `pattern export compiles to more than one path: ${name}`,
                    );
                }
                metadata.exports[name] ??= {};
                metadata.exports[name][condition] = `./build/${target}`;
            }

            // fix each dependency at its exact release, the same across outputs
            for (const [dependency, release] of Object.entries(output.dependencies)) {
                PackageArchive.#requireResolution(resolutions, dependency, release);
                const isAdded = PackageArchive.#declareDependency(
                    metadata,
                    source,
                    dependency,
                    release.package,
                );
                if (isAdded) {
                    required.push({ name: release.package.name, version: release.package.version });
                }
            }
        }

        return required;
    }

    /** Record a dependency's resolution. */
    static #requireResolution(
        resolutions: Map<string, string>,
        dependency: string,
        release: DependencyRelease,
    ): void {
        // require an exact version
        const { name, version } = release.package;
        if (valid(version) !== version) {
            throw ForgeError.of("DEPENDENCY_INVALID", `invalid dependency version: ${dependency}`);
        }

        // refuse another resolution of the dependency
        const resolution = JSON.stringify(
            release.kind === "npm"
                ? [name, version, release.kind, release.registry, release.integrity]
                : [name, version, release.kind, release.manifest],
        );
        const previous = resolutions.get(dependency);
        if (previous !== undefined && previous !== resolution) {
            throw ForgeError.of("DEPENDENCY_CONFLICTING", `conflicting dependency: ${dependency}`);
        }
        resolutions.set(dependency, resolution);
    }

    /** Read the export a compiled entry falls under: its pattern's, else its own, with its path. */
    static #subpath(
        entry: string,
        path: string,
        patterns: readonly SubpathPattern[],
    ): readonly [string, string] {
        // place an entry a pattern expanded to under the pattern, its path's stem as the wildcard
        for (const pattern of patterns) {
            const stem = pattern.stem(entry);
            const at = stem === undefined ? -1 : path.lastIndexOf(stem);
            if (stem !== undefined && at >= 0) {
                return [pattern.text, `${path.slice(0, at)}*${path.slice(at + stem.length)}`];
            }
        }

        return [entry, path];
    }

    /** Declare a dependency at its exact version, and return whether it is new. */
    static #declareDependency(
        metadata: PackageMetadata,
        source: Source,
        dependency: string,
        resolved: Pack["dependencies"][number],
    ): boolean {
        // leave a peer to consumers
        if (source.peerDependencies?.[dependency] !== undefined) {
            return false;
        }

        // declare it among the optional or required dependencies
        const collection =
            source.optionalDependencies?.[dependency] !== undefined
                ? metadata.optionalDependencies
                : metadata.dependencies;
        const isFirst = collection[dependency] === undefined;
        collection[dependency] =
            resolved.name === dependency
                ? resolved.version
                : `npm:${resolved.name}@${resolved.version}`;

        return isFirst;
    }
}

/** An npm subpath pattern, such as `./phosphor/*`, split at its one wildcard. */
class SubpathPattern {
    /** The pattern as written. */
    readonly text: string;
    /** The text before the wildcard. */
    readonly prefix: string;
    /** The text after the wildcard. */
    readonly suffix: string;

    /** Split a pattern at its wildcard. */
    private constructor(text: string, prefix: string, suffix: string) {
        this.text = text;
        this.prefix = prefix;
        this.suffix = suffix;
    }

    /** Read a pattern with one wildcard, absent for a plain subpath. */
    static parse(text: string): SubpathPattern | undefined {
        const [prefix, suffix, ...rest] = text.split("*");
        if (prefix === undefined || suffix === undefined) {
            return undefined;
        } else if (rest.length > 0) {
            throw ForgeError.of("EXPORT_INVALID", `a pattern export takes one wildcard: ${text}`);
        }

        return new SubpathPattern(text, prefix, suffix);
    }

    /** Read the text the wildcard stands for in a name, absent for a name the pattern misses. */
    stem(name: string): string | undefined {
        const isMatch =
            name.length > this.prefix.length + this.suffix.length &&
            name.startsWith(this.prefix) &&
            name.endsWith(this.suffix);

        return isMatch
            ? name.slice(this.prefix.length, name.length - this.suffix.length)
            : undefined;
    }
}
