import type { Rule } from "@oxlint/plugins";
import { CheckError } from "../error/index.ts";

/** Reject source comments that change the fixed rule configuration. */
export const noInlineConfig: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "use the fixed Destack rule configuration" },
    },
    create(context) {
        return {
            Program() {
                // reject configuration before Oxlint filters suppressed diagnostics
                for (const comment of context.sourceCode.getAllComments()) {
                    if (/^\s*(?:oxlint|eslint)(?:\s|-(?:disable|enable)\b)/u.test(comment.value)) {
                        throw new CheckError(
                            "configuration",
                            `${context.filename}: inline lint configuration is not supported`,
                        );
                    }
                }
            },
        };
    },
};
