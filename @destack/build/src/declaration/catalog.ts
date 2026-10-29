import { join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import {
    type DeclarationKind,
    type DeclarationKindValue,
    DependencyPackage,
    Package,
    PackageDefinition,
    PackageExport,
} from "@destack/package";
import { schema, type Version } from "@destack/schema";
import { PackageLocator } from "@destack/package/locator";
import { BuildError } from "../error/index.ts";
import { modulePackage } from "../source/dependency.ts";
import type { FunctionExport, Inspector } from "./declaration.ts";

/** The exports a package.json lists. */
const ExportsManifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
});

/** A package containing modules, with the name and version its package.json declares. */
export type PackageLocation = Awaited<ReturnType<typeof modulePackage>>;

/** A kind a constructor describes each declaration as, with the functions inspecting it. */
export interface ConstructorKind {
    /** The description kind. */
    readonly kind: string;
    /** The package declaring the kind. */
    readonly package: Package;
    /** The functions inspecting the kind. */
    readonly inspector: Inspector;
}

/** A declaration constructor its package declares as inspected. */
export interface InspectedConstructor {
    /** The declaring package's identity. */
    readonly package: Package;
    /** The kinds it describes each declaration as. */
    readonly kinds: readonly ConstructorKind[];
}

/** The packages and constructors one inspection finds, each read once per directory. */
export class ConstructorCatalog {
    /** The package containing each module directory. */
    readonly #locations = new Map<string, Promise<PackageLocation>>();
    /** The identity of each package directory, a Destack package's or an npm package's. */
    readonly #owners = new Map<string, Promise<Package | DependencyPackage>>();
    /** The inspected constructors of each package directory. */
    readonly #constructors = new Map<string, Promise<Map<string, InspectedConstructor>>>();
    /** The exports of each package directory, absent when its package.json lists none. */
    readonly #exports = new Map<
        string,
        Promise<Readonly<Record<string, PackageExport>> | undefined>
    >();
    /** The packages and declared constructors the inspection finds. */
    readonly #packages: PackageLocator;
    /** The versions the build releases its source packages at, by package name. */
    readonly #versions: Readonly<Record<string, Version>>;
    /** The files the packages are read from. */
    readonly files: FileSystem;

    /** Inspect the constructors of a build's packages, the sources at the versions it releases. */
    constructor(versions: Readonly<Record<string, Version>>, files: FileSystem) {
        this.#versions = versions;
        this.files = files;
        this.#packages = new PackageLocator(files, versions);
    }

    /** Find the package containing a module directory. */
    locate(directory: string): Promise<PackageLocation> {
        let location = this.#locations.get(directory);
        if (location === undefined) {
            location = modulePackage(directory, this.#versions, this.files);
            this.#locations.set(directory, location);
        }

        return location;
    }

    /** Read a package's identity, with the stable id of its destack.json if declared. */
    owner(location: PackageLocation): Promise<Package | DependencyPackage> {
        let owner = this.#owners.get(location.directory);
        if (owner === undefined) {
            owner = readOwner(location, this.files);
            this.#owners.set(location.directory, owner);
        }

        return owner;
    }

    /** Read a Destack package's identity, requiring its destack.json. */
    async identify(location: PackageLocation): Promise<Package> {
        const owner = await this.owner(location);
        if (!("id" in owner)) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `${location.name} has Destack declarations but no destack.json`,
            );
        }

