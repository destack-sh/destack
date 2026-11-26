import { dirname, join } from "node:path";
import { type FileSystem, FileSystemError } from "@destack/environment";
import {
    type DeclarationKindDefinition,
    type DeclarationKindValue,
    DependencyPackage,
    PackageVersion,
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
    readonly package: PackageVersion;
    /** The member of each declaration holding the kind's value, if not the declaration itself. */
    readonly member?: string;
    /** The functions inspecting the kind. */
    readonly inspector: Inspector;
}

/** A declaration constructor its package declares as inspected. */
export interface InspectedConstructor {
    /** The declaring package's identity. */
    readonly package: PackageVersion;
    /** The kinds it describes each declaration as. */
    readonly kinds: readonly ConstructorKind[];
}

/** The packages and constructors one inspection finds, each read once per directory. */
export class ConstructorCatalog {
    /** The build's own cache of the packages outside its stored roots. */
    readonly #own = new PackageCache();
    /** The packages and declared constructors the inspection finds. */
    readonly #packages: PackageLocator;
    /** The versions the build releases its source packages at, by package name. */
    readonly #versions: Readonly<Record<string, Version>>;
    /** The files the packages are read from. */
    readonly files: FileSystem;
    /** The cache shared with other builds, for the stored roots of the build's tree. */
    readonly #stored: PackageCache;

    /** Inspect the constructors of a build's packages, the sources at the versions it releases. */
    constructor(
        versions: Readonly<Record<string, Version>>,
        files: FileSystem,
        stored: PackageCache,
    ) {
        // read the packages at the versions, keeping the stored roots' cache for other builds
        this.#versions = versions;
        this.files = files;
        this.#packages = new PackageLocator(files, versions);
        this.#stored = stored;
    }

    /** Find the package containing a module directory. */
    locate(directory: string): Promise<PackageLocation> {
        const { locations } = this.#cacheOf(directory);

        return cached(locations, directory, () =>
            modulePackage(directory, this.#versions, this.files),
        );
    }

    /** Read a package's identity, with the stable id of its destack.json if declared. */
    owner(location: PackageLocation): Promise<PackageVersion | DependencyPackage> {
        const { owners } = this.#cacheOf(location.directory);

        return cached(owners, location.directory, () => readOwner(location, this.files));
    }

    /** Read a Destack package's identity, requiring its destack.json. */
    async identify(location: PackageLocation): Promise<PackageVersion> {
        const owner = await this.owner(location);
        if (!("id" in owner)) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `package '${location.name}' is invalid: it has Destack declarations but no destack.json`,
            );
        }

        return owner;
    }

    /** Read a package's inspected constructors and locate their describing functions. */
    inspected(location: PackageLocation): Promise<Map<string, InspectedConstructor>> {
        const { constructors } = this.#cacheOf(location.directory);

        return cached(constructors, location.directory, () => this.#readInspected(location));
    }

    /** Return a directory's cache: the stored packages' below a root, else the build's own. */
    #cacheOf(directory: string): PackageCache {
        return this.#stored.isStored(directory) ? this.#stored : this.#own;
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
        identity: PackageVersion,
        entry: DeclarationKindDefinition | DeclarationKindValue,
    ): Promise<ConstructorKind> {
        // locate a kind of this package, inspected by the functions it names
        if (!("package" in entry)) {
            return {
                kind: entry.kind,
                package: identity,
                inspector: await this.#locateInspector(location, entry),
            };
        }

        // locate a dependency's kind, described by this package's functions where it names them
        const declaring = await this.#locateKind(entry.package, entry.kind, location);
        if (!("member" in entry)) {
            return {
                kind: entry.kind,
                package: declaring.package,
                inspector: await this.#locateInspector(location, entry),
            };
        }

        // else inspect the member it names by the kind's functions
        return {
            kind: entry.kind,
            package: declaring.package,
            member: entry.member,
            inspector: await this.#locateInspector(declaring.location, declaring.entry),
        };
    }

    /** Locate the functions a package lists for a kind it declares. */
    async #locateInspector(
        location: PackageLocation,
        kind: DeclarationKindDefinition,
    ): Promise<Inspector> {
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
        package: PackageVersion;
        location: PackageLocation;
        entry: DeclarationKindDefinition;
    }> {
        // require a dependency whose constructors describe the kind as theirs
        const directory = await this.#packages.directory(name, location.directory);
        const constructed = Object.values(
            await this.#packages.constructors(name, location.directory),
        )
            .flatMap((constructor) => constructor.describes)
            .filter(
                (candidate): candidate is DeclarationKindDefinition => !("package" in candidate),
            );
        const defined = await this.#packages.kinds(name, location.directory);
        const entry = [...constructed, ...defined].find((candidate) => candidate.kind === kind);
        if (directory === undefined || entry === undefined) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `kind ${kind} is invalid: '${location.name}' describes it, but '${name}' does not declare it`,
            );
        }
        // refuse a stored package's kind outside the stored roots, as a stored root links what it requires
        if (this.#stored.isStored(location.directory) && !this.#stored.isStored(directory)) {
            throw BuildError.of(
                "INVARIANT_BROKEN",
                `kind ${kind} is invalid: stored '${location.name}' resolves '${name}' outside its tree, at '${directory}'`,
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
                "INSPECTION_INVALID",
                `function reference '${reference}' is invalid: it is not an export`,
            );
        }
        const subpath = reference.slice(0, separator);
        const name = reference.slice(separator + 1);
        const exports = await this.#exported(location);
        const target = exports?.[subpath];
        if (target === undefined) {
            throw BuildError.of(
                "INSPECTION_INVALID",
                `function reference '${reference}' is invalid: '${location.name}' does not export '${subpath}'`,
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
        const { exports } = this.#cacheOf(location.directory);
        const exported = cached(exports, location.directory, () =>
            readExports(location, this.files),
        );

        return exported;
    }
}

