import { PackageError } from "../error/index.ts";
import { defineSchema, schema } from "@destack/schema";
import { Language, Package } from "../package/index.ts";
import { PackageFile } from "../file/file.ts";
import { PackageOutput } from "./output.ts";
import { SourceMapReference } from "../source/map.ts";

/** The portable structure of a package build manifest. */
export const PackageManifest = defineSchema(schema.object({
    /** The manifest format version, independent of the package version. */
    formatVersion: schema.literal(1),
    /** The package being built. */
    package: Package,
    /** The package's source language. */
    language: Language,
    /** Whether dynamic package inspection was enabled for this build. */
    dynamic: schema.boolean(),
    /** Named outputs compiled from the package. */
    outputs: schema.record(schema.string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/), PackageOutput),
    /** Every distributed file except this manifest. */
    files: schema.array(PackageFile),
    /** Source maps associated with exact generated files. */
    sourceMaps: schema.array(SourceMapReference),
}));

/** A package build description. */
export type PackageManifest = schema.Infer<typeof PackageManifest>;

/** Parse a manifest and check its file references. */
export function parseManifest(value: unknown): PackageManifest {
    const manifest = PackageManifest.parse(value);

    // index distributed files and reject conflicting paths
    const files = new Map<string, PackageFile>();
    for (const file of manifest.files) {
        if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
            throw new PackageError("INVALID_FILE", `Reserved build path: ${file.path}`);
        }
        if (files.has(file.path)) {
            throw new PackageError("INVALID_FILE", `Duplicate file: ${file.path}`);
        }
        files.set(file.path, file);
    }

    // reject files that also serve as directories, regardless of declaration order
    for (const path of files.keys()) {
        let separator = path.indexOf("/");
        while (separator !== -1) {
            const parent = path.slice(0, separator);
            if (files.has(parent)) {
                throw new PackageError("INVALID_FILE", `File is also a directory: ${parent}`);
            }
            separator = path.indexOf("/", separator + 1);
        }
    }

    // keep independently deployed outputs in separate directories
    const directories = new Set<string>();
    for (const output of Object.values(manifest.outputs)) {
        if (directories.has(output.directory)) {
            throw new PackageError(
                "INVALID_FILE",
                `Duplicate output directory: ${output.directory}`,
            );
        }
        directories.add(output.directory);
    }
    for (const directory of directories) {
        let path = directory;
        while (path.length) {
            if (files.has(path)) {
                throw new PackageError("INVALID_FILE", `Output directory is a file: ${path}`);
            }
            if (path !== directory && directories.has(path)) {
                throw new PackageError(
                    "INVALID_FILE",
                    `Overlapping output directories: ${path}, ${directory}`,
                );
            }
            const separator = path.lastIndexOf("/");
            if (separator === -1) break;
            path = path.slice(0, separator);
        }
    }

    // resolve each public entrypoint to a distributed file
    for (const output of Object.values(manifest.outputs)) {
        for (const [name, path] of Object.entries(output.exports)) {
            if (!files.has(path)) {
                throw new PackageError(
                    "INVALID_FILE",
                    `Missing export file: ${name} -> ${path}`,
                );
            }
            if (!path.startsWith(`${output.directory}/`)) {
                throw new PackageError("INVALID_FILE", `Export is outside its output: ${path}`);
            }
        }
    }

    // require one map per generated file and retain both files in the build
    const mapped = new Set<string>();
    for (const source of manifest.sourceMaps) {
        if (mapped.has(source.generated)) {
            throw new PackageError("INVALID_FILE", `Duplicate source map: ${source.generated}`);
        }
        mapped.add(source.generated);
        if (source.generated === source.map) {
            throw new PackageError(
                "INVALID_FILE",
                `Source map refers to itself: ${source.map}`,
            );
        }
        for (const path of [source.generated, source.map]) {
            if (!files.has(path)) {
                throw new PackageError("INVALID_FILE", `Missing source map file: ${path}`);
            }
        }
    }

    // resolve inspection paths against the distributed files
    for (const output of Object.values(manifest.outputs)) {
        const names = new Set<string>();
        for (const inspection of output.inspections) {
            if (names.has(inspection.name)) {
                throw new PackageError("INVALID_FILE", `Duplicate inspection: ${inspection.name}`);
            }
            names.add(inspection.name);
            for (const path of [inspection.document, inspection.schema]) {
                if (!files.has(path)) {
                    throw new PackageError("INVALID_FILE", `Missing inspection file: ${path}`);
                }
            }
            if (inspection.document === inspection.schema) {
                throw new PackageError(
                    "INVALID_FILE",
                    `Inspection document and schema share a path: ${inspection.document}`,
                );
            }
        }
    }

    return manifest;
}
