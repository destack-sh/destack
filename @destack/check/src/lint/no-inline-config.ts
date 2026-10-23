import type { Rule } from "@oxlint/plugins";
import { CheckError } from "../error/index.ts";

/** A comment configuring oxlint or ESLint rules. */
const DIRECTIVE = /^\s*(?:oxlint|eslint)(?:\s|-(?:disable|enable)\b)/u;

/** A suppression of named rules on the next line, with the reason after two dashes. */
const REASONED_SUPPRESSION =
    /^\s*oxlint-disable-next-line\s+[\w@/-]+(?:\s*,\s*[\w@/-]+)*\s+--\s+\S/u;

/** Reject source comments changing the fixed rules, allowing reasoned next-line suppressions. */
export const noInlineConfiguration: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: {
            description:
                "use the fixed Destack rule configuration, suppressing a line only with a reason",
        },
    },
    create(context) {
        return {
            Program() {
                // reject configuration before Oxlint filters suppressed diagnostics
                const line = directiveLine(context.sourceCode.getAllComments());
                if (line !== undefined) {
                    throw CheckError.of(
                        "INVALID_CONFIGURATION",
                        `${context.filename}:${line}: fix the code the rule reports, or suppress its next line with \`oxlint-disable-next-line <rules> -- <reason>\``,
                    );
                }
            },
        };
    },
};

/** Find the line of the first comment configuring rules, skipping reasoned suppressions. */
export function directiveLine(
    comments: readonly {
        readonly value: string;
        readonly loc: { readonly start: { readonly line: number } };
    }[],
): number | undefined {
    return comments.find(
        (comment) => DIRECTIVE.test(comment.value) && !REASONED_SUPPRESSION.test(comment.value),
    )?.loc.start.line;
}
