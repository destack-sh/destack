import type { ESTree, Rule } from "@oxlint/plugins";
import { DECLARATION_CONSTRUCTORS } from "@destack/package/declare";
import { isPackageFile } from "./package.ts";
import { isTestFile } from "./word.ts";

/** Constructors whose results build inspection collects from package exports. */
const CONSTRUCTORS = new Set(
    Object.entries(DECLARATION_CONSTRUCTORS)
        .filter(([, constructor]) => "kind" in constructor)
        .map(([name]) => name),
);

/** Require declarations to be exported module constants that inspection can find. */
export const validDeclaration: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "declare as an exported module constant" },
        messages: { export: "[PK05] assign '{{name}}(...)' to an exported module-level const" },
    },
    create(context) {
        return {
            CallExpression(node) {
                // skip calls that are not declaration constructors and test modules
                const name = node.callee.type === "Identifier" ? node.callee.name : undefined;
                if (
                    !name ||
                    !CONSTRUCTORS.has(name) ||
                    isTestFile(context.filename) ||
                    !isPackageFile(context.filename)
                ) {
                    return;
                }

                // require export const name = constructor(...) at module level
                if (!isExportedConstant(node)) {
                    context.report({ node, messageId: "export", data: { name } });
                }
            },
        };
    },
};

/** Report whether a call initializes an exported module-level const. */
function isExportedConstant(node: ESTree.Node): boolean {
    const declarator = node.parent;
    const declaration = declarator?.parent;

    return (
        declarator?.type === "VariableDeclarator" &&
        declarator.init === node &&
        declaration?.type === "VariableDeclaration" &&
        declaration.kind === "const" &&
        declaration.parent?.type === "ExportNamedDeclaration" &&
        declaration.parent.parent?.type === "Program"
    );
}
