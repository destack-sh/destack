import { createHash } from "node:crypto";
import { Tarball, type TarballEntry } from "@destack/package/archive";
import { PackageDefinition, Publication, type PackageId } from "@destack/package";
import { PackageError } from "@destack/package/error";
import { PackageFile } from "@destack/package/file";
import type { PackageDistribution, PackageManifest } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { valid } from "semver";
import type { Distribution, PackageMetadata } from "../object/index.ts";

/** Source metadata retained in the generated installation declaration. */
const SOURCE = schema
    .object({
        name: schema.string(),
        version: schema.string(),
        description: schema.string().optional(),
        license: schema.string().optional(),
        sideEffects: schema.boolean().optional(),
        bin: schema.json().optional(),
        peerDependencies: schema.record(schema.string(), schema.string()).optional(),
        peerDependenciesMeta: schema
            .record(schema.string(), schema.object({ optional: schema.boolean().optional() }))
            .optional(),
        optionalDependencies: schema.record(schema.string(), schema.string()).optional(),
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
            .optional(),
    })
    .strip();

/** The TypeScript files an export's types condition may name: sources and declarations. */
const TYPESCRIPT = /\.(?:d\.)?[mc]?tsx?$/;

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
        // refuse duplicate paths
        const { manifest, reader } = source;
        const inventory = await reader.inventory();
        const paths = new Set<string>();
        for (const file of inventory) {
            if (file.path === "manifest.json") {
                throw new ServiceError("BAD_REQUEST", {
                    message: "manifest.json is reserved for the build manifest",
                });
            } else if (paths.has(file.path)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `duplicate build path: ${file.path}`,
                });
            }
            paths.add(file.path);
        }

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
        const built = new Map(inventory.map((file) => [`package/build/${file.path}`, file]));
        const entries = [...generated.keys(), ...built.keys()].sort();

        // archive the entries in path order, hashing the compressed bytes as they stream
        const read = async function* (): AsyncGenerator<TarballEntry> {
            for (const path of entries) {
                const file = built.get(path);
                const contents =
                    file === undefined
                        ? generated.get(path)!
                        : await PackageArchive.#load(source, file);
                yield { path, contents };
            }
        };
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
        const bytes = new Blob(chunks);

        // count the unpacked bytes of the generated declarations and of the build
        const declared = [...generated.values()].reduce((sum, bytes) => sum + bytes.byteLength, 0);
        const unpacked = inventory.reduce((sum, file) => sum + file.size, declared);

        return new PackageArchive(
            {
                packageId: manifest.package.id,
                manifest: createHash("sha256").update(description).digest("hex"),
                metadata,
                dependencies,
                distribution: {
                    digest: sha256.digest("hex"),
                    shasum: sha1.digest("hex"),
                    integrity: `sha512-${sha512.digest("base64")}`,
                    size: bytes.size,
                    unpackedSize: unpacked,
                    fileCount: entries.length,
                },
            },
            bytes,
        );
    }

    /** Open the compressed archive. */
    open(): ReadableStream<Uint8Array<ArrayBuffer>> {
        return this.#bytes.stream() as ReadableStream<Uint8Array<ArrayBuffer>>;
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

        // refuse bytes other than the inventory names
        try {
            await PackageFile.verify(file, contents);
        } catch (error) {
            if (!(error instanceof PackageError)) {
                throw error;
            }
            throw new ServiceError("BAD_REQUEST", {
                message: `file contents differ: ${file.path}`,
                cause: error,
            });
        }

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
        // accept canonical npm release versions while retaining Destack calendar versioning
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const bytes = files.get("package.json");
        if (!bytes) {
            throw new ServiceError("BAD_REQUEST", { message: "build has no package.json" });
        }
        const source = SOURCE.parse(JSON.parse(decoder.decode(bytes)));

        // match the published identity to its retained source declaration
        const declaration = files.get("destack.json");
        if (!declaration) {
            throw new ServiceError("BAD_REQUEST", { message: "build has no destack.json" });
        }
        const definition = PackageDefinition.read(decoder.decode(declaration));
        if (definition.id !== manifest.package.id) {
            throw new ServiceError("BAD_REQUEST", {
                message: "package ID does not match its source declaration",
            });
        }

        // require the declared npm name, the manifest's release version and no executables
        if (source.name !== manifest.package.name || source.version !== manifest.package.version) {
            throw new ServiceError("BAD_REQUEST", {
                message: "invalid npm package name or release version",
            });
        }
        if (source.bin !== undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "executable exports require a compiled bin mapping",
            });
        }

        // retain peer declarations while deriving installed versions from build resolutions
        const required: { readonly name: string; readonly version: string }[] = [];
        const metadata: PackageMetadata = {
            name: source.name,
            version: source.version,
            type: "module",
            exports: {},
            dependencies: {},
            optionalDependencies: {},
            peerDependencies: source.peerDependencies,
            peerDependenciesMeta: source.peerDependenciesMeta,
            description: source.description,
            license: source.license,
            sideEffects: source.sideEffects,
            gitHead: commit,
        };

        // select the outputs the publication's conditions load
        if (definition.publication === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "destack.json declares no publication",
            });
        }
        Publication.require(definition.publication);
        const loads = Object.entries(definition.publication.conditions);

        // declare each source export's TypeScript first, as the types condition installers and compilers read
        const sourceExports =
            source.exports === undefined
                ? {}
                : typeof source.exports === "string"
                  ? { ".": source.exports }
                  : source.exports;
        for (const [entry, target] of Object.entries(sourceExports)) {
            const declared = typeof target === "string" ? target : (target.types ?? target.default);
            if (declared === undefined || !TYPESCRIPT.test(declared)) {
                continue;
            }
            const path = declared.replace(/^\.\//, "");
            if (!paths.has(path)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `missing export source: ${entry}`,
                });
            }
            metadata.exports[entry] = { types: `./build/${path}` };
        }

        // map each selected output's compiled exports and exact dependencies
        const dependencies = new Map<string, string>();
        for (const [condition, outputName] of loads) {
            // require the condition's output to be built and emitted
            const output = manifest.outputs[outputName];
            if (!output) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `unknown package output: ${outputName}`,
                });
            }
            if (!output.emit) {
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

            // pin each dependency to its exact release, aliased when installed under another name, alike across outputs
            for (const [dependency, release] of Object.entries(output.dependencies)) {
                const { name, version } = release.package;
                if (valid(version) !== version) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `invalid dependency version: ${dependency}`,
                    });
                }
                const specification = name === dependency ? version : `npm:${name}@${version}`;
                const resolution = JSON.stringify(
                    release.kind === "npm"
                        ? [name, version, release.kind, release.registry, release.integrity]
                        : [name, version, release.kind, release.manifest],
                );
                const previous = dependencies.get(dependency);
                if (previous && previous !== resolution) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: `conflicting dependency: ${dependency}`,
                    });
                }
                dependencies.set(dependency, resolution);

                // declare it unless consumers supply it as a peer, optional where the source declares it so
                if (source.peerDependencies?.[dependency] !== undefined) {
                    continue;
                }
                const collection =
                    source.optionalDependencies?.[dependency] !== undefined
                        ? metadata.optionalDependencies
                        : metadata.dependencies;
                if (collection[dependency] === undefined) {
                    required.push({ name, version });
                }
                collection[dependency] = specification;
            }
        }

        return { metadata, dependencies: required };
    }
}
