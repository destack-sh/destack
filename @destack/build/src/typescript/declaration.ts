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
import { Package } from "@destack/package";
import { BuildError } from "../error/index.ts";
import { packageConstructors } from "@destack/package/transform";
import { modulePackage } from "../source/dependency.ts";
import type { TestDeclaration } from "@destack/test/inspect";

import type { DeclarationExport } from "../declaration/declaration.ts";

/** A package holding modules, as its package.json names it. */
type PackageLocation = Awaited<ReturnType<typeof modulePackage>>;

/** A declaration constructor its package declares as inspected, with its describing function. */
interface InspectedConstructor {
    /** The manifest description kind. */
    readonly kind: string;
    /** The declaring package's identity. */
    readonly package: Package;
    /** The describing function. */
    readonly inspector: DeclarationExport["inspector"];
}

/** A resolved constructor call target. */
interface ResolvedConstructor {
    /** The describing function. */
    readonly inspector: DeclarationExport["inspector"];
    /** The constructor's package and symbol. */
    readonly constructor: DeclarationDescription["constructor"];
    /** The manifest description kind. */
    readonly kind: string;
}

/** The packages and inspected constructors one inspection reaches, each read once per package directory. */
export class ConstructorCatalog {
    /** The package containing each module directory. */
    readonly #locations = new Map<string, Promise<PackageLocation>>();
    /** The identity of each package directory. */
    readonly #identities = new Map<string, Promise<Package>>();
    /** The inspected constructors of each package directory. */
    readonly #constructors = new Map<string, Promise<Map<string, InspectedConstructor>>>();

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
            inspector: inspected.inspector,
            constructor: { package: inspected.package, symbol: { module, name: symbol.name } },
            kind: inspected.kind,
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
        const declared = Object.entries(packageConstructors(location.name, location.directory));
        const inspected = new Map<string, InspectedConstructor>();
        if (declared.every(([, constructor]) => constructor.inspect === undefined)) {
            return inspected;
        }

        // read the package exports and identity once for every constructor
        const manifest = JSON.parse(
            await readFile(join(location.directory, "package.json"), "utf8"),
        );
        const identity = await this.identify(location);
        for (const [name, constructor] of declared) {
            if (constructor.inspect === undefined) {
                continue;
            }

            // locate the describing function's export, which may be conditional
            const [subpath, exportName] = constructor.inspect.describe.split("#") as [
                string,
                string,
            ];
            const target: unknown = manifest.exports?.[subpath];
            if (target === undefined) {
                throw new BuildError(
                    "INSPECTION_FAILED",
                    `${location.name} does not export ${subpath} for ${name}`,
                );
            }
            inspected.set(name, {
                kind: constructor.inspect.kind,
                package: identity,
                inspector: { directory: location.directory, subpath, target, name: exportName },
            });
        }

        return inspected;
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
    const owner = await catalog.identify(sourcePackage);

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
        const { inspector, constructor, kind } = resolved;

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

        // retain package-relative locations independently of the evaluation directory
        const prefix = source.text.slice(0, node.getStart());
        declarations.push({
            inspector,
            file: source.fileName,
            export: exportedName,
            description: {
                kind,
                constructor,
                name: symbolName,
                symbol: { package: owner, symbol: { module: file, name: symbolName } },
                source: {
                    file,
                    line: prefix.split("\n").length - 1,
                    column: prefix.length - prefix.lastIndexOf("\n") - 1,
                },
            },
        });
    }

    return declarations;
}

/** Read a Destack package's identity from its destack.json and package.json. */
async function readIdentity(location: PackageLocation): Promise<Package> {
    const definition = JSON.parse(await readFile(join(location.directory, "destack.json"), "utf8"));

    return Package.parse({ id: definition.id, name: location.name, version: location.version });
}
