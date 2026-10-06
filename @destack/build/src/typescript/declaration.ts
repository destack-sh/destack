import { dirname, join, relative } from "node:path";
import { readFile } from "node:fs/promises";
import type { Symbol as TypeScriptSymbol } from "typescript/unstable/async";
import {
    type CallExpression,
    isCallExpression,
    isFunctionDeclaration,
    isIdentifier,
    isVariableDeclaration,
    type Node,
    NodeFlags,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import {
    type DeclarationConstructor,
    DependencyPackage,
    Package,
    PackageDefinition,
    PackageExport,
    type DeclarationDescription,
} from "@destack/package";
import { BuildError, isMissing } from "../error/index.ts";
import { schema } from "@destack/schema";
import { type ModulePackage, PackageLocator } from "@destack/package/transform";
import { modulePackage } from "../source/dependency.ts";
import type { TestDeclaration } from "@destack/test/inspect";

import type { DeclarationExport, FunctionExport, Inspector } from "../declaration/declaration.ts";
import type { SymbolInspector } from "./symbol.ts";

/** The exports a package.json lists. */
const ExportsManifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
});

/** A package containing modules, with the name and version its package.json declares. */
type PackageLocation = Awaited<ReturnType<typeof modulePackage>>;

/** A kind a constructor's entry describes declarations as, with the functions it names. */
type DescribedKind = DeclarationConstructor["describes"][number];

/** A kind a constructor describes each declaration as, with the functions inspecting it. */
interface ConstructorKind {
    /** The description kind. */
    readonly kind: string;
    /** The package declaring the kind. */
    readonly package: Package;
    /** The functions inspecting the kind. */
    readonly inspector: Inspector;
}

/** A declaration constructor its package declares as inspected. */
interface InspectedConstructor {
    /** The declaring package's identity. */
    readonly package: Package;
    /** The kinds it describes each declaration as. */
    readonly kinds: readonly ConstructorKind[];
}

/** A resolved constructor call target. */
interface ResolvedConstructor {
    /** The constructor's package and symbol. */
    readonly constructor: DeclarationDescription["constructor"];
    /** The kinds it describes each declaration as. */
    readonly kinds: readonly ConstructorKind[];
}

