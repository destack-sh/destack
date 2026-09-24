import type { ESTree } from "@oxlint/plugins";

/** Split an identifier into lowercase words at case changes, digits, underscores and hyphens. */
export function splitWords(identifier: string): string[] {
    return identifier
        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
        .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
        .split(/[\s_\-$0-9]+/)
        .filter(Boolean)
        .map((word) => word.toLowerCase());
}

/** Report whether an identifier node declares a name rather than referencing one. */
export function isDeclaredName(node: ESTree.Node): boolean {
    const parent = node.parent as (ESTree.Node & Record<string, unknown>) | undefined;
    if (!parent) {
        return false;
    }

    // match the name positions of declarations, members and parameters

    return (
        ((parent.type === "VariableDeclarator" ||
            parent.type === "FunctionDeclaration" ||
            parent.type === "ClassDeclaration" ||
            parent.type === "TSInterfaceDeclaration" ||
            parent.type === "TSTypeAliasDeclaration" ||
            parent.type === "TSEnumDeclaration") &&
            parent.id === node) ||
        ((parent.type === "PropertyDefinition" ||
            parent.type === "MethodDefinition" ||
            parent.type === "TSPropertySignature" ||
            parent.type === "TSMethodSignature") &&
            parent.key === node) ||
        ((parent.type === "FunctionDeclaration" ||
            parent.type === "FunctionExpression" ||
            parent.type === "ArrowFunctionExpression") &&
            (parent.params as ESTree.Node[]).includes(node))
    );
}

/** Report whether a path names a test module. */
export function isTestFile(path: string): boolean {
    return /\.test\.[cm]?[jt]sx?$/.test(path) || /[\\/]tests[\\/]/.test(path);
}
