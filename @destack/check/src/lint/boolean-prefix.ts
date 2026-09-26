import type { ESTree, Rule } from "@oxlint/plugins";

/** Name prefixes that read as a question. */
const PREFIX = /^_?(?:is|has|should|can|was|did|does|will|needs|allows)[A-Z]/;

/** Operators that always produce a boolean. */
const COMPARISONS = new Set(["==", "===", "!=", "!==", "<", "<=", ">", ">=", "in", "instanceof"]);

/** Require question-shaped names for values initialized as booleans. */
export const booleanPrefix: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "prefix boolean names with is, has or should" },
        messages: { prefix: "[WN16] prefix the boolean '{{name}}' with is, has or should" },
    },
    create(context) {
        /** Report a name bound to a boolean initializer without a question prefix. */
        function check(name: ESTree.Node, value: ESTree.Expression | null | undefined) {
            if (
                name.type === "Identifier" &&
                value &&
                isBoolean(value) &&
                !PREFIX.test(name.name)
            ) {
                context.report({ node: name, messageId: "prefix", data: { name: name.name } });
            }
        }

        return {
            VariableDeclarator(node) {
                check(node.id, node.init);
            },
            PropertyDefinition(node) {
                check(node.key, node.value);
            },
        };
    },
};

/** Report whether an expression always evaluates to a boolean. */
function isBoolean(node: ESTree.Expression): boolean {
    // literals, negations and comparisons
    if (node.type === "Literal") {
        return typeof node.value === "boolean";
    } else if (node.type === "UnaryExpression") {
        return node.operator === "!";
    } else if (node.type === "BinaryExpression") {
        return COMPARISONS.has(node.operator);
    }
    // logical combinations of boolean operands
    else if (node.type === "LogicalExpression" && node.operator !== "??") {
        return isBoolean(node.left) && isBoolean(node.right as ESTree.Expression);
    } else {
        return false;
    }
}