/** The packages and inspected constructors one inspection finds, each read once per package directory. */
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
    /** The package the inspection's build compiles, as it releases it. */
    readonly #compiled: ModulePackage;

    /** Inspect the constructors of a build's packages, the compiled one as the build releases it. */
    constructor(compiled: ModulePackage) {
        this.#compiled = compiled;
        this.#packages = new PackageLocator(compiled);
    }

    /** Resolve a call target to an inspected constructor, or undefined for any other function. */
    async resolve(
        symbol: TypeScriptSymbol | undefined,
        inspector: SymbolInspector,
    ): Promise<ResolvedConstructor | undefined> {
        // follow imported constructor aliases to a function declaration
        const project = inspector.project;
        if (symbol) {
            symbol = await inspector.original(symbol);
        }
        const handle = symbol?.declarations[0];
        if (!symbol || handle?.kind !== SyntaxKind.FunctionDeclaration) {
            return undefined;
        }

        // leave compiler library functions unread, since no package declares them
        const metadata = await project.program.getSourceFileMetadataByPath(handle.path);
        if (metadata === undefined) {
            throw new BuildError("INSPECTION_FAILED", `undeclared source file: ${handle.path}`);
        }
        if (metadata.isDefaultLibrary) {
            return undefined;
        }
        const declaration = await inspector.node(handle);
        if (!isFunctionDeclaration(declaration)) {
            return undefined;
        }

        // require a function its package declares as an inspected constructor
        const file = declaration.getSourceFile().fileName;
        const location = await this.locate(dirname(file));
        const inspected = (await this.#inspected(location)).get(symbol.name);
        if (inspected === undefined) {
            return undefined;
        }
        const module = relative(location.directory, file).replaceAll("\\", "/");

        return {
            constructor: { package: inspected.package, symbol: { module, name: symbol.name } },
            kinds: inspected.kinds,
        };
    }

    /** Find the package containing a module directory. */
    locate(directory: string): Promise<PackageLocation> {
        let location = this.#locations.get(directory);
        if (location === undefined) {
            location = modulePackage(directory);
            this.#locations.set(directory, location);
        }

        return location;
    }

    /** Read a package's identity, with the stable id its destack.json declares when it has one, the compiled package's as its build releases it. */
    owner(location: PackageLocation): Promise<Package | DependencyPackage> {
        let owner = this.#owners.get(location.directory);
        if (owner === undefined) {
            owner =
                location.directory === this.#compiled.directory
                    ? Promise.resolve(this.#compiled.metadata.package)
                    : readOwner(location);
            this.#owners.set(location.directory, owner);
        }

        return owner;
    }

    /** Read a Destack package's identity, requiring its destack.json. */
    async identify(location: PackageLocation): Promise<Package> {
        const owner = await this.owner(location);
        if (!("id" in owner)) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${location.name} declares Destack declarations without a destack.json`,
            );
        }

        return owner;
    }

    /** Read a package's inspected constructors and locate their describing functions. */
    #inspected(location: PackageLocation): Promise<Map<string, InspectedConstructor>> {
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
            this.#packages.constructors(location.name, location.directory),
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

    /** Describe a kind a constructor lists, with the functions of the entry declaring the kind. */
    async #describeKind(
        location: PackageLocation,
        identity: Package,
        entry: DescribedKind,
    ): Promise<ConstructorKind> {
        // locate the entry declaring the kind: this one, or a dependency's
        const declaring =
            entry.package === undefined
                ? { package: identity, location, entry }
                : await this.#locateKind(entry.package, entry.kind, location);
        const hasFunctions =
            entry.compare !== undefined ||
            entry.vocabulary !== undefined ||
            entry.symbols !== undefined;
        if (entry.package !== undefined && hasFunctions) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${location.name} describes kind ${entry.kind} of ${entry.package}, which compares, lists its terms and derives its symbols itself`,
            );
        }

        // locate the describing function here and the kind's functions at its declaration
        const describe = await this.#locateFunction(location, entry.function);
        const kind = declaring.entry;
        const compare = await this.#locateOptional(declaring.location, kind.compare);
        const vocabulary = await this.#locateOptional(declaring.location, kind.vocabulary);
        const symbols = await this.#locateOptional(declaring.location, kind.symbols);

        return {
            kind: entry.kind,
            package: declaring.package,
            inspector: {
                describe,
                ...(compare === undefined ? {} : { compare }),
                ...(vocabulary === undefined ? {} : { vocabulary }),
                ...(symbols === undefined ? {} : { symbols }),
            },
        };
    }

    /** Find the dependency declaring a kind another package's constructor describes, with the entry declaring it. */
    async #locateKind(
        name: string,
        kind: string,
        location: PackageLocation,
    ): Promise<{
        package: Package;
        location: PackageLocation;
        entry: DescribedKind;
    }> {
        // require a dependency whose constructors describe the kind as theirs
        const directory = this.#packages.directory(name, location.directory);
        const entry = Object.values(this.#packages.constructors(name, location.directory))
            .flatMap((constructor) => constructor.describes)
            .find((candidate) => candidate.kind === kind && candidate.package === undefined);
        if (directory === undefined || entry === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${location.name} describes kind ${kind}, which ${name} does not declare`,
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
            throw new TypeError(`function reference ${reference} names no export`);
        }
        const subpath = reference.slice(0, separator);
        const name = reference.slice(separator + 1);
        const exports = await this.#exported(location);
        const target = exports?.[subpath];
        if (target === undefined) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${location.name} does not export ${subpath} for ${reference}`,
            );
        }

        return { directory: location.directory, subpath, target, name };
    }

    /** Locate a function a package exports when a reference names one. */
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
            exported = readExports(location);
            this.#exports.set(location.directory, exported);
        }

        return exported;
    }
}

