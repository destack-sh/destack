import { existsSync, readFileSync, realpathSync } from "node:fs";
import { readFile } from "node:fs/promises";
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
    /** The directory of each package name, by requesting directory. */
    readonly #directories = new Map<string, string | undefined>();
    /** The name each directory's package.json declares. */
    readonly #names = new Map<string, string | undefined>();
    /** The definition of each package directory, absent without a package definition. */
    readonly #definitions = new Map<string, PackageDefinition | undefined>();
    /** The module parameter positions of stamped functions, by package name and directory. */
    readonly #imported = new Map<string, ReadonlyMap<string, number>>();
    /** The versions the build releases its source packages at, by package name. */
    readonly #versions: Readonly<Record<string, Version>>;

    /** Find packages by directory, the sources at the versions the build releases them at. */
    constructor(versions: Readonly<Record<string, Version>> = {}) {
        this.#versions = versions;
    }

    /** Find the Destack package containing a module, or nothing outside Destack packages. */
    find(path: string): Promise<ModulePackage | undefined> {
        return this.#owner(dirname(path));
    }

    /** Find a package's directory, an ancestor named like it or its node_modules installation. */
    directory(name: string, from: string): string | undefined {
        // answer a directory asked before
        const key = `${from}\0${name}`;
        if (this.#directories.has(key)) {
            return this.#directories.get(key);
        }

        // accept the package containing the directory
        const installed = join(from, "node_modules", name);
        let root: string | undefined;
        if (this.#name(from) === name) {
            root = from;
        }
        // accept an installation of the package
        else if (existsSync(join(installed, "package.json"))) {
            root = realpathSync(installed);
        }
        // continue with the parent directory, stopping at the filesystem root
        else if (dirname(from) !== from) {
            root = this.directory(name, dirname(from));
        }

        // remember the answer for the directory
        this.#directories.set(key, root);

        return root;
    }

    /** Read the declaration constructors of a package's build, resolved from a directory. */
    constructors(name: string, from: string): DeclarationConstructorMap {
        return this.#definition(name, from)?.declarations ?? {};
    }

    /** Map a module specifier's functions to their module parameters, by export path. */
    imported(
        specifier: string,
        path: string,
        metadata: ModuleMetadata,
    ): ReadonlyMap<string, number> {
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
        const definition = this.#definition(owner, dirname(path));
        stamps = new Map(
            Object.entries(definition?.stamps ?? {}).map(([name, stamp]) => [name, stamp.module]),
        );
        this.#imported.set(key, stamps);

        return stamps;
    }

    /** Read a package's destack.json once, resolved by name from a directory, if any. */
    #definition(name: string, from: string): PackageDefinition | undefined {
        // find the package itself or its installation above the directory
        const root = this.directory(name, from);
        if (root === undefined) {
            return undefined;
        }

        // read its definition once
        if (!this.#definitions.has(root)) {
            const path = join(root, "destack.json");
            this.#definitions.set(
                root,
                existsSync(path)
                    ? Definition.package(Definition.read(readFileSync(path, "utf8")))
                    : undefined,
            );
        }

        return this.#definitions.get(root);
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
        const text = await readOptional(join(directory, "destack.json"));
        const definition =
            text === undefined ? undefined : Definition.package(Definition.read(text));
        if (definition === undefined) {
            return this.#owner(dirname(directory));
        }

        // refuse a definition without the manifest naming its package
        const manifestText = await readOptional(join(directory, "package.json"));
        if (manifestText === undefined) {
            throw new PackageError(
                "INVALID_DEFINITION",
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
    #name(directory: string): string | undefined {
        // read the manifest once
        if (!this.#names.has(directory)) {
            const manifest = join(directory, "package.json");
            this.#names.set(
                directory,
                existsSync(manifest)
                    ? Manifest.parse(JSON.parse(readFileSync(manifest, "utf8"))).name
                    : undefined,
            );
        }

        return this.#names.get(directory);
    }
}

/** Read a file's text, or nothing when it does not exist. */
async function readOptional(path: string): Promise<string | undefined> {
    try {
        return await readFile(path, "utf8");
    } catch (error) {
        // treat only a missing file as absent
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return undefined;
        }
        throw error;
    }
}
