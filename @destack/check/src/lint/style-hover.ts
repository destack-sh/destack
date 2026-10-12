import type { Rule } from "@oxlint/plugins";
import { eachCreatedProperty, isCreateCall, keyOf } from "./style.ts";

/** Apply hover styles only where the pointer hovers, so touch screens get no sticky hover. */
export const styleHover: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "guard hover styles by the hover media condition" },
        messages: {
            hover: "guard :hover inside { default: null, [media.hover]: value }",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // find each hover condition and require a computed media key inside it
                if (!isCreateCall(node)) {
                    return;
                }
                eachCreatedProperty(node, (property) => {
                    const isGuarded =
                        property.value.type === "ObjectExpression" &&
                        property.value.properties.some(
                            (entry) => entry.type === "Property" && entry.computed,
                        );
                    if (keyOf(property) === ":hover" && !isGuarded) {
                        context.report({ node: property, messageId: "hover" });
                    }
                });
            },
        };
    },
};
