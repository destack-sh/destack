import type { Rule } from "@oxlint/plugins";
import { eachCreatedProperty, isCreateCall, keyOf } from "./style.ts";

/** Draw focus rings with the theme's shared state style, so every control shows the same ring. */
export const styleFocus: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "draw focus rings with the theme's shared focus style" },
        messages: {
            focus: "use state.focus from @destack/theme/state instead of a :focus-visible outline",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // find each outline style that turns on under keyboard focus
                if (!isCreateCall(node)) {
                    return;
                }
                eachCreatedProperty(node, (property) => {
                    const isRing =
                        keyOf(property) === "outlineStyle" &&
                        property.value.type === "ObjectExpression" &&
                        property.value.properties.some(
                            (entry) =>
                                entry.type === "Property" && keyOf(entry) === ":focus-visible",
                        );
                    if (isRing) {
                        context.report({ node: property, messageId: "focus" });
                    }
                });
            },
        };
    },
};
