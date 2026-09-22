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

/** Inspect runtime globals while excluding local declarations that shadow them. */
export async function collectGlobals(
    source: SourceFile,
    project: Project,
    file: string,
    reference: (symbol: TypeScriptSymbol) => Promise<SymbolReference>,
): Promise<GlobalReference[]> {
    // collect identifiers used in runtime expressions
    const globals: GlobalReference[] = [];
    const identifiers: Identifier[] = [];
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        // visit the runtime base class without treating its generic arguments as expressions
        if (isExpressionWithTypeArguments(node)) {
            if (
                isHeritageClause(node.parent) &&
                node.parent.token === SyntaxKind.ExtendsKeyword &&
                (node.parent.parent.kind === SyntaxKind.ClassDeclaration ||
                    node.parent.parent.kind === SyntaxKind.ClassExpression)
            ) {
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
        if (
            !isIdentifier(node) ||
            (isPropertyAccessExpression(node.parent) && node.parent.name === node)
        ) {
            continue;
        }
        identifiers.push(node);
    }

    // resolve candidate identifiers in one compiler request per module
    const symbols = await project.checker.getSymbolAtLocation(identifiers);
    for (const [index, node] of identifiers.entries()) {
        const symbol = symbols[index];
        if (!symbol || !(symbol.flags & SymbolFlags.Value) || symbol.flags & SymbolFlags.Alias) {
            continue;
        }

        // distinguish ambient globals from package declarations
        const declarations = await Promise.all(
            symbol.declarations.map((entry) => entry.resolve(project)),
        );
        if (
            ((declarations.length &&
                declarations.every((entry) => entry?.getSourceFile().isDeclarationFile)) ||
                node.text === "globalThis" ||
                node.text === "undefined") &&
            !(await symbol.getParent())
        ) {
            // retain property access without evaluating computed application expressions
            const members: string[] = [];
            let dynamic = false;
            let expression: Node = node;
            while (true) {
                const parent = expression.parent;
                if (isPropertyAccessExpression(parent) && parent.expression === expression) {
                    members.push(parent.name.text);
                } else if (isElementAccessExpression(parent) && parent.expression === expression) {
                    const argument = parent.argumentExpression;
                    if (isStringLiteral(argument) || isNumericLiteral(argument)) {
                        members.push(argument.text);
                    } else {
                        dynamic = true;
                    }
                } else {
                    break;
                }
                expression = parent;
            }

            // associate the access with its source location
            globals.push({
                name: node.text,
                symbol: declarations.length ? await reference(symbol) : undefined,
                members,
                dynamic,
                source: { file, start: node.getStart(), end: node.getEnd() },
            });
        }
    }

    return globals;
}
