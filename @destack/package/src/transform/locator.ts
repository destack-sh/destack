import { existsSync, readFileSync, realpathSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import type { DeclarationConstructorMap } from "../definition/constructor.ts";
import { PackageDefinition } from "../definition/definition.ts";
import { ModuleMetadata } from "../definition/metadata.ts";

/** A package directory and the metadata its modules receive. */
export interface ModulePackage {
    /** The directory containing destack.json and package.json. */
    readonly directory: string;
    /** The metadata injected into each module of the package. */
    readonly metadata: ModuleMetadata;
}

/** The packages and declaration constructors one build finds, read once per build. */
export class PackageLocator {
    /** The Destack package above each directory, pending or found. */
    readonly #owners = new Map<string, Promise<ModulePackage | undefined>>();
    /** The directory of each package name, by requesting directory. */
    readonly #directories = new Map<string, string | undefined>();
    /** The name each directory's package.json declares. */
    readonly #names = new Map<string, string | undefined>();
    /** The declaration constructors of each package directory. */
    readonly #constructors = new Map<string, DeclarationConstructorMap>();
    /** The module parameter positions of each package's constructors, by package name and directory. */
    readonly #imported = new Map<string, Readonly<Record<string, number>>>();

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

    /** Read the declaration constructors a package declares, resolving it by name from a directory. */
    constructors(name: string, from: string): DeclarationConstructorMap {
        // find the package itself or its installation above the directory
        const root = this.directory(name, from);
        if (root === undefined) {
            return {};
        }

        // read the constructors its destack.json declares once
        let declared = this.#constructors.get(root);
        if (!declared) {
            const path = join(root, "destack.json");
            const definition = existsSync(path)
                ? PackageDefinition.read(readFileSync(path, "utf8"))
                : undefined;
            declared = definition?.declarations ?? {};
            this.#constructors.set(root, declared);
        }

        return declared;
    }

    /** Map the stamped constructors a module's import specifier provides to their module parameters. */
    imported(
        specifier: string,
        path: string,
        metadata: ModuleMetadata,
    ): Readonly<Record<string, number>> {
        // read relative imports from the module's own package, and bare imports from the named package
        const owner = specifier.startsWith(".")
            ? metadata.package.name
            : specifier.startsWith("@")
              ? specifier.split("/").slice(0, 2).join("/")
              : specifier.split("/")[0]!;
        const key = `${owner}\0${dirname(path)}`;
        let constructors = this.#imported.get(key);
        if (constructors !== undefined) {
            return constructors;
        }

        // keep the constructors taking a module, by parameter position
        constructors = Object.fromEntries(
            Object.entries(this.constructors(owner, dirname(path)))
                .filter(([, constructor]) => constructor.module !== undefined)
                .map(([name, constructor]) => [name, constructor.module!]),
        );
        this.#imported.set(key, constructors);

        return constructors;
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

        // read both manifests where the Destack definition exists
        try {
            const definition = PackageDefinition.read(
                await readFile(join(directory, "destack.json"), "utf8"),
            );
            const manifest = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
            const metadata = ModuleMetadata.parse({
                package: { id: definition.id, name: manifest.name, version: manifest.version },
            });

            return { directory, metadata };
        }
        // continue with the parent directory when this one defines no package
        catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
                throw error;
            }

            return this.#owner(dirname(directory));
        }
    }

    /** Read the name in a directory's package.json once, or nothing without one. */
    #name(directory: string): string | undefined {
        // read the manifest once
        if (!this.#names.has(directory)) {
            const manifest = join(directory, "package.json");
            this.#names.set(
                directory,
                existsSync(manifest) ? JSON.parse(readFileSync(manifest, "utf8")).name : undefined,
            );
        }

        return this.#names.get(directory);
    }
}
