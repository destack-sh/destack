import { type Project, SymbolFlags } from "typescript/unstable/async";
import {
    isIdentifier,
    isMetaProperty,
    isNewExpression,
    isPropertyAccessExpression,
    isStringLiteral,
    type Node,
    type SourceFile,
    type StringLiteral,
    SyntaxKind,
} from "typescript/unstable/ast";
import { isDeclarationPath } from "./module.ts";

/** A literal directory URL in an authored module. */
export interface DirectoryReference {
    /** The directory path relative to the module. */
    path: string;
    /** The first character of the URL construction. */
    start: number;
    /** The character after the URL construction. */
    end: number;
}

/** Collect directory URLs resolved to the runtime's global URL constructor. */
export async function collectDirectories(
    source: SourceFile,
    project: Project,
): Promise<DirectoryReference[]> {
    // visit constructor calls throughout the module
    const references: DirectoryReference[] = [];
    const pending: Node[] = [...source.statements];
    for (const node of pending) {
        node.forEachChild((child) => {
            pending.push(child);
        });
        if (
            !isNewExpression(node) ||
            !isIdentifier(node.expression) ||
            node.expression.text !== "URL"
        ) {
            continue;
        }

        // require a literal relative directory based on import.meta.url
        const [path, base] = node.arguments ?? [];
        if (node.arguments?.length !== 2 || !isDirectoryPath(path) || !isModuleUrl(base)) {
            continue;
        }

        // exclude imported constructors and lexical declarations that shadow the global
        const symbol = await project.checker.getSymbolAtLocation(node.expression);
        if (!symbol || symbol.flags & SymbolFlags.Alias || (await symbol.getParent())) {
            continue;
        }
        if (
            !symbol.declarations.length ||
            !symbol.declarations.every((entry) => isDeclarationPath(entry.path))
        ) {
            continue;
        }

        references.push({ path: path.text, start: node.getStart(), end: node.getEnd() });
    }

    return references;
}

/** Report whether a node is a literal relative directory path, such as `./migration/`. */
function isDirectoryPath(node: Node | undefined): node is StringLiteral {
    return (
        node !== undefined &&
        isStringLiteral(node) &&
        /^\.{1,2}\//u.test(node.text) &&
        node.text.endsWith("/")
    );
}

/** Report whether a node is the expression `import.meta.url`. */
function isModuleUrl(node: Node | undefined): boolean {
    return (
        node !== undefined &&
        isPropertyAccessExpression(node) &&
        node.name.text === "url" &&
        isMetaProperty(node.expression) &&
        node.expression.keywordToken === SyntaxKind.ImportKeyword &&
        node.expression.name.text === "meta"
    );
}
