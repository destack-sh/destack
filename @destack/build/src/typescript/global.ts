import {
    type Project,
    type Symbol as TypeScriptSymbol,
    SymbolFlags,
} from "typescript/unstable/async";
import {
    type Identifier,
    isCallExpression,
    isElementAccessExpression,
    isExpressionWithTypeArguments,
    isHeritageClause,
    isIdentifier,
    isNumericLiteral,
    isPropertyAccessExpression,
    isPropertyDeclaration,
    isStringLiteral,
    isTypeNode,
    type Node,
    type PropertyDeclaration,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";
import type { GlobalReference, SymbolReference } from "@destack/package/code";
import { isDeclarationPath } from "./module.ts";

/** The symbols a runtime declares as globals: variables, functions, classes, enums, namespaces. */
const GLOBAL_SYMBOL =
    SymbolFlags.Variable |
    SymbolFlags.Function |
    SymbolFlags.Class |
    SymbolFlags.Enum |
    SymbolFlags.ValueModule;

/** The functions whose bodies and parameters run once called, not as the module evaluates. */
const DEFERRING_FUNCTIONS: ReadonlySet<SyntaxKind> = new Set([
    SyntaxKind.FunctionDeclaration,
    SyntaxKind.FunctionExpression,
    SyntaxKind.ArrowFunction,
    SyntaxKind.MethodDeclaration,
    SyntaxKind.GetAccessor,
    SyntaxKind.SetAccessor,
    SyntaxKind.Constructor,
]);

/** Report whether a node is a function whose body and parameters run once called. */
export function isDeferringFunction(node: Node): boolean {
    return DEFERRING_FUNCTIONS.has(node.kind);
}

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
        // take globals the runtime declares, leaving properties, imports and package declarations
        const symbol = symbols[index];
        if (!symbol || !(symbol.flags & GLOBAL_SYMBOL) || symbol.flags & SymbolFlags.Alias) {
            continue;
        }
        if (!(await isAmbient(symbol, node))) {
            continue;
        }

        // retain each property access with its source location
        const declarations = symbol.declarations;
        const { members, isDynamic } = accessPath(node);
        globals.push({
            name: node.text,
            ...(declarations.length ? { symbol: await reference(symbol) } : {}),
            members,
            dynamic: isDynamic,
            evaluation: evaluation(node),
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

/** Report whether an identifier is the member of a property access rather than a value. */
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

/** Report whether an identifier refers to an ambient global rather than a package declaration. */
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
        // extend the path by a property name
        if (isPropertyAccessExpression(parent) && parent.expression === expression) {
            members.push(parent.name.text);
        }
        // extend it by a literal key, ending at a well-known symbol key
        else if (isElementAccessExpression(parent) && parent.expression === expression) {
            const argument = parent.argumentExpression;
            if (isStringLiteral(argument) || isNumericLiteral(argument)) {
                members.push(argument.text);
            } else if (isWellKnownSymbol(argument)) {
                break;
            } else {
                isDynamic = true;
                break;
            }
        } else {
            break;
        }
        expression = parent;
    }

    return { members, isDynamic };
}

/** Find when an identifier runs: as its module evaluates, or once a function around it runs. */
function evaluation(node: Node): GlobalReference["evaluation"] {
    // walk outward to the first function or instance field that defers the expression
    for (
        let child = node, parent = node.parent;
        parent !== undefined;
        child = parent, parent = parent.parent
    ) {
        // defer a function's body and parameters, unless the module calls the function right away
        if (DEFERRING_FUNCTIONS.has(parent.kind) && !isCalledAtOnce(parent)) {
            return "function";
        }
        // defer an instance field's initializer to construction
        else if (
            isPropertyDeclaration(parent) &&
            parent.initializer === child &&
            !isStatic(parent)
        ) {
            return "function";
        }
    }

    return "module";
}

/** Report whether a function expression is called where it is written, as an IIFE is. */
function isCalledAtOnce(node: Node): boolean {
    // step out of the parentheses around the function
    let expression = node;
    while (expression.parent.kind === SyntaxKind.ParenthesizedExpression) {
        expression = expression.parent;
    }
    const call = expression.parent;
    const isExpression =
        node.kind === SyntaxKind.FunctionExpression || node.kind === SyntaxKind.ArrowFunction;

    return isExpression && isCallExpression(call) && call.expression === expression;
}

/** Report whether a class field is static. */
function isStatic(node: PropertyDeclaration): boolean {
    return node.modifiers?.some((modifier) => modifier.kind === SyntaxKind.StaticKeyword) ?? false;
}

/** Report whether a key is a member of the global Symbol, such as Symbol.iterator. */
function isWellKnownSymbol(node: Node): boolean {
    return (
        isPropertyAccessExpression(node) &&
        isIdentifier(node.expression) &&
        node.expression.text === "Symbol"
    );
}
