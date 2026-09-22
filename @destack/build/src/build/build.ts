import { type DependencyResolution } from "@destack/package/package";
import { type CompileOptions } from "../compile/module.ts";
import { type ApplicationOptions } from "../compile/application.ts";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { PackageManifest } from "@destack/package/manifest";
import { verifyFile } from "@destack/package/file";
import { PackageError } from "@destack/package/error";

/** A package manifest and its distributed file bytes. */
export class PackageBuild {
    /** The package build manifest. */
    readonly manifest: PackageManifest;
    /** Files indexed by their manifest paths. */
    readonly files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;

    /** Associate compiler output or imported files with their manifest. */
    constructor(manifest: PackageManifest, files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>) {
        this.manifest = manifest;
        this.files = files;
    }

    /** Read a distributed build and verify its files. */
    static async read(directory: string): Promise<PackageBuild> {
        const manifest = PackageManifest.parse(
            JSON.parse(await readFile(resolve(directory, "manifest.json"), "utf8")),
        );

        // reject ambiguous paths before opening distributed files
        const paths = new Set<string>();
        for (const file of manifest.files) {
            if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
                throw new PackageError("INVALID_FILE", `Reserved build path: ${file.path}`);
            }
            if (paths.has(file.path)) {
                throw new PackageError("INVALID_FILE", `Duplicate file: ${file.path}`);
            }
            paths.add(file.path);
        }
        for (const path of paths) {
            for (
                let separator = path.indexOf("/");
                separator !== -1;
                separator = path.indexOf("/", separator + 1)
            ) {
                const parent = path.slice(0, separator);
                if (paths.has(parent)) {
                    throw new PackageError("INVALID_FILE", `File is also a directory: ${parent}`);
                }
            }
        }

        // keep deployed outputs separate and resolve their public files
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
            const segments = directory.split("/");
            for (let count = 1; count <= segments.length; count++) {
                const path = segments.slice(0, count).join("/");
                if (paths.has(path)) {
                    throw new PackageError("INVALID_FILE", `Output directory is a file: ${path}`);
                }
                if (path !== directory && directories.has(path)) {
                    throw new PackageError(
                        "INVALID_FILE",
                        `Overlapping output directories: ${path}, ${directory}`,
                    );
                }
            }
        }
        const references: string[] = [];
        for (const output of Object.values(manifest.outputs)) {
            for (const path of Object.values(output.exports)) {
                if (!path.startsWith(`${output.directory}/`)) {
                    throw new PackageError("INVALID_FILE", `Export is outside its output: ${path}`);
                }
                references.push(path);
            }
            for (const inspection of output.inspections) {
                references.push(inspection.document, inspection.schema);
            }
        }
        for (const source of manifest.sourceMaps) {
            references.push(source.generated, source.map);
        }
        for (const path of references) {
            if (!paths.has(path)) {
                throw new PackageError("INVALID_FILE", `Missing build file: ${path}`);
            }
        }

        // retain the exact bytes checked against the distributed manifest
        const files = new Map<string, Uint8Array<ArrayBuffer>>();
        for (const file of manifest.files) {
            const bytes = new Uint8Array(await readFile(resolve(directory, file.path)));
            await verifyFile(file, bytes);
            files.set(file.path, bytes);
        }

        return new PackageBuild(manifest, files);
    }

    /** Write a build to a new directory and publish its manifest last. */
    async write(directory: string): Promise<void> {
        // require a new directory so a failed write cannot damage an existing build
        await mkdir(directory);
        for (const [path, bytes] of this.files) {
            const destination = resolve(directory, path);
            await mkdir(dirname(destination), { recursive: true });
            await writeFile(destination, bytes, { flag: "wx" });
        }

        // make the build readable only after every file has been written
        await writeFile(resolve(directory, "manifest.json"), JSON.stringify(this.manifest), {
            flag: "wx",
        });
    }
}

/** Inputs to a package build. */
export interface BuildOptions {
    /** The source package directory, kept unchanged for the duration of this build. */
    directory: string;
    /** Named browser and server outputs. */
    outputs: Readonly<Record<string, CompileOptions | ApplicationOptions>>;
    /** Exact dependency releases selected by the package resolver. */
    dependencies: Readonly<Record<string, DependencyResolution>>;
    /** The TypeScript configuration; omit to use package defaults. */
    configuration?: string;
    /** Cancel compilation and terminate its subprocesses. */
    signal?: AbortSignal;
    /** Maximum compilation time in milliseconds. */
    timeout?: number;
}
