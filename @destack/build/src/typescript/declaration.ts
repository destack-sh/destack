import { dirname, join, relative } from "node:path";
import { readFile } from "node:fs/promises";
import type { Symbol as TypeScriptSymbol } from "typescript/unstable/async";
import {
    type CallExpression,
    isCallExpression,
    isExportAssignment,
    isFunctionDeclaration,
    isIdentifier,
    isPropertyAccessExpression,
    isReturnStatement,
    isVariableDeclaration,
    type Node,
    NodeFlags,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import {
    type DeclarationKind,
    type DeclarationKindValue,
    DependencyPackage,
    Package,
    PackageDefinition,
    PackageExport,
    type DeclarationDescription,
} from "@destack/package";
import { BuildError, isMissing } from "../error/index.ts";
import { nth, schema, type Version } from "@destack/schema";
import { PackageLocator } from "@destack/package/locator";
import { modulePackage } from "../source/dependency.ts";
import type { TestDeclaration } from "./test.ts";

import type { DeclarationExport, FunctionExport, Inspector } from "../declaration/declaration.ts";
import type { SymbolInspector } from "./symbol.ts";

/** The nodes whose bodies run once called, where a declaration factory makes the types it lists. */
const FUNCTION_KINDS: ReadonlySet<SyntaxKind> = new Set([
    SyntaxKind.FunctionDeclaration,
    SyntaxKind.FunctionExpression,
    SyntaxKind.ArrowFunction,
    SyntaxKind.MethodDeclaration,
    SyntaxKind.Constructor,
    SyntaxKind.GetAccessor,
    SyntaxKind.SetAccessor,
]);

/** The exports a package.json lists. */
const ExportsManifest = schema.looseObject({
    exports: schema.record(schema.string(), PackageExport).exactOptional(),
});

/** The name of a module's default export, as a declaration and its export. */
const DEFAULT_EXPORT = "default";

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

    /** Inspect the constructors of a build's packages, the sources at the versions it releases. */
    constructor(versions: Readonly<Record<string, Version>>) {
        this.#versions = versions;
        this.#packages = new PackageLocator(versions);
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
            throw BuildError.of("INSPECTION_FAILED", `undeclared source file: ${handle.path}`);
        }
        if (metadata.isDefaultLibrary) {
            return undefined;
        }
        // require a function its package declares as an inspected constructor
        const file = await inspector.file(handle);
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

    /** Resolve a factory, a function returning a constructor's call, to its constructor. */
    async resolveFactory(
        symbol: TypeScriptSymbol | undefined,
        inspector: SymbolInspector,
    ): Promise<ResolvedConstructor | undefined> {
        // require a function declaration with a body
        const original = symbol === undefined ? undefined : await inspector.original(symbol);
        const handle = original?.declarations[0];
        if (handle?.kind !== SyntaxKind.FunctionDeclaration) {
            return undefined;
        }
        const node = await inspector.node(handle);
        if (!isFunctionDeclaration(node) || node.body === undefined) {
            return undefined;
        }

        // resolve the constructor the first returned call chain starts with
        const returned = node.body.statements
            .filter(isReturnStatement)
            .map((statement) => statement.expression)
            .filter((expression) => expression !== undefined && isCallExpression(expression))
            .map((call) => chainStart(call));
        const callees = await inspector.project.checker.getSymbolAtLocation(
            returned.map((call) => call.expression),
        );
        for (const callee of callees) {
            const resolved = await this.resolve(callee, inspector);
            if (resolved !== undefined) {
                return resolved;
            }
        }

        return undefined;
    }

    /** Find the package containing a module directory. */
    locate(directory: string): Promise<PackageLocation> {
        let location = this.#locations.get(directory);
        if (location === undefined) {
            location = modulePackage(directory, this.#versions);
            this.#locations.set(directory, location);
        }

        return location;
    }

    /** Read a package's identity, with the stable id of its destack.json if declared. */
    owner(location: PackageLocation): Promise<Package | DependencyPackage> {
        let owner = this.#owners.get(location.directory);
        if (owner === undefined) {
            owner = readOwner(location);
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
        const directory = this.#packages.directory(name, location.directory);
        const entry = Object.values(this.#packages.constructors(name, location.directory))
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
    // resolve declaration constructors in one compiler request, and the factories module constants call
    const project = inspector.project;
    const calls = collectCalls(source, tests);
    const symbols = await project.checker.getSymbolAtLocation(calls.map((node) => node.expression));
    const constructors = await Promise.all(
        symbols.map(async (symbol, index) => {
            const resolved = await catalog.resolve(symbol, inspector);
            const isConstant = isModuleConstant(chainEnd(nth(calls, index)));

            return (
                resolved ??
                (isConstant ? await catalog.resolveFactory(symbol, inspector) : undefined)
            );
        }),
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
        // leave a call that constructs no declaration, and a factory's own calls, which its callers declare
        const resolved = constructors[index];
        if (!resolved || isInFunction(node)) {
            continue;
        }
        const located = await locateDeclaration(node, resolved, exported, declaring, inspector);
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

/** Find the last call chained on a call, such as `implement` on a declared type, or itself. */
function chainEnd(node: CallExpression): CallExpression {
    const member = node.parent;
    const chained = member.parent;

    return isPropertyAccessExpression(member) &&
        member.expression === node &&
        isCallExpression(chained) &&
        chained.expression === member
        ? chainEnd(chained)
        : node;
}

/** Find the call a chain of calls starts with, such as the one `implement` is chained on. */
function chainStart(node: CallExpression): CallExpression {
    const callee = node.expression;

    return isPropertyAccessExpression(callee) && isCallExpression(callee.expression)
        ? chainStart(callee.expression)
        : node;
}

/** Report whether a call initializes a module's constant or is its default export. */
function isModuleConstant(node: CallExpression): boolean {
    const declaration = node.parent;

    return (
        (isExportAssignment(declaration) && !declaration.isExportEquals) ||
        (isVariableDeclaration(declaration) &&
            declaration.parent.parent.parent.kind === SyntaxKind.SourceFile)
    );
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

        // visit the children and collect calls
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (isCallExpression(node)) {
            calls.push(node);
        }
    }

    return calls;
}

/** Report whether a call runs inside a function's body, as a declaration factory's own calls do. */
function isInFunction(node: Node): boolean {
    for (let at: Node | undefined = node.parent; at !== undefined; at = at.parent) {
        if (FUNCTION_KINDS.has(at.kind)) {
            return true;
        }
    }

    return false;
}

/** Read the exported name of each symbol a module exports, including lists and aliases. */
export async function exportedNames(
    source: SourceFile,
    inspector: SymbolInspector,
): Promise<Map<number, string>> {
    // key each export's original symbol
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

/** Require a constructor call to be the default export or initialize an exported constant. */
async function locateDeclaration(
    node: CallExpression,
    resolved: ResolvedConstructor,
    exported: ReadonlyMap<number, string>,
    declaring: Package,
    inspector: SymbolInspector,
): Promise<{ name: string; export: string }> {
    // take a module's default export, such as its package handle
    const { constructor } = resolved;
    const declaration = chainEnd(node).parent;
    if (isExportAssignment(declaration) && !declaration.isExportEquals) {
        return { name: DEFAULT_EXPORT, export: DEFAULT_EXPORT };
    }

    // require a named variable initialized by the constructor, or by methods chained on its result
    if (!isVariableDeclaration(declaration) || !isIdentifier(declaration.name)) {
        throw BuildError.of(
            "INSPECTION_FAILED",
            `${constructor.symbol.name} requires a named module declaration`,
        );
    }

    // require an exported constant to evaluate
    const symbol = await inspector.project.checker.getSymbolAtLocation(declaration.name);
    const exportedName = symbol === undefined ? undefined : exported.get(symbol.id);
    if (exportedName === undefined) {
        throw BuildError.of(
            "DECLARATION_UNEXPORTED",
            `declared ${declaration.name.text} is not exported by ${declaring.name}`,
        );
    }

    // require an unconditional module constant
    if (
        !(declaration.parent.flags & NodeFlags.Const) ||
        declaration.parent.parent.parent.kind !== SyntaxKind.SourceFile
    ) {
        throw BuildError.of(
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
