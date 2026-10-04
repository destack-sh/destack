import {
    type Project,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
} from "typescript/unstable/async";
import {
    type Identifier,
    isElementAccessExpression,
    isExpressionWithTypeArguments,
    isHeritageClause,
    isIdentifier,
    isNumericLiteral,
    isPropertyAccessExpression,
    isStringLiteral,
    isTypeNode,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type { GlobalReference, SymbolReference } from "@destack/package/code";
import { isDeclarationPath } from "./module.ts";

/** The symbols a runtime declares as globals: variables, functions, classes, enums and namespaces, never properties. */
const GLOBAL_SYMBOL =
    SymbolFlags.Variable |
    SymbolFlags.Function |
    SymbolFlags.Class |
    SymbolFlags.Enum |
    SymbolFlags.ValueModule;

/** Inspect runtime globals while excluding local declarations that shadow them. */
export async function collectGlobals(
    source: SourceFile,
    project: Project,
    file: string,
    reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>,
): Promise<GlobalReference[]> {
    // resolve candidate identifiers in one compiler request per module
    const identifiers = runtimeIdentifiers(source);
    const symbols = await project.checker.getSymbolAtLocation(identifiers);
    const globals: GlobalReference[] = [];
    for (const [index, node] of identifiers.entries()) {
        // keep globals the runtime declares, leaving properties, imports and package declarations
        const symbol = symbols[index];
        if (!symbol || !(symbol.flags & GLOBAL_SYMBOL) || symbol.flags & SymbolFlags.Alias) {
            continue;
        }
        if (!(await isAmbient(symbol, node))) {
            continue;
        }

        // retain property access with its source location, without evaluating computed application expressions
        const declarations = symbol.declarations;
        const { members, isDynamic } = accessPath(node);
        globals.push({
            name: node.text,
            ...(declarations.length ? { symbol: await reference(symbol) } : {}),
            members,
            dynamic: isDynamic,
            source: { file, start: node.getStart(), end: node.getEnd() },
        });
    }

    return globals;
}

/** Collect the root identifiers of a module's runtime expressions. */
function runtimeIdentifiers(source: SourceFile): Identifier[] {
    // walk the module breadth first
    const identifiers: Identifier[] = [];
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        // visit the runtime base class without treating its generic arguments as expressions
        if (isExpressionWithTypeArguments(node)) {
            if (isClassExtension(node)) {
                pending.push(node.expression);
            }
            continue;
        }

        // skip declarations that have no runtime expression
        if (
            isTypeNode(node) ||
            node.kind === SyntaxKind.InterfaceDeclaration ||
            node.kind === SyntaxKind.TypeAliasDeclaration ||
            node.kind === SyntaxKind.ImportDeclaration ||
            node.kind === SyntaxKind.ExportDeclaration
        ) {
            continue;
        }

        // traverse expressions and retain root identifiers
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (isIdentifier(node) && !isMemberName(node)) {
            identifiers.push(node);
        }
    }

    return identifiers;
}

/** Report whether an identifier names the member of a property access rather than a value. */
function isMemberName(node: Identifier): boolean {
    const parent = node.parent;

    return isPropertyAccessExpression(parent) && parent.name === node;
}

/** Report whether a node is the base class of a class declaration or expression. */
function isClassExtension(node: Node): boolean {
    const clause = node.parent;

    return (
        isHeritageClause(clause) &&
        clause.token === SyntaxKind.ExtendsKeyword &&
        (clause.parent.kind === SyntaxKind.ClassDeclaration ||
            clause.parent.kind === SyntaxKind.ClassExpression)
    );
}

/** Report whether an identifier names an ambient global rather than a package declaration. */
async function isAmbient(symbol: TypeScriptSymbol, node: Identifier): Promise<boolean> {
    // accept declaration files and the built-in globals outside any namespace
    const declarations = symbol.declarations;
    const isDeclared =
        declarations.length > 0 && declarations.every((entry) => isDeclarationPath(entry.path));
    const isBuiltin = node.text === "globalThis" || node.text === "undefined";

    return (isDeclared || isBuiltin) && !(await symbol.getParent());
}

/** Read the literal members accessed on an identifier, marking computed access dynamic. */
function accessPath(node: Identifier): { members: string[]; isDynamic: boolean } {
    // walk outward through the accesses
    const members: string[] = [];
    let isDynamic = false;
    let expression: Node = node;
    while (true) {
        // follow a property access, or an element access by a literal key
        const parent = expression.parent;
        if (isPropertyAccessExpression(parent) && parent.expression === expression) {
            members.push(parent.name.text);
        } else if (isElementAccessExpression(parent) && parent.expression === expression) {
            const argument = parent.argumentExpression;
            if (isStringLiteral(argument) || isNumericLiteral(argument)) {
                members.push(argument.text);
            } else {
                isDynamic = true;
            }
        } else {
            break;
        }
        expression = parent;
    }

    return { members, isDynamic };
}
