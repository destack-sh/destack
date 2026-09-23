import { type Project, SymbolFlags } from "typescript/unstable/async";
import {
    isIdentifier,
    isMetaProperty,
    isNewExpression,
    isPropertyAccessExpression,
    isStringLiteral,
    type Node,
    type SourceFile,
    SyntaxKind,
} from "typescript/unstable/ast";

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
        if (
            node.arguments?.length !== 2 ||
            !path ||
            !isStringLiteral(path) ||
            !/^\.{1,2}\//.test(path.text) ||
            !path.text.endsWith("/") ||
            !base ||
            !isPropertyAccessExpression(base) ||
            base.name.text !== "url" ||
            !isMetaProperty(base.expression) ||
            base.expression.keywordToken !== SyntaxKind.ImportKeyword ||
            base.expression.name.text !== "meta"
        ) {
            continue;
        }

        // exclude imported constructors and lexical declarations that shadow the global
        const symbol = await project.checker.getSymbolAtLocation(node.expression);
        if (!symbol || symbol.flags & SymbolFlags.Alias || (await symbol.getParent())) {
            continue;
        }
        const declarations = await Promise.all(
            symbol.declarations.map((entry) => entry.resolve(project)),
        );
        if (
            !declarations.length ||
            !declarations.every((entry) => entry?.getSourceFile().isDeclarationFile)
        ) {
            continue;
        }

        references.push({ path: path.text, start: node.getStart(), end: node.getEnd() });
    }

    return references;
}
