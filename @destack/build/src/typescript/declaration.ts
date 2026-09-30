import { dirname, join, relative } from "node:path";
import { readFile } from "node:fs/promises";
import {
    type Project,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
} from "typescript/unstable/async";
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
import type { DeclarationDescription } from "@destack/package/inspect";
import { type DeclarationConstructor, Package, PackageDefinition } from "@destack/package";
import { BuildError } from "../error/index.ts";
import { PackageLocator } from "@destack/package/transform";
import { modulePackage } from "../source/dependency.ts";
import type { TestDeclaration } from "@destack/test/inspect";

import type { DeclarationExport, FunctionExport, Inspector } from "../declaration/declaration.ts";

/** A package containing modules, with the name and version its package.json declares. */
type PackageLocation = Awaited<ReturnType<typeof modulePackage>>;

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

/** The packages and inspected constructors one inspection reaches, each read once per package directory. */
export class ConstructorCatalog {
    /** The package containing each module directory. */
    readonly #locations = new Map<string, Promise<PackageLocation>>();
    /** The identity of each package directory. */
    readonly #identities = new Map<string, Promise<Package>>();
    /** The inspected constructors of each package directory. */
    readonly #constructors = new Map<string, Promise<Map<string, InspectedConstructor>>>();
    /** The exports of each package directory, absent when its package.json lists none. */
    readonly #exports = new Map<string, Promise<Record<string, unknown> | undefined>>();
    /** The packages and declared constructors the inspection finds. */
    readonly #packages = new PackageLocator();

    /** Resolve a call target to an inspected constructor, or undefined for any other function. */
    async resolve(
        symbol: TypeScriptSymbol | undefined,
        project: Project,
    ): Promise<ResolvedConstructor | undefined> {
        // follow imported constructor aliases to a function declaration
        if (symbol && symbol.flags & SymbolFlags.Alias) {
            symbol = await project.checker.getAliasedSymbol(symbol);
        }
        const declaration = await symbol?.declarations[0]?.resolve(project);
        if (!symbol || !declaration || !isFunctionDeclaration(declaration)) {
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

    /** Read a Destack package's identity from its destack.json. */
    identify(location: PackageLocation): Promise<Package> {
        let identity = this.#identities.get(location.directory);
        if (identity === undefined) {
            identity = readIdentity(location);
            this.#identities.set(location.directory, identity);
        }

        return identity;
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
        // skip packages without inspected constructors before reading their manifests
        const declared = Object.entries(
            this.#packages.constructors(location.name, location.directory),
        );
        const inspected = new Map<string, InspectedConstructor>();
        if (declared.every(([, constructor]) => constructor.describes === undefined)) {
            return inspected;
        }

        // read the package identity once for every constructor
        const identity = await this.identify(location);
        for (const [name, constructor] of declared) {
            // describe each kind the constructor lists, compared and listed by the kind's own entry
            const kinds: ConstructorKind[] = [];
            for (const entry of constructor.describes ?? []) {
                const declaring =
                    entry.package === undefined
                        ? { package: identity, location, entry }
                        : await this.#locateKind(entry.package, entry.kind, location);
                if (entry.package !== undefined && (entry.compare ?? entry.vocabulary)) {
                    throw new BuildError(
                        "INSPECTION_FAILED",
                        `${location.name} describes kind ${entry.kind} of ${entry.package}, which compares and lists its terms itself`,
                    );
                }
                kinds.push({
                    kind: entry.kind,
                    package: declaring.package,
                    inspector: {
                        describe: await this.#locateFunction(location, entry.function),
                        ...(declaring.entry.compare === undefined
                            ? {}
                            : {
                                  compare: await this.#locateFunction(
                                      declaring.location,
                                      declaring.entry.compare,
                                  ),
                              }),
                        ...(declaring.entry.vocabulary === undefined
                            ? {}
                            : {
                                  vocabulary: await this.#locateFunction(
                                      declaring.location,
                                      declaring.entry.vocabulary,
                                  ),
                              }),
                    },
                });
            }

            // keep constructors the build inspects
            if (kinds.length > 0) {
                inspected.set(name, { package: identity, kinds });
            }
        }

        return inspected;
    }

