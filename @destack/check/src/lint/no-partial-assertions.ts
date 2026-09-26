import type { Rule } from "@oxlint/plugins";
import { isTestFile } from "./word.ts";

/** Matchers that assert part of a value. */
const PARTIAL = new Set(["toContain", "toMatch", "stringContaining", "stringMatching"]);

/** Require tests to assert complete values. */
export const noPartialAssertions: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "assert complete values in tests" },
        messages: { partial: "[CT11] assert the complete value instead of '{{matcher}}'" },
    },
    create(context) {
        return {
            MemberExpression(node) {
                // check matcher names in test modules
                const call = node.parent?.type === "CallExpression" ? node.parent : undefined;
                const pattern = call?.arguments[0];
                const isAnchored =
                    pattern?.type === "Literal" &&
                    "regex" in pattern &&
                    pattern.regex.pattern.startsWith("^") &&
                    pattern.regex.pattern.endsWith("$");
                if (
                    isTestFile(context.filename) &&
                    node.property.type === "Identifier" &&
                    PARTIAL.has(node.property.name) &&
                    !(node.property.name === "stringMatching" && isAnchored)
                ) {
                    context.report({
                        node,
                        messageId: "partial",
                        data: { matcher: node.property.name },
                    });
                }
            },
        };
    },
};
