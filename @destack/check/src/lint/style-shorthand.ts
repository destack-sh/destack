import type { Rule } from "@oxlint/plugins";
import { eachCreatedProperty, isCreateCall, keyOf } from "./style.ts";

/** The shorthands that take one value per side or corner, which StyleX merges by property. */
const SHORTHANDS = new Set([
    "margin",
    "marginBlock",
    "marginInline",
    "padding",
    "paddingBlock",
    "paddingInline",
    "inset",
    "insetBlock",
    "insetInline",
    "borderWidth",
    "borderStyle",
    "borderColor",
    "borderRadius",
    "gap",
]);

/** Report whether a value lists several parts outside parentheses, as `0 auto` does and `calc(1px + 2px)` does not. */
function isList(text: string): boolean {
    let depth = 0;
    for (const character of text.trim()) {
        if (character === "(") {
            depth += 1;
        } else if (character === ")") {
            depth -= 1;
        } else if (depth === 0 && /\s/u.test(character)) {
            return true;
        }
    }

    return false;
}

/** Write one value per longhand, so later styles override exactly the sides they set. */
export const styleShorthand: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "give shorthands a single value" },
        messages: {
            shorthand: "[WS06] give '{{name}}' one value, writing each side as its own longhand",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // read every literal value of a shorthand
                if (!isCreateCall(node)) {
                    return;
                }
                eachCreatedProperty(node, (property) => {
                    const name = keyOf(property);
                    const value = property.value;
                    if (
                        name !== undefined &&
                        SHORTHANDS.has(name) &&
                        value.type === "Literal" &&
                        typeof value.value === "string" &&
                        isList(value.value)
                    ) {
                        context.report({ node: value, messageId: "shorthand", data: { name } });
                    }
                });
            },
        };
    },
};
