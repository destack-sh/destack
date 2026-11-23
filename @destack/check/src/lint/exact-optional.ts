import type { Rule } from "@oxlint/plugins";

/** The builders whose `optional` method makes a nullable field or an action's optional input. */
const BUILDERS: ReadonlySet<string> = new Set(["field", "selected", "current", "picked"]);

/** Require schema properties that may be absent to reject an explicit undefined. */
export const exactOptional: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "declare absent schema properties with exactOptional" },
        messages: { optional: "declare a property that may be absent with .exactOptional()" },
    },
    create(context) {
        return {
            CallExpression(node) {
                // match a bare `.optional()` call
                const callee = node.callee;
                const isOptional =
                    callee.type === "MemberExpression" &&
                    callee.property.type === "Identifier" &&
                    callee.property.name === "optional" &&
                    node.arguments.length === 0;
                if (!isOptional) {
                    return;
                }

                // find the identifier the chain starts at
                let root = callee.object;
                while (root.type === "CallExpression" || root.type === "MemberExpression") {
                    root = root.type === "CallExpression" ? root.callee : root.object;
                }

                // report every chain other than a field's or an action input's
                if (root.type !== "Identifier" || !BUILDERS.has(root.name)) {
                    context.report({ node: callee.property, messageId: "optional" });
                }
            },
        };
    },
};