/** What builds read of packages, by directory, shared below their trees' stored roots. */
export class PackageCache {
    /** The package containing each module directory. */
    readonly locations = new Map<string, Promise<PackageLocation>>();
    /** The identity of each package directory, a Destack package's or an npm package's. */
    readonly owners = new Map<string, Promise<PackageVersion | DependencyPackage>>();
    /** The inspected constructors of each package directory. */
    readonly constructors = new Map<string, Promise<Map<string, InspectedConstructor>>>();
    /** The exports of each package directory, absent when its package.json lists none. */
    readonly exports = new Map<
        string,
        Promise<Readonly<Record<string, PackageExport>> | undefined>
    >();
    /** The stored roots of the current build's tree, below which the cache never changes. */
    #roots: ReadonlySet<string> = new Set();

    /** Take a build's stored roots, forgetting the entries below none of them. */
    retain(roots: ReadonlySet<string>): void {
        this.#roots = roots;
        for (const entries of [this.locations, this.owners, this.constructors, this.exports]) {
            for (const directory of entries.keys()) {
                if (!this.isStored(directory)) {
                    entries.delete(directory);
                }
            }
        }
    }

    /** Report whether a directory lies below a stored root, walking up its ancestors to one. */
    isStored(directory: string): boolean {
        for (let current = directory; dirname(current) !== current; current = dirname(current)) {
            if (this.#roots.has(current)) {
                return true;
            }
        }

        return false;
    }
}

/** Read a package's identity from its package.json, with the id of its optional destack.json. */
async function readOwner(
    location: PackageLocation,
    files: FileSystem,
): Promise<PackageVersion | DependencyPackage> {
    // read the optional Destack definition and preserve every other filesystem failure
    let definition: string | undefined;
    try {
        definition = await files.readTextFile(join(location.directory, "destack.json"));
    } catch (error) {
        if (!FileSystemError.is(error, "FILE_NOT_FOUND")) {
            throw error;
        }
    }

    // retain stable identity for Destack packages and registry identity for npm packages
    const identity = { name: location.name, version: location.version };

    return definition === undefined
        ? DependencyPackage.parse(identity)
        : PackageVersion.parse({ ...identity, id: PackageDefinition.read(definition).id });
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

/** Read a key's value from a cache, making it once and forgetting it when the making fails. */
function cached<Value>(
    cache: Map<string, Promise<Value>>,
    key: string,
    make: () => Promise<Value>,
): Promise<Value> {
    // reuse the key's value, else make it
    const known = cache.get(key);
    if (known !== undefined) {
        return known;
    }
    const made = make();
    cache.set(key, made);

    // forget a failed value, so the next read makes it again
    made.catch(() => {
        if (cache.get(key) === made) {
            cache.delete(key);
        }
    });

    return made;
}