    /** Find the dependency declaring a kind another package's constructor describes, with its own entry. */
    async #locateKind(
        name: string,
        kind: string,
        location: PackageLocation,
    ): Promise<{
        package: Package;
        location: PackageLocation;
        entry: NonNullable<DeclarationConstructor["describes"]>[number];
    }> {
        // require a dependency whose own constructors describe the kind
        const directory = this.#packages.directory(name, location.directory);
        const entry = Object.values(this.#packages.constructors(name, location.directory))
            .flatMap((constructor) => constructor.describes ?? [])
            .find((entry) => entry.kind === kind && entry.package === undefined);
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
        const [subpath, name] = reference.split("#") as [string, string];
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

    /** Read the exports a package's package.json lists. */
    #exported(location: PackageLocation): Promise<Record<string, unknown> | undefined> {
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
    project: Project,
    sourcePackage: PackageLocation,
    catalog: ConstructorCatalog,
    tests: readonly TestDeclaration[] = [],
): Promise<DeclarationExport[]> {
    // collect calls before resolving constructor symbols in one request
    const declarations: DeclarationExport[] = [];
    const calls: CallExpression[] = [];
    const testEnds = new Map(
        tests.filter((test) => test.kind === "test").map((test) => [test.start, test.end]),
    );
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        // leave test case setup and bodies to the test runner
        const testEnd = testEnds.get(node.getStart());
        if (testEnd !== undefined && node.end <= testEnd) {
            continue;
        }

        node.forEachChild((child) => {
            pending.push(child);
        });
        if (isCallExpression(node)) {
            calls.push(node);
        }
    }

    // resolve declaration constructors in one compiler request
    const symbols = await project.checker.getSymbolAtLocation(calls.map((node) => node.expression));
    const constructors = await Promise.all(
        symbols.map((symbol) => catalog.resolve(symbol, project)),
    );
    if (constructors.every((resolved) => resolved === undefined)) {
        return declarations;
    }

    // resolve stable identity only for packages exporting Destack declarations
    const declaring = await catalog.identify(sourcePackage);

    // identify exports through the compiler, including export lists and aliases
    const module = await project.checker.getSymbolAtLocation(source);
    const exports = module ? await project.checker.getExportsOfModule(module) : [];
    const exported = new Map<number, string>();
    for (const symbol of exports) {
        const target =
            symbol.flags & SymbolFlags.Alias
                ? await project.checker.getAliasedSymbol(symbol)
                : symbol;
        exported.set(target.id, symbol.name);
    }

    // locate exported constants for evaluation
    for (const [index, node] of calls.entries()) {
        const resolved = constructors[index];
        if (!resolved) {
            continue;
        }
        const { constructor, kinds } = resolved;

        // require a named variable initialized by the constructor
        const declaration = node.parent;
        if (
            !isVariableDeclaration(declaration) ||
            !declaration.name ||
            !isIdentifier(declaration.name)
        ) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${constructor.symbol.name} requires a named module declaration`,
            );
        }

        // require an exported reference to evaluate
        const symbol = await project.checker.getSymbolAtLocation(declaration.name);
        const exportedName = symbol && exported.get(symbol.id);
        if (!exportedName) {
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

        // retain each exported declaration once
        const symbolName = declaration.name.text;
        if (declarations.some((entry) => entry.description.symbol.symbol.name === symbolName)) {
            continue;
        }

        // retain one description per kind at package-relative locations
        const prefix = source.text.slice(0, node.getStart());
        for (const { kind, package: owner, inspector } of kinds) {
            declarations.push({
                inspector,
                file: source.fileName,
                export: exportedName,
                description: {
                    kind,
                    package: owner,
                    constructor,
                    name: symbolName,
                    symbol: { package: declaring, symbol: { module: file, name: symbolName } },
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

/** Read a Destack package's identity from its destack.json and package.json. */
async function readIdentity(location: PackageLocation): Promise<Package> {
    const definition = PackageDefinition.read(
        await readFile(join(location.directory, "destack.json"), "utf8"),
    );

    return Package.parse({ id: definition.id, name: location.name, version: location.version });
}

/** Read the exports a package's package.json lists, absent when it lists none. */
async function readExports(
    location: PackageLocation,
): Promise<Record<string, unknown> | undefined> {
    const manifest = JSON.parse(await readFile(join(location.directory, "package.json"), "utf8"));

    return manifest.exports;
}
