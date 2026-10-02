import type { ESTree, Rule } from "@oxlint/plugins";

/** Module statements that declare a documented name. */
const DECLARATIONS = new Set([
    "FunctionDeclaration",
    "ClassDeclaration",
    "VariableDeclaration",
    "TSInterfaceDeclaration",
    "TSTypeAliasDeclaration",
    "TSEnumDeclaration",
    "TSModuleDeclaration",
]);

/** The files whose default export is a tool's configuration, such as vitest.config.ts. */
const CONFIGURATION = /\.config\.[cm]?[jt]sx?$/u;

/** Require a documentation comment on declarations, members and fields. */
export const requireJsdoc: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "document every declaration, member and field" },
        messages: { missing: "[WD02] add a documentation comment" },
    },
    create(context) {
        /** Report a node without a documentation comment on the line directly above it. */
        function check(node: ESTree.Node) {
            // take the comment directly above the declaration
            const comment = context.sourceCode.getCommentsBefore(node).at(-1);
            const isDocumented =
                comment?.type === "Block" &&
                comment.value.startsWith("*") &&
                comment.loc.end.line >= node.loc.start.line - 1;
            if (!isDocumented) {
                context.report({ node, messageId: "missing" });
            }
        }

        return {
            Program(node) {
                // check module declarations, including exported ones
                for (const statement of node.body) {
                    const declaration =
                        statement.type === "ExportNamedDeclaration"
                            ? statement.declaration
                            : statement;
                    if (declaration && DECLARATIONS.has(declaration.type)) {
                        check(statement);
                    }
                    // check default exports, except a tool's configuration
                    else if (
                        statement.type === "ExportDefaultDeclaration" &&
                        statement.declaration.type !== "Identifier" &&
                        !CONFIGURATION.test(context.filename)
                    ) {
                        check(statement);
                    }
                }
            },
            MethodDefinition(node) {
                // document overloads once, on the first signature
                const members = node.parent?.type === "ClassBody" ? node.parent.body : [];
                const previous = members[members.indexOf(node) - 1];
                const isOverload =
                    previous?.type === "TSAbstractMethodDefinition" ||
                    (previous !== undefined &&
                        "key" in previous &&
                        memberName(previous.key) !== undefined &&
                        memberName(previous.key) === memberName(node.key));
                if (!isOverload) {
                    check(node);
                }
            },
            PropertyDefinition: check,
            TSPropertySignature(node) {
                if (node.parent?.type === "TSInterfaceBody") {
                    check(node);
                }
            },
            TSMethodSignature(node) {
                if (node.parent?.type === "TSInterfaceBody") {
                    check(node);
                }
            },
            TSEnumMember: check,
        };
    },
};

/** Read a class member's plain or private identifier, absent for computed keys. */
function memberName(key: ESTree.MethodDefinition["key"]): string | undefined {
    return key.type === "Identifier" || key.type === "PrivateIdentifier"
        ? `${key.type}:${key.name}`
        : undefined;
}
