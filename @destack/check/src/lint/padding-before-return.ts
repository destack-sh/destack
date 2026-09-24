import type { Rule } from "@oxlint/plugins";

/** Require a blank line before the final return of a function with several statements. */
export const paddingBeforeReturn: Rule = {
    meta: {
        type: "suggestion",
        fixable: "whitespace",
        schema: [],
        docs: { description: "separate the return value with a blank line" },
        messages: { blank: "[WL11] add a blank line before the return" },
    },
    create(context) {
        return {
            ReturnStatement(node) {
                // check returns directly in function bodies with several statements
                const body = node.parent;
                const isFunctionBody =
                    body?.type === "BlockStatement" &&
                    (body.parent?.type === "FunctionDeclaration" ||
                        body.parent?.type === "FunctionExpression" ||
                        body.parent?.type === "ArrowFunctionExpression");
                if (!isFunctionBody || body.body.length < 3 || body.body[0] === node) {
                    return;
                }

                // require an empty line between the previous code or comment and the return
                const previous = context.sourceCode.getTokenBefore(node, {
                    includeComments: true,
                })!;
                if (node.loc.start.line - previous.loc.end.line < 2) {
                    context.report({
                        node,
                        messageId: "blank",
                        fix: (fixer) => fixer.insertTextAfter(previous, "\n"),
                    });
                }
            },
        };
    },
};
