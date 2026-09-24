import type { ESTree, Rule } from "@oxlint/plugins";
import { isTestFile } from "./word.ts";

/** The statement count above which function bodies need block comments. */
const MINIMUM_STATEMENTS = 4;

/** Require each blank-line-separated block of a longer function to start with a comment. */
export const requireBlockComment: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "open each logic block with a comment" },
        messages: { missing: "[WL09] describe this logic block with a comment above it" },
    },
    create(context) {
        /** Check the blocks of one function body. */
        function check(body: ESTree.Node | null | undefined) {
            if (
                body?.type !== "BlockStatement" ||
                body.body.length < MINIMUM_STATEMENTS ||
                isTestFile(context.filename)
            ) {
                return;
            }

            // split statements into blocks at blank lines, counting leading comments
            let previousEnd = body.loc.start.line;
            for (const statement of body.body) {
                const comments = context.sourceCode.getCommentsBefore(statement);
                const start = comments[0]?.loc.start.line ?? statement.loc.start.line;
                const isBlockStart = start - previousEnd > 1 || previousEnd === body.loc.start.line;
                previousEnd = statement.loc.end.line;

                // require a comment on every block except a lone return and leading guards
                const isExempt =
                    statement.type === "ReturnStatement" ||
                    (statement === body.body[0] && isGuard(statement));
                if (isBlockStart && !comments.length && !isExempt) {
                    context.report({ node: statement, messageId: "missing" });
                }
            }
        }

        return {
            FunctionDeclaration: (node) => check(node.body),
            FunctionExpression: (node) => check(node.body),
            ArrowFunctionExpression: (node) => check(node.body),
        };
    },
};

/** Report whether a statement is an early exit guard. */
function isGuard(statement: ESTree.Statement): boolean {
    const exit =
        statement.type === "IfStatement" && statement.alternate === null
            ? statement.consequent
            : undefined;
    const exits = exit?.type === "BlockStatement" ? exit.body : exit ? [exit] : [];

    return (
        exits.length === 1 &&
        (exits[0].type === "ReturnStatement" ||
            exits[0].type === "ThrowStatement" ||
            exits[0].type === "ContinueStatement")
    );
}
