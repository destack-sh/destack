import { type FileSystem, FileSystemError } from "@destack/environment";
import { basename, dirname, join, sep } from "node:path";
import type { DeclarationConstructorMap } from "../definition/constructor.ts";
import { Definition, type PackageDefinition } from "../definition/definition.ts";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DependencyName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { schema, type Version } from "@destack/schema";

/** The name and version fields of a package.json. */
const Manifest = schema.looseObject({
    name: schema.string().exactOptional(),
    version: schema.string().exactOptional(),
});

/** The name and version fields of a Destack package's package.json. */
const ReleasedManifest = schema.looseObject({ name: schema.string(), version: schema.string() });

/** A package directory and the metadata its modules receive. */
export interface ModulePackage {
    /** The directory containing destack.json and package.json. */
    readonly directory: string;
    /** The metadata injected into each module of the package. */
    readonly metadata: ModuleMetadata;
}

/** The packages and stamped functions one build finds, read once per build. */
export class PackageLocator {
    /** The Destack package above each directory, pending or found. */
    readonly #owners = new Map<string, Promise<ModulePackage | undefined>>();
    /** The directory of each package name, by requesting directory, pending or found. */
    readonly #directories = new Map<string, Promise<string | undefined>>();
    /** The name each directory's package.json declares, pending or read. */
    readonly #names = new Map<string, Promise<string | undefined>>();
    /** The definition of each package directory, absent without a package definition. */
    readonly #definitions = new Map<string, Promise<PackageDefinition | undefined>>();
    /** The module parameter positions of stamped functions, by package name and directory. */
    readonly #imported = new Map<string, ReadonlyMap<string, number>>();
    /** The versions the build releases its source packages at, by package name. */
    readonly #versions: Readonly<Record<string, Version>>;
    /** The files the packages are read from. */
    readonly #files: FileSystem;

    /** Find packages in files, the sources at the versions the build releases them at. */
    constructor(files: FileSystem, versions: Readonly<Record<string, Version>> = {}) {
        this.#files = files;
        this.#versions = versions;
    }

    /** Find the Destack package containing a module, or nothing outside Destack packages. */
    find(path: string): Promise<ModulePackage | undefined> {
        return this.#owner(dirname(path));
    }

    /** Find a package's directory, an ancestor named like it or its node_modules installation. */
    directory(name: string, from: string): Promise<string | undefined> {
        // answer a directory asked before, else look it up once
        const key = `${from}\0${name}`;
        let found = this.#directories.get(key);
        if (found === undefined) {
            found = this.#lookUp(name, from);
            this.#directories.set(key, found);
        }

        return found;
    }

    /** Look a package's directory up from a directory towards the filesystem root. */
    async #lookUp(name: string, from: string): Promise<string | undefined> {
        // accept the package containing the directory
        const installed = join(from, "node_modules", name);
        if ((await this.#name(from)) === name) {
            return from;
        }

        // accept an installation of the package
        if (await this.#isInstalled(join(installed, "package.json"))) {
            return this.#files.realPath(installed);
        }
        // continue with the parent directory up to the filesystem root
        else if (dirname(from) !== from) {
            return this.directory(name, dirname(from));
        }
        // stop at the filesystem root
        else {
            return undefined;
        }
    }

