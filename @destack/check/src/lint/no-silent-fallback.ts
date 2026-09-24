import type { ESTree, Rule } from "@oxlint/plugins";

/** Identifiers and members that act as sentinel values. */
const SENTINEL_NAMES = new Set([
    "NaN",
    "Infinity",
    "MAX_SAFE_INTEGER",
    "MIN_SAFE_INTEGER",
    "MAX_VALUE",
]);

/** Reject sentinel fallbacks and caught errors that disappear. */
export const noSilentFallback: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "fail loudly instead of substituting sentinel values" },
        messages: {
            sentinel:
                "[DF02] handle the missing value explicitly instead of substituting a sentinel",
            swallow: "[DF02] rethrow or use the caught error",
        },
    },
    create(context) {
        return {
            LogicalExpression(node) {
                // flag numeric, string and limit sentinels after ?? and ||
                if ((node.operator === "??" || node.operator === "||") && isSentinel(node.right)) {
                    context.report({ node: node.right, messageId: "sentinel" });
                }
            },
            CatchClause(node) {
                // require the handler to rethrow or read the error
                const parameter = node.param?.type === "Identifier" ? node.param.name : undefined;
                const body = context.sourceCode.getText(node.body);
                const isRethrown = /\bthrow\b/.test(body);
                const isRead =
                    parameter !== undefined && new RegExp(`\\b${parameter}\\b`).test(body);
                if (!isRethrown && !isRead) {
                    context.report({ node, messageId: "swallow" });
                }
            },
        };
    },
};

/** Report whether an expression is a numeric, string or limit sentinel. */
function isSentinel(node: ESTree.Expression): boolean {
    // literal zero, negative one and empty string
    if (node.type === "Literal") {
        return node.value === 0 || node.value === "";
    } else if (node.type === "UnaryExpression") {
        return node.operator === "-" && node.argument.type === "Literal";
    }
    // NaN, Infinity and numeric limits
    else if (node.type === "Identifier") {
        return SENTINEL_NAMES.has(node.name);
    } else if (node.type === "MemberExpression" && node.property.type === "Identifier") {
        return SENTINEL_NAMES.has(node.property.name);
    } else {
        return false;
    }
}