/** Locate exported declarations without evaluating package modules. */
export async function collectDeclarations(
    source: SourceFile,
    file: string,
    inspector: SymbolInspector,
    sourcePackage: PackageLocation,
    catalog: ConstructorCatalog,
    tests: readonly TestDeclaration[] = [],
): Promise<DeclarationExport[]> {
    // resolve declaration constructors in one compiler request
    const project = inspector.project;
    const calls = collectCalls(source, tests);
    const symbols = await project.checker.getSymbolAtLocation(calls.map((node) => node.expression));
    const constructors = await Promise.all(
        symbols.map((symbol) => catalog.resolve(symbol, inspector)),
    );
    if (constructors.every((resolved) => resolved === undefined)) {
        return [];
    }

    // resolve stable identity only for packages exporting Destack declarations
    const declaring = await catalog.identify(sourcePackage);
    const exported = await exportedNames(source, inspector);

    // locate exported constants for evaluation, each once
    const declarations: DeclarationExport[] = [];
    for (const [index, node] of calls.entries()) {
        const resolved = constructors[index];
        if (!resolved) {
            continue;
        }
        const located = await locateDeclaration(node, resolved, exported, file, inspector);
        if (declarations.some((entry) => entry.description.symbol.symbol.name === located.name)) {
            continue;
        }

        // retain one description per kind at package-relative locations
        const prefix = source.text.slice(0, node.getStart());
        for (const { kind, package: owner, inspector: functions } of resolved.kinds) {
            declarations.push({
                inspector: functions,
                file: source.fileName,
                export: located.export,
                description: {
                    kind,
                    package: owner,
                    constructor: resolved.constructor,
                    name: located.name,
                    symbol: { package: declaring, symbol: { module: file, name: located.name } },
                    source: {
                        file,
                        line: prefix.split("\n").length - 1,
                        column: prefix.length - prefix.lastIndexOf("\n") - 1,
                    },
                },
            });
        }
    }

    return declarations;
}

/** Collect a module's calls, leaving test case setup and bodies to the test runner. */
function collectCalls(source: SourceFile, tests: readonly TestDeclaration[]): CallExpression[] {
    // index where each test case ends
    const calls: CallExpression[] = [];
    const testEnds = new Map(
        tests.filter((test) => test.kind === "test").map((test) => [test.start, test.end]),
    );
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        // skip test cases
        const testEnd = testEnds.get(node.getStart());
        if (testEnd !== undefined && node.end <= testEnd) {
            continue;
        }

        // visit the children and keep calls
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (isCallExpression(node)) {
            calls.push(node);
        }
    }

    return calls;
}

/** Name each symbol a module exports through the compiler, including export lists and aliases, by symbol. */
export async function exportedNames(
    source: SourceFile,
    inspector: SymbolInspector,
): Promise<Map<number, string>> {
    // name each export's original symbol
    const checker = inspector.project.checker;
    const module = await checker.getSymbolAtLocation(source);
    const exports = module ? await checker.getExportsOfModule(module) : [];
    const exported = new Map<number, string>();
    for (const symbol of exports) {
        const target = await inspector.original(symbol);
        exported.set(target.id, symbol.name);
    }

    return exported;
}

/** Require a constructor call to initialize an exported module constant, returning its name and export. */
async function locateDeclaration(
    node: CallExpression,
    resolved: ResolvedConstructor,
    exported: ReadonlyMap<number, string>,
    file: string,
    inspector: SymbolInspector,
): Promise<{ name: string; export: string }> {
    // require a named variable initialized by the constructor
    const { constructor } = resolved;
    const declaration = node.parent;
    if (!isVariableDeclaration(declaration) || !isIdentifier(declaration.name)) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${constructor.symbol.name} requires a named module declaration`,
        );
    }

    // require an exported reference to evaluate
    const symbol = await inspector.project.checker.getSymbolAtLocation(declaration.name);
    const exportedName = symbol === undefined ? undefined : exported.get(symbol.id);
    if (exportedName === undefined) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `export ${file}:${declaration.name.text} to inspect its declaration`,
        );
    }

    // require an unconditional module constant
    if (
        !(declaration.parent.flags & NodeFlags.Const) ||
        declaration.parent.parent.parent.kind !== SyntaxKind.SourceFile
    ) {
        throw new BuildError(
            "INSPECTION_FAILED",
            `${constructor.symbol.name} must initialize a module-level constant`,
        );
    }

    return { name: declaration.name.text, export: exportedName };
}

/** Read a package's identity from its package.json, with the id of its optional destack.json. */
async function readOwner(location: PackageLocation): Promise<Package | DependencyPackage> {
    // read the optional Destack definition and preserve every other filesystem failure
    let definition: string | undefined;
    try {
        definition = await readFile(join(location.directory, "destack.json"), "utf8");
    } catch (error) {
        if (!isMissing(error)) {
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
): Promise<Readonly<Record<string, PackageExport>> | undefined> {
    const manifest = ExportsManifest.parse(
        JSON.parse(await readFile(join(location.directory, "package.json"), "utf8")),
    );

    return manifest.exports;
}