    /** Report whether a manifest exists, an unreadable directory having none as in Node. */
    async #isInstalled(manifest: string): Promise<boolean> {
        try {
            return await this.#files.exists(manifest);
        } catch (error) {
            // read a directory the process cannot open as without the manifest
            if (FileSystemError.is(error, "PERMISSION_DENIED")) {
                return false;
            }
            throw error;
        }
    }

    /** Read the declaration constructors of a package's build, resolved from a directory. */
    async constructors(name: string, from: string): Promise<DeclarationConstructorMap> {
        return (await this.#definition(name, from))?.declarations ?? {};
    }

    /** Map a module specifier's functions to their module parameters, by export path. */
    async imported(
        specifier: string,
        path: string,
        metadata: ModuleMetadata,
    ): Promise<ReadonlyMap<string, number>> {
        // read relative imports from the module's own package, and bare imports from the named package
        const owner = specifier.startsWith(".")
            ? metadata.package.name
            : DependencyName.of(specifier);
        const key = `${owner}\0${dirname(path)}`;
        let stamps = this.#imported.get(key);
        if (stamps !== undefined) {
            return stamps;
        }

        // keep the stamped functions by parameter position
        const definition = await this.#definition(owner, dirname(path));
        stamps = new Map(
            Object.entries(definition?.stamps ?? {}).map(([name, stamp]) => [name, stamp.module]),
        );
        this.#imported.set(key, stamps);

        return stamps;
    }

    /** Read a package's destack.json once, resolved by name from a directory, if any. */
    async #definition(name: string, from: string): Promise<PackageDefinition | undefined> {
        // find the package itself or its installation above the directory
        const root = await this.directory(name, from);
        if (root === undefined) {
            return undefined;
        }

        // read its definition once
        let definition = this.#definitions.get(root);
        if (definition === undefined) {
            definition = this.#files
                .readTextFile(join(root, "destack.json"))
                .catch(FileSystemError.absent)
                .then((text) =>
                    text === undefined ? undefined : Definition.package(Definition.read(text)),
                );
            this.#definitions.set(root, definition);
        }

        return definition;
    }

    /** Share one lookup of the package above a directory across concurrent module loads. */
    #owner(directory: string): Promise<ModulePackage | undefined> {
        let lookup = this.#owners.get(directory);
        if (!lookup) {
            lookup = this.#read(directory);
            this.#owners.set(directory, lookup);
        }

        return lookup;
    }

    /** Read the nearest package definition at or above a directory. */
    async #read(directory: string): Promise<ModulePackage | undefined> {
        // stop at dependency installations and the filesystem root
        if (basename(directory) === "node_modules" || dirname(directory) === directory) {
            return undefined;
        }

        // continue with the parent directory when this one defines no package
        const text = await this.#files
            .readTextFile(join(directory, "destack.json"))
            .catch(FileSystemError.absent);
        const definition =
            text === undefined ? undefined : Definition.package(Definition.read(text));
        if (definition === undefined) {
            return this.#owner(dirname(directory));
        }

        // refuse a definition without the manifest of its package
        const manifestText = await this.#files
            .readTextFile(join(directory, "package.json"))
            .catch(FileSystemError.absent);
        if (manifestText === undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `${join(directory, "destack.json")} has no package.json beside it`,
            );
        }
        const manifest = ReleasedManifest.parse(JSON.parse(manifestText));
        const version = PackageLocator.release(directory, manifest, this.#versions);
        const metadata = ModuleMetadata.parse({
            package: { id: definition.id, name: manifest.name, version },
        });

        return { directory, metadata };
    }

    /** Read a package's release, the version a build releases its source at or the declared one. */
    static release(
        directory: string,
        declared: { readonly name: string; readonly version: string },
        versions: Readonly<Record<string, Version>>,
    ): string {
        // keep the declared version of an installation or a package the build leaves out
        const version = versions[declared.name];
        const isInstalled = directory.split(sep).includes("node_modules");
        if (isInstalled || version === undefined) {
            return declared.version;
        }

        return version;
    }

    /** Read the name in a directory's package.json once, or nothing without one. */
    #name(directory: string): Promise<string | undefined> {
        // read the manifest once
        let name = this.#names.get(directory);
        if (name === undefined) {
            name = this.#files
                .readTextFile(join(directory, "package.json"))
                .catch(FileSystemError.absent)
                .then((text) =>
                    text === undefined ? undefined : Manifest.parse(JSON.parse(text)).name,
                );
            this.#names.set(directory, name);
        }

        return name;
    }
}
