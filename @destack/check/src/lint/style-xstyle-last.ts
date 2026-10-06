import type { ESTree, Rule } from "@oxlint/plugins";

/** Report whether an expression reads a caller's `xstyle`. */
function isCallerStyles(node: ESTree.Node | null | undefined): boolean {
    return (
        node?.type === "MemberExpression" &&
        node.property.type === "Identifier" &&
        node.property.name === "xstyle"
    );
}

/** Merge a caller's `xstyle` after a component's own styles, so the caller's styles win. */
export const styleXstyleLast: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "merge the caller's xstyle last" },
        messages: { last: "[WS07] merge the caller's xstyle after the component's own styles" },
    },
    create(context) {
        return {
            ArrayExpression(node) {
                // find the caller's styles among others and require them at the end
                const index = node.elements.findIndex((element) => isCallerStyles(element));
                if (index !== -1 && index !== node.elements.length - 1) {
                    const element = node.elements[index];
                    if (element !== null && element !== undefined) {
                        context.report({ node: element, messageId: "last" });
                    }
                }
            },
        };
    },
};
