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
            // skip lint directives between the documentation and its subject
            const comments = context.sourceCode.getCommentsBefore(node);
            const comment = comments.findLast((entry) => !isDirective(entry.value));
            const isDocumented =
                comment?.type === "Block" &&
                comment.value.startsWith("*") &&
                comment.loc.end.line >= node.loc.start.line - 1 - directiveLines(comments, comment);
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
                    } else if (
                        statement.type === "ExportDefaultDeclaration" &&
                        statement.declaration.type !== "Identifier"
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
                        previous.key.type === "Identifier" &&
                        node.key.type === "Identifier" &&
                        previous.key.name === node.key.name);
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

/** Report whether a comment is a lint directive. */
function isDirective(text: string): boolean {
    return /^\s*(?:oxlint|eslint)-/.test(text);
}

/** Count the directive lines between a documentation comment and its subject. */
function directiveLines(comments: readonly { value: string }[], documentation: object): number {
    return comments
        .slice(comments.indexOf(documentation as never) + 1)
        .filter((entry) => isDirective(entry.value)).length;
}