        return owner;
    }

    /** Read a package's inspected constructors and locate their describing functions. */
    inspected(location: PackageLocation): Promise<Map<string, InspectedConstructor>> {
        let inspected = this.#constructors.get(location.directory);
        if (inspected === undefined) {
            inspected = this.#readInspected(location);
            this.#constructors.set(location.directory, inspected);
        }

        return inspected;
    }

    /** Parse the inspected constructors of a package once, with its exports and identity. */
    async #readInspected(location: PackageLocation): Promise<Map<string, InspectedConstructor>> {
        // skip packages without described constructors before reading their manifests
        const declared = Object.entries(
            await this.#packages.constructors(location.name, location.directory),
        );
        const inspected = new Map<string, InspectedConstructor>();
        if (declared.length === 0) {
            return inspected;
        }

        // read the package identity once for every constructor
        const identity = await this.identify(location);
        for (const [name, constructor] of declared) {
            // describe each kind the constructor lists
            const kinds: ConstructorKind[] = [];
            for (const entry of constructor.describes) {
                kinds.push(await this.#describeKind(location, identity, entry));
            }
            inspected.set(name, { package: identity, kinds });
        }

        return inspected;
    }

    /** Describe a constructor's kind, with the functions of the package declaring it. */
    async #describeKind(
        location: PackageLocation,
        identity: Package,
        entry: DeclarationKind | DeclarationKindValue,
    ): Promise<ConstructorKind> {
        // locate a kind of this package, inspected by the functions it names
        if (!("package" in entry)) {
            return {
                kind: entry.kind,
                package: identity,
                inspector: await this.#locateInspector(location, entry),
            };
        }

        // locate a dependency's kind, inspected by its functions on the value read here
        const declaring = await this.#locateKind(entry.package, entry.kind, location);

        return {
            kind: entry.kind,
            package: declaring.package,
            inspector: {
                value: await this.#locateFunction(location, entry.value),
                ...(await this.#locateInspector(declaring.location, declaring.entry)),
            },
        };
    }

    /** Locate the functions a package lists for a kind it declares. */
    async #locateInspector(location: PackageLocation, kind: DeclarationKind): Promise<Inspector> {
        // locate the describing function and the optional ones
        const describe = await this.#locateFunction(location, kind.function);
        const compare = await this.#locateOptional(location, kind.compare);
        const vocabulary = await this.#locateOptional(location, kind.vocabulary);
        const symbols = await this.#locateOptional(location, kind.symbols);

        return {
            describe,
            ...(compare === undefined ? {} : { compare }),
            ...(vocabulary === undefined ? {} : { vocabulary }),
            ...(symbols === undefined ? {} : { symbols }),
        };
    }

    /** Find the dependency and entry declaring a kind another package's constructor describes. */
    async #locateKind(
        name: string,
        kind: string,
        location: PackageLocation,
    ): Promise<{
        package: Package;
        location: PackageLocation;
        entry: DeclarationKind;
    }> {
        // require a dependency whose constructors describe the kind as theirs
        const directory = await this.#packages.directory(name, location.directory);
        const entry = Object.values(await this.#packages.constructors(name, location.directory))
            .flatMap((constructor) => constructor.describes)
            .find(
                (candidate): candidate is DeclarationKind =>
                    candidate.kind === kind && !("package" in candidate),
            );
        if (directory === undefined || entry === undefined) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `${location.name} describes kind ${kind}, but ${name} does not declare it`,
            );
        }
        const declaring = await this.locate(directory);

        return { package: await this.identify(declaring), location: declaring, entry };
    }

    /** Locate a function a package exports, such as `./inspect#describeDatabase`. */
    async #locateFunction(location: PackageLocation, reference: string): Promise<FunctionExport> {
        // require the package export of the function, which may be conditional
        const separator = reference.indexOf("#");
        if (separator === -1) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `function reference ${reference} is not an export`,
            );
        }
        const subpath = reference.slice(0, separator);
        const name = reference.slice(separator + 1);
        const exports = await this.#exported(location);
        const target = exports?.[subpath];
        if (target === undefined) {
            throw BuildError.of(
                "INSPECTION_FAILED",
                `${location.name} does not export ${subpath} for ${reference}`,
            );
        }

        return { package: location.name, directory: location.directory, subpath, target, name };
    }

    /** Locate a function a package exports when a reference refers to one. */
    async #locateOptional(
        location: PackageLocation,
        reference: string | undefined,
    ): Promise<FunctionExport | undefined> {
        return reference === undefined
            ? undefined
            : await this.#locateFunction(location, reference);
    }

    /** Read the exports a package's package.json lists. */
    #exported(
        location: PackageLocation,
    ): Promise<Readonly<Record<string, PackageExport>> | undefined> {
        let exported = this.#exports.get(location.directory);
        if (exported === undefined) {
            exported = readExports(location, this.files);
            this.#exports.set(location.directory, exported);
        }

        return exported;
    }
}

/** Read a package's identity from its package.json, with the id of its optional destack.json. */
async function readOwner(
    location: PackageLocation,
    files: FileSystem,
): Promise<Package | DependencyPackage> {
    // read the optional Destack definition and preserve every other filesystem failure
    let definition: string | undefined;
    try {
        definition = await files.readTextFile(join(location.directory, "destack.json"));
    } catch (error) {
        if (!FileSystemError.is(error, "NOT_FOUND")) {
            throw error;
        }
    }

    // retain stable identity for Destack packages and registry identity for npm packages
    const identity = { name: location.name, version: location.version };

    return definition === undefined
        ? DependencyPackage.parse(identity)
        : Package.parse({ ...identity, id: PackageDefinition.read(definition).id });
}

/** Read the exports a package's package.json lists, absent when it lists none. */
async function readExports(
    location: PackageLocation,
    files: FileSystem,
): Promise<Readonly<Record<string, PackageExport>> | undefined> {
    const manifest = ExportsManifest.parse(
        JSON.parse(await files.readTextFile(join(location.directory, "package.json"))),
    );

    return manifest.exports;
}
