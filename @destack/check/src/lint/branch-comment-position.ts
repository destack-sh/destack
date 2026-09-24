import type { ESTree, Rule } from "@oxlint/plugins";

/** Require comments for if and else cases to precede each case. */
export const branchCommentPosition: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "place case comments before each if and else case" },
        messages: { position: "[WC13] move this comment before its if or else case" },
    },
    create(context) {
        /** Report a comment that opens a case block. */
        function check(block: ESTree.Statement | null | undefined) {
            if (block?.type !== "BlockStatement") {
                return;
            }
            const first = context.sourceCode.getTokenAfter(
                context.sourceCode.getFirstToken(block)!,
                {
                    includeComments: true,
                },
            );
            if (first && (first.type === "Line" || first.type === "Block")) {
                context.report({ loc: first.loc, messageId: "position" });
            }
        }

        return {
            IfStatement(node) {
                // check cases of chains with an else branch
                const isChain = node.alternate !== null || node.parent?.type === "IfStatement";
                if (isChain) {
                    check(node.consequent);
                    if (node.alternate?.type === "BlockStatement") {
                        check(node.alternate);
                    }
                }
            },
        };
    },
};
