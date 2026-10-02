import type { Rule } from "@oxlint/plugins";
import { CheckError } from "../error/index.ts";

/** Reject source comments that change the fixed rule configuration. */
export const noInlineConfiguration: Rule = {
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
                    const isDirective = /^\s*(?:oxlint|eslint)(?:\s|-(?:disable|enable)\b)/u.test(
                        comment.value,
                    );
                    if (isDirective) {
                        throw new CheckError(
                            "configuration",
                            `${context.filename}:${comment.loc.start.line}: fix the code the rule reports, since source cannot change the rules`,
                        );
                    }
                }
            },
        };
    },
};
