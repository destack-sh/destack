import type { Rule } from "@oxlint/plugins";
import { eachCreatedProperty, isCreateCall } from "./style.ts";

/** A literal color: hexadecimal, or a color function over numbers. */
const COLOR = /#[\da-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch)\(\s*[\d.]/iu;

/** The header that marks a module written by a generator, such as the theme's styles from its tokens. */
const GENERATED = "// generate with";

/** Style from the theme's tokens, never raw custom properties or literal colors, outside generated modules. */
export const styleTokens: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "style from theme tokens" },
        messages: {
            variable: "[WS04] read the theme token instead of a raw custom property",
            color: "[WS05] use a color token instead of a literal color",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // read every string value of the created styles a person wrote
                if (!isCreateCall(node) || context.sourceCode.text.startsWith(GENERATED)) {
                    return;
                }
                eachCreatedProperty(node, (property) => {
                    const value = property.value;
                    const text =
                        value.type === "Literal" && typeof value.value === "string"
                            ? value.value
                            : value.type === "TemplateLiteral"
                              ? value.quasis.map((quasi) => quasi.value.raw).join("")
                              : undefined;
                    if (text?.includes("var(--") === true) {
                        context.report({ node: value, messageId: "variable" });
                    } else if (text !== undefined && COLOR.test(text)) {
                        context.report({ node: value, messageId: "color" });
                    }
                });
            },
        };
    },
};
