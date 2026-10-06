import type { ESTree, Rule } from "@oxlint/plugins";
import { isCreateCall } from "./style.ts";

/** Report whether an expression reads StyleX styles: a created style, a list of styles, or a choice between them. */
function isStyles(node: ESTree.Node, created: ReadonlySet<string>): boolean {
    // read lists and choices of styles
    if (node.type === "ArrayExpression") {
        return true;
    } else if (node.type === "LogicalExpression" || node.type === "ConditionalExpression") {
        const [left, right] =
            node.type === "LogicalExpression"
                ? [node.left, node.right]
                : [node.consequent, node.alternate];

        return isStyles(left, created) || isStyles(right, created);
    }

    // find the identifier a member or call chain starts at
    let root: ESTree.Node = node;
    while (root.type === "MemberExpression" || root.type === "CallExpression") {
        root = root.type === "CallExpression" ? root.callee : root.object;
    }

    return root.type === "Identifier" && created.has(root.name);
}

/** Pass StyleX styles to components as `xstyle`, keeping `style` for inline styles. */
export const styleAttribute: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "pass StyleX styles as xstyle" },
        messages: { style: "[WS01] pass StyleX styles as xstyle, keeping style for inline styles" },
    },
    create(context) {
        // collect the names bound to created styles
        const created = new Set<string>();

        return {
            VariableDeclarator(node) {
                if (
                    node.id.type === "Identifier" &&
                    node.init?.type === "CallExpression" &&
                    isCreateCall(node.init)
                ) {
                    created.add(node.id.name);
                }
            },
            JSXAttribute(node) {
                // report a style attribute holding StyleX styles
                const value = node.value;
                if (
                    node.name.type === "JSXIdentifier" &&
                    node.name.name === "style" &&
                    value?.type === "JSXExpressionContainer" &&
                    value.expression.type !== "JSXEmptyExpression" &&
                    isStyles(value.expression, created)
                ) {
                    context.report({ node, messageId: "style" });
                }
            },
        };
    },
};
