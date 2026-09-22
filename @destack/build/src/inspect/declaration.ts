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
import type { DeclarationDescription, BuildDescription } from "@destack/package/inspect";
import { Package } from "@destack/package";
import { BuildError } from "../error/index.ts";
import { type InspectorName, INSPECTORS } from "./inspector.ts";
import { modulePackage } from "../source/dependency.ts";
import type { TestDeclaration } from "@destack/test/inspect";

/** An exported declaration located by the compiler. */
export interface Declaration {
    /** The declaration constructor verified by its compiler symbol. */
    inspector: InspectorName;
    /** The absolute source module path. */
    file: string;
    /** The exported constant name. */
    export: string;
    /** The symbol and source location. */
    description: Omit<DeclarationDescription, "description">;
}

/** Locate exported declarations without evaluating package modules. */
export async function collectDeclarations(
    source: SourceFile,
    file: string,
    project: Project,
    sourcePackage: Awaited<ReturnType<typeof modulePackage>>,
    tests: readonly TestDeclaration[] = [],
): Promise<Declaration[]> {
    // collect calls before resolving constructor symbols in one request
    const declarations: Declaration[] = [];
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
    const inspectors = await Promise.all(
        symbols.map((symbol) => resolveInspector(symbol, project)),
    );
    if (inspectors.every((inspector) => inspector === undefined)) {
        return declarations;
    }

    // resolve stable identity only for packages exporting Destack declarations
    const definition = JSON.parse(
        await readFile(join(sourcePackage.directory, "destack.json"), "utf8"),
    );
    const owner = Package.parse({
        id: definition.id,
        name: sourcePackage.name,
        version: sourcePackage.version,
    });

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
        const inspector = inspectors[index];
        if (!inspector) {
            continue;
        }
        const { name, constructor } = inspector;

        // require a named variable initialized by the constructor
        const definition = INSPECTORS[name];
        const declaration = node.parent;
        if (
            !isVariableDeclaration(declaration) ||
            !declaration.name ||
            !isIdentifier(declaration.name)
        ) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${definition.kind} requires a named module declaration.`,
            );
        }

        // require an exported reference to evaluate
        const symbol = await project.checker.getSymbolAtLocation(declaration.name);
        const exportedName = symbol && exported.get(symbol.id);
        if (!exportedName) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `Export ${file}:${declaration.name.text} to inspect its declaration.`,
            );
        }

        // require an unconditional module constant
        if (
            !(declaration.parent.flags & NodeFlags.Const) ||
            declaration.parent.parent.parent.kind !== SyntaxKind.SourceFile
        ) {
            throw new BuildError(
                "INSPECTION_FAILED",
                `${definition.kind} must initialize a module-level constant.`,
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
            inspector: name,
            file: source.fileName,
            export: exportedName,
            description: {
                kind: definition.kind,
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

/** Verify a constructor's name and defining package through its compiler symbol. */
async function resolveInspector(
    symbol: TypeScriptSymbol | undefined,
    project: Project,
): Promise<
    { name: InspectorName; constructor: DeclarationDescription["constructor"] } | undefined
> {
    // follow imported constructor aliases
    if (symbol && symbol.flags & SymbolFlags.Alias) {
        symbol = await project.checker.getAliasedSymbol(symbol);
    }
    if (!symbol || !Object.hasOwn(INSPECTORS, symbol.name)) {
        return undefined;
    }

    // require a constructor declared by the expected package
    const name = symbol.name as InspectorName;
    const declaration = await symbol.declarations[0]?.resolve(project);
    if (!declaration || !isFunctionDeclaration(declaration)) {
        return undefined;
    }
    const owner = await modulePackage(dirname(declaration.getSourceFile().fileName));

    if (owner.name !== INSPECTORS[name].package) {
        return undefined;
    }

    // identify the domain package and constructor without embedding its format schema
    const definition = JSON.parse(await readFile(join(owner.directory, "destack.json"), "utf8"));
    const packageIdentity = Package.parse({
        id: definition.id,
        name: owner.name,
        version: owner.version,
    });
    const module = relative(owner.directory, declaration.getSourceFile().fileName).replaceAll(
        "\\",
        "/",
    );

    return { name, constructor: { package: packageIdentity, symbol: { module, name } } };
}

/** Select package declarations and dependency declarations parsed by this compilation. */
export function selectDeclarations(
    declarations: readonly DeclarationDescription[],
    source: Package,
    build: BuildDescription,
): DeclarationDescription[] {
    // index dependency source files, including declarations removed during tree shaking
    const paths = new Map<string, Set<string>>();
    for (const input of Object.values(build.inputs)) {
        if (!input.package) {
            continue;
        }
        const files = paths.get(input.package) ?? new Set<string>();
        files.add(input.path.split("?")[0]);
        paths.set(input.package, files);
    }

    // retain authored declarations and only the dependencies used by this output
    return declarations.filter((declaration) => {
        const owner = declaration.symbol.package;
        if (owner.id === source.id && owner.version === source.version) {
            return true;
        }

        return paths.get(`${owner.name}@${owner.version}`)?.has(declaration.source.file) === true;
    });
}
