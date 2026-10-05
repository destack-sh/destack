import { createHash } from "node:crypto";
import { Tarball, type TarballEntry } from "@destack/package/archive";
import {
    type DependencyRelease,
    PackageDefinition,
    Publication,
    type PackageId,
} from "@destack/package";
import { PackageFile } from "@destack/package/file";
import type { PackageDistribution, PackageManifest } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { valid } from "semver";
import type { Distribution, PackageMetadata } from "../object/index.ts";

/** The source metadata the generated installation declaration keeps. */
const Source = schema
    .object({
        name: schema.string(),
        version: schema.string(),
        description: schema.string().exactOptional(),
        license: schema.string().exactOptional(),
        sideEffects: schema.boolean().exactOptional(),
        bin: schema.json().exactOptional(),
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

/** The source metadata the installation declaration keeps. */
type Source = schema.Infer<typeof Source>;

/** The strict UTF-8 decoder of the build's declarations. */
const DECODER = new TextDecoder("utf-8", { fatal: true });

/** The TypeScript files an export's types condition may name: sources and declarations. */
const TYPESCRIPT = /\.(?:d\.)?[mc]?tsx?$/u;

/** An npm archive with a complete Destack build beneath a generated package declaration. */
export class PackageArchive {
    /** The package identity the build manifest names. */
    readonly packageId: PackageId;
    /** The digest of the build manifest in the archive. */
    readonly manifest: string;
    /** The generated npm declaration. */
    readonly metadata: PackageMetadata;
    /** The exact releases the package requires, by package name, peers excluded. */
    readonly dependencies: readonly { readonly name: string; readonly version: string }[];
    /** The archive's digests, sizes and file count. */
    readonly distribution: Distribution;
    /** The compressed archive. */
    readonly #bytes: Blob;

    /** Keep a packed archive and its description. */
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

    /** Pack a build from a commit with its compiled npm exports and exact dependencies, verifying every file. */
    static async pack(source: PackageDistribution, commit: string): Promise<PackageArchive> {
        // list the build's files, refusing duplicate paths
        const { manifest, reader } = source;
        const list = await reader.distributed();
        const paths = PackageArchive.#paths(list);

        // generate the npm declaration, and keep the build manifest beneath it
        const declarations = new Map(
            await Promise.all(
                ["package.json", "destack.json"].map(
                    async (path) => [path, await reader.load(path)] as const,
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
        const description = encoder.encode(JSON.stringify(manifest));
        const generated = new Map<string, Uint8Array<ArrayBuffer>>([
            ["package/package.json", encoder.encode(JSON.stringify(metadata))],
            ["package/build/manifest.json", description],
        ]);
        const built = list.map((file) => [`package/build/${file.path}`, file] as const);
        const entries = [...generated, ...built].toSorted(([left], [right]) =>
            left < right ? -1 : 1,
        );

        // archive the entries, and count the unpacked bytes of the declarations and of the build
        const { bytes, digests } = await PackageArchive.#archive(source, entries);
        const declared = [...generated.values()].reduce(
            (sum, contents) => sum + contents.byteLength,
            0,
        );
        const unpacked = list.reduce((sum, file) => sum + file.size, declared);

        return new PackageArchive(
            {
                packageId: manifest.package.id,
                manifest: createHash("sha256").update(description).digest("hex"),
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
                throw new ServiceError("BAD_REQUEST", {
                    message: "manifest.json is reserved for the build manifest",
                });
            }
            // refuse a duplicate path
            else if (paths.has(file.path)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `duplicate build path: ${file.path}`,
                });
            }
            paths.add(file.path);
        }

        return paths;
    }

    /** Archive entries in path order, hashing the compressed bytes as they stream. */
    static async #archive(
        source: PackageDistribution,
        entries: readonly (readonly [string, Uint8Array<ArrayBuffer> | PackageFile])[],
    ): Promise<{
        readonly bytes: Blob;
        readonly digests: Pick<Distribution, "digest" | "shasum" | "integrity">;
    }> {
        // read each entry, verifying a build file's bytes
        const read = async function* (): AsyncGenerator<TarballEntry> {
            for (const [path, entry] of entries) {
                const contents =
                    entry instanceof Uint8Array ? entry : await PackageArchive.#load(source, entry);
                yield { path, contents };
            }
        };

        // hash the compressed chunks as they stream
        const sha256 = createHash("sha256");
        const sha1 = createHash("sha1");
        const sha512 = createHash("sha512");
        const chunks: Uint8Array<ArrayBuffer>[] = [];
        for await (const chunk of Tarball.stream(read())) {
            sha256.update(chunk);
            sha1.update(chunk);
            sha512.update(chunk);
            chunks.push(chunk);
        }

        return {
            bytes: new Blob(chunks),
            digests: {
                digest: sha256.digest("hex"),
                shasum: sha1.digest("hex"),
                integrity: `sha512-${sha512.digest("base64")}`,
            },
        };
    }

    /** Open the compressed archive. */
    open(): ReadableStream<Uint8Array<ArrayBuffer>> {
        return this.#bytes.stream();
    }

    /** Read one build file and verify its exact bytes. */
    static async #load(
        source: PackageDistribution,
        file: PackageFile,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // read the whole file
        const contents = new Uint8Array(
            await new Response(await source.open(file.path)).arrayBuffer(),
        );

        // refuse bytes other than the list names
        await PackageFile.verify(file, contents);

        return contents;
    }

    /** Describe the npm exports and exact dependencies the build's publication loads. */
    static #describe(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        paths: ReadonlySet<string>,
        commit: string,
    ): {
        readonly metadata: PackageMetadata;
        readonly dependencies: { readonly name: string; readonly version: string }[];
    } {
        // read the declarations matching the manifest
        const source = PackageArchive.#source(manifest, files);
        const publication = PackageArchive.#publication(manifest, files);

        // declare the source exports' TypeScript and each loaded output's exports and dependencies
        const metadata = PackageArchive.#metadata(source, commit);
        PackageArchive.#declareTypes(metadata, source, paths);
        const dependencies = PackageArchive.#declareOutputs(
            metadata,
            source,
            manifest,
            publication,
            paths,
        );

        return { metadata, dependencies };
    }

    /** Read the build's package.json, requiring the manifest's name and version and no executables. */
    static #source(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): Source {
        // parse the declaration
        const bytes = files.get("package.json");
        if (!bytes) {
            throw new ServiceError("BAD_REQUEST", { message: "build has no package.json" });
        }
        const source = Source.parse(JSON.parse(DECODER.decode(bytes)));

        // require the declared npm name and the manifest's release version
        if (source.name !== manifest.package.name || source.version !== manifest.package.version) {
            throw new ServiceError("BAD_REQUEST", {
                message: "invalid npm package name or release version",
            });
        }
        // refuse executables
        else if (source.bin !== undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "executable exports require a compiled bin mapping",
            });
        }

        return source;
    }

    /** Read the publication of the build's destack.json, requiring the manifest's package id. */
    static #publication(
        manifest: PackageManifest,
        files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    ): Publication {
        // parse the definition and match its identity to the manifest
        const declaration = files.get("destack.json");
        if (!declaration) {
            throw new ServiceError("BAD_REQUEST", { message: "build has no destack.json" });
        }
        const definition = PackageDefinition.read(DECODER.decode(declaration));
        if (definition.id !== manifest.package.id) {
            throw new ServiceError("BAD_REQUEST", {
                message: "package ID does not match its source declaration",
            });
        }

        // require a valid publication
        if (definition.publication === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "destack.json declares no publication",
            });
        }
        Publication.require(definition.publication);

        return definition.publication;
    }

    /** Start the npm declaration from the source's descriptive fields and peers, with no exports or dependencies yet. */
    static #metadata(source: Source, commit: string): PackageMetadata {
        return {
            name: source.name,
            version: source.version,
            type: "module",
            exports: {},
            dependencies: {},
            optionalDependencies: {},
            gitHead: commit,
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

    /** Add each source export's TypeScript to the declaration's exports first, as the types condition installers and compilers read. */
    static #declareTypes(
        metadata: PackageMetadata,
        source: Source,
        paths: ReadonlySet<string>,
    ): void {
        for (const [entry, target] of Object.entries(PackageArchive.#exports(source))) {
            // pass over an export naming no TypeScript file
            const declared =
                typeof target === "string" ? target : (target["types"] ?? target["default"]);
            if (declared === undefined || !TYPESCRIPT.test(declared)) {
                continue;
            }

            // require the file in the build
            const path = declared.replace(/^\.\//u, "");
            if (!paths.has(path)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `missing export source: ${entry}`,
                });
            }
            metadata.exports[entry] = { types: `./build/${path}` };
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

    /** Add each loaded output's compiled exports and exact dependencies to the declaration, returning the releases it requires. */
    static #declareOutputs(
        metadata: PackageMetadata,
        source: Source,
        manifest: PackageManifest,
        publication: Publication,
        paths: ReadonlySet<string>,
    ): { readonly name: string; readonly version: string }[] {
        // collect the required releases and each dependency's resolution across outputs
        const required: { readonly name: string; readonly version: string }[] = [];
        const resolutions = new Map<string, string>();
        for (const [condition, outputName] of Object.entries(publication.conditions)) {
            // require the condition's output to be built and emitted
            const output = manifest.outputs[outputName];
            if (!output) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `unknown package output: ${outputName}`,
                });
            }
            // refuse a build-only output
            else if (!output.emit) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `output is build-only: ${outputName}`,
                });
            }

            // load each compiled export under the condition
            for (const [entry, path] of Object.entries(output.exports)) {
                if (!paths.has(path) || (entry !== "." && !entry.startsWith("./"))) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `invalid compiled export: ${entry}`,
                    });
                }
                metadata.exports[entry] ??= {};
                metadata.exports[entry][condition] = `./build/${path}`;
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

    /** Record a dependency's resolution, refusing an invalid version and another resolution of the same dependency. */
    static #requireResolution(
        resolutions: Map<string, string>,
        dependency: string,
        release: DependencyRelease,
    ): void {
        // require an exact version
        const { name, version } = release.package;
        if (valid(version) !== version) {
            throw new ServiceError("BAD_REQUEST", {
                message: `invalid dependency version: ${dependency}`,
            });
        }

        // refuse another resolution of the dependency
        const resolution = JSON.stringify(
            release.kind === "npm"
                ? [name, version, release.kind, release.registry, release.integrity]
                : [name, version, release.kind, release.manifest],
        );
        const previous = resolutions.get(dependency);
        if (previous !== undefined && previous !== resolution) {
            throw new ServiceError("BAD_REQUEST", {
                message: `conflicting dependency: ${dependency}`,
            });
        }
        resolutions.set(dependency, resolution);
    }

    /** Declare a dependency at its exact version and return whether it is new, aliased when installed under another name. */
    static #declareDependency(
        metadata: PackageMetadata,
        source: Source,
        dependency: string,
        resolved: { readonly name: string; readonly version: string },
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
