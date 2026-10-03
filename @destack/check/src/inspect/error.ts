import {
    SyntaxKind,
    type CallExpression,
    type NewExpression,
    type Node,
    type SourceFile,
    type ThrowStatement,
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
import { found } from "@destack/schema";
import { CheckError } from "../error/index.ts";

/** Callable declarations with independently executed bodies. */
const FUNCTION_KINDS = new Set([
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

/** A callable body's root and the nodes it runs, without nested callable bodies. */
interface Body {
    /** The source file or callable declaration. */
    readonly root: Node;
    /** The nodes the body runs, the root first. */
    readonly nodes: readonly Node[];
}

/** A call's resolved target, absent for an unresolved or declaration-less callee. */
interface Call {
    /** The target declaration. */
    readonly target: SymbolReference | undefined;
}

/** Collect error relationships without executing source or treating unknown calls as safe. */
export async function inspectErrors(
    source: SourceFile,
    inspector: ErrorInspector,
): Promise<ErrorDescription[]> {
    if (source.isDeclarationFile) {
        return [];
    }

    // resolve every throw's type and every call's target in one compiler request each
    const bodies = bodiesOf(source);
    const nodes = bodies.flatMap((body) => body.nodes);
    const calls = nodes.filter((node) => isCallExpression(node) || isNewExpression(node));
    const [thrown, called] = await Promise.all([
        throwTypes(nodes.filter(isThrowStatement), inspector),
        callTargets(calls, inspector),
    ]);

    // describe each body's throws, calls and catches
    const descriptions = bodies.map((body) => describeBody(body, thrown, called, inspector));

    return descriptions.toSorted((left, right) => left.source.start - right.source.start);
}

/** Separate a source file's callable bodies, each without the bodies nested in it. */
function bodiesOf(source: SourceFile): Body[] {
    // take each pending root, the source file first
    const bodies: Body[] = [];
    const pending: Node[] = [source];
    for (let root = pending.pop(); root !== undefined; root = pending.pop()) {
        // collect the root's nodes, deferring nested callable bodies
        const nodes: Node[] = [];
        const visit = (node: Node): void => {
            if (node !== root && FUNCTION_KINDS.has(node.kind)) {
                pending.push(node);
            } else {
                nodes.push(node);
                node.forEachChild(visit);
            }
        };
        visit(root);
        bodies.push({ root, nodes });
    }

    return bodies;
}

/** Describe the type each throw's expression has, in one compiler request. */
async function throwTypes(
    throws: readonly ThrowStatement[],
    inspector: ErrorInspector,
): Promise<Map<Node, TypeDescription>> {
    // resolve the types, failing for an expression without one
    const expressions = throws.map((node) => node.expression);
    const types =
        expressions.length === 0
            ? []
            : await inspector.project.checker.getTypeAtLocation(expressions);
    const described = await Promise.all(
        throws.map(async (node, index) => {
            const type = types[index];
            if (!type) {
                throw new CheckError(
                    "language",
                    "compiler returned no type for a throw expression",
                );
            }

            return [node, await inspector.type(type, node.expression)] as const;
        }),
    );

    return new Map(described);
}

/** Locate each call's target declaration, in one compiler request. */
async function callTargets(
    calls: readonly (CallExpression | NewExpression)[],
    inspector: ErrorInspector,
): Promise<Map<Node, Call>> {
    // resolve the callee symbols, then each one's declaration
    const expressions = calls.map((node) => node.expression);
    const symbols =
        expressions.length === 0
            ? []
            : await inspector.project.checker.getSymbolAtLocation(expressions);
    const located = await Promise.all(
        calls.map(
            async (node, index) =>
                [node, { target: await callTarget(symbols[index], inspector) }] as const,
        ),
    );

    return new Map(located);
}

/** Locate a call's target declaration through aliases, absent for an unresolved or declaration-less callee. */
async function callTarget(
    symbol: Symbol | undefined,
    inspector: ErrorInspector,
): Promise<SymbolReference | undefined> {
    // follow an alias to the symbol it names
    const isAlias = symbol !== undefined && (symbol.flags & SymbolFlags.Alias) !== 0;
    const target = isAlias ? await inspector.project.checker.getAliasedSymbol(symbol) : symbol;
    if (target === undefined || target.declarations.length === 0) {
        return undefined;
    }

    return inspector.reference(target);
}

/** Describe one callable body's throws, calls, catches and the nodes that may run user code. */
function describeBody(
    body: Body,
    thrown: ReadonlyMap<Node, TypeDescription>,
    called: ReadonlyMap<Node, Call>,
    inspector: ErrorInspector,
): ErrorDescription {
    // describe each node the body runs
    const { root, nodes } = body;
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
            description.throws.push({
                source: inspector.range(node),
                type: found(thrown, node),
                catches: enclosingCatches(node, root, inspector),
            });
        }
        // retain call targets while marking callee behavior as unresolved
        else if (isCallExpression(node) || isNewExpression(node)) {
            const { target } = found(called, node);
            description.calls.push({
                source: inspector.range(node),
                ...(target === undefined ? {} : { target }),
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

    return description;
}

/** Find catches covering this expression, excluding catches attached to catch/finally bodies. */
function enclosingCatches(node: Node, root: Node, inspector: ErrorInspector): SourceRange[] {
    // walk up to the root, collecting try blocks that contain the node
    const catches: SourceRange[] = [];
    let child = node;
    while (child !== root) {
        const parent = child.parent;
        if (isTryStatement(parent) && parent.tryBlock === child && parent.catchClause) {
            catches.push(inspector.range(parent.catchClause));
        }
        child = parent;
    }

    return catches;
}
