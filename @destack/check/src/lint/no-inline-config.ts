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
                const line = directiveLine(context.sourceCode.getAllComments());
                if (line !== undefined) {
                    throw new CheckError(
                        "CONFIGURATION",
                        `${context.filename}:${line}: fix the code the rule reports, since source cannot change the rules`,
                    );
                }
            },
        };
    },
};

/** Find the line of the first comment that configures oxlint or ESLint rules, absent without one. */
export function directiveLine(
    comments: readonly {
        readonly value: string;
        readonly loc: { readonly start: { readonly line: number } };
    }[],
): number | undefined {
    return comments.find((comment) =>
        /^\s*(?:oxlint|eslint)(?:\s|-(?:disable|enable)\b)/u.test(comment.value),
    )?.loc.start.line;
}
