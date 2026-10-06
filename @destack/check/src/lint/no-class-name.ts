import type { Rule } from "@oxlint/plugins";

/** Style elements through StyleX instead of class names. */
export const noClassName: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "style elements through StyleX" },
        messages: { className: "[WS02] style the element through xstyle instead of a class name" },
    },
    create(context) {
        return {
            JSXAttribute(node) {
                if (node.name.type === "JSXIdentifier" && node.name.name === "className") {
                    context.report({ node, messageId: "className" });
                }
            },
        };
    },
};
