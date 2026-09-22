import {
    SyntaxKind,
    type Node,
    type SourceFile,
    isThrowStatement,
    isCallExpression,
    isNewExpression,
    isTryStatement,
    isPropertyAccessExpression,
    isElementAccessExpression,
} from "typescript/unstable/ast";
import { SymbolFlags, type Project, type Type, type Symbol } from "typescript/unstable/async";
import type { ErrorDescription, TypeDescription, SymbolReference } from "@destack/package/code";
import type { SourceRange } from "@destack/package/source";
import { CheckError } from "../error/index.ts";

/** Callable declarations with independently executed bodies. */
const functionKinds = new Set([
    SyntaxKind.FunctionDeclaration,
    SyntaxKind.FunctionExpression,
    SyntaxKind.ArrowFunction,
    SyntaxKind.MethodDeclaration,
    SyntaxKind.Constructor,
    SyntaxKind.GetAccessor,
    SyntaxKind.SetAccessor,
]);

/** Compiler operations shared with package code inspection. */
export interface ErrorInspector {
    /** The existing compiler snapshot. */
    project: Project;
    /** Resolve a compiler symbol into its exact package. */
    reference(symbol: Symbol): Promise<SymbolReference>;
    /** Describe a compiler type. */
    type(type: Type, location: Node): Promise<TypeDescription>;
    /** Locate an expression in its source package. */
    range(node: Node): SourceRange;
}

/** Collect error relationships without executing source or treating unknown calls as safe. */
export async function inspectErrors(
    source: SourceFile,
    inspector: ErrorInspector,
): Promise<ErrorDescription[]> {
    if (source.isDeclarationFile) {
        return [];
    }

    // separate nested callable bodies from the enclosing function
    const groups: { root: Node; nodes: Node[] }[] = [];
    const pending: Node[] = [source];
    while (pending.length) {
        const root = pending.pop()!;
        const nodes: Node[] = [];
        const visit = (node: Node): void => {
            if (node !== root && functionKinds.has(node.kind)) {
                pending.push(node);
            } else {
                nodes.push(node);
                node.forEachChild(visit);
            }
        };
        visit(root);
        groups.push({ root, nodes });
    }

    const descriptions: ErrorDescription[] = [];
    for (const { root, nodes } of groups) {
        const description: ErrorDescription = {
            source: inspector.range(root),
            throws: [],
            calls: [],
            catches: [],
            finally: [],
            unknowns: [],
        };
        for (const node of nodes) {
            // retain the actual type and handlers for each explicit throw
            if (isThrowStatement(node)) {
                const type = await inspector.project.checker.getTypeAtLocation(node.expression);
                if (!type) {
                    throw new CheckError(
                        "language",
                        "compiler returned no type for a throw expression",
                    );
                }
                description.throws.push({
                    source: inspector.range(node),
                    type: await inspector.type(type, node.expression),
                    catches: enclosingCatches(node, root, inspector),
                });
            }
            // retain call targets while marking callee behavior as unresolved
            else if (isCallExpression(node) || isNewExpression(node)) {
                let symbol = await inspector.project.checker.getSymbolAtLocation(node.expression);
                if (symbol && symbol.flags & SymbolFlags.Alias) {
                    symbol = await inspector.project.checker.getAliasedSymbol(symbol);
                }
                description.calls.push({
                    source: inspector.range(node),
                    target: symbol ? await inspector.reference(symbol) : undefined,
                    isAwaited: node.parent?.kind === SyntaxKind.AwaitExpression,
                    catches: enclosingCatches(node, root, inspector),
                });
                description.unknowns.push({ source: inspector.range(node), reason: "call" });
            }
            // preserve handler and finally locations for source navigation
            else if (isTryStatement(node)) {
                if (node.catchClause) {
                    description.catches.push(inspector.range(node.catchClause));
                }
                if (node.finallyBlock) {
                    description.finally.push(inspector.range(node.finallyBlock));
                }
            }
            // getters, proxies, awaiting, and iteration can execute user code
            else if (isPropertyAccessExpression(node) || isElementAccessExpression(node)) {
                description.unknowns.push({ source: inspector.range(node), reason: "property" });
            } else if (node.kind === SyntaxKind.AwaitExpression) {
                description.unknowns.push({ source: inspector.range(node), reason: "await" });
            } else if (
                node.kind === SyntaxKind.ForOfStatement ||
                node.kind === SyntaxKind.SpreadElement
            ) {
                description.unknowns.push({ source: inspector.range(node), reason: "iteration" });
            }
            // implicit conversions can invoke user-defined methods
            else if (
                node.kind === SyntaxKind.BinaryExpression ||
                node.kind === SyntaxKind.PrefixUnaryExpression ||
                node.kind === SyntaxKind.PostfixUnaryExpression ||
                node.kind === SyntaxKind.TemplateExpression
            ) {
                description.unknowns.push({ source: inspector.range(node), reason: "implicit" });
            }
        }
        descriptions.push(description);
    }

    return descriptions.sort((left, right) => left.source.start - right.source.start);
}

/** Find catches covering this expression, excluding catches attached to catch/finally bodies. */
function enclosingCatches(node: Node, root: Node, inspector: ErrorInspector): SourceRange[] {
    const catches: SourceRange[] = [];
    let child = node;
    while (child !== root && child.parent) {
        const parent = child.parent;
        if (isTryStatement(parent) && parent.tryBlock === child && parent.catchClause) {
            catches.push(inspector.range(parent.catchClause));
        }
        child = parent;
    }

    return catches;
}
