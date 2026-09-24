import type { Rule } from "@oxlint/plugins";
import { CheckError } from "../error/index.ts";

/** Judgment rules that source may disable with a stated reason. */
const DISABLEABLE = [
    "destack/no-sludge",
    "destack/prevent-abbreviations",
    "destack/boolean-prefix",
    "destack/no-silent-fallback",
];

/** A file or next-line exception for judgment rules with a stated reason. */
const ALLOWED = new RegExp(
    `^\\s*oxlint-disable(?:-next-line)? (?:${DISABLEABLE.join("|")})(?:, (?:${DISABLEABLE.join("|")}))* -- \\S`,
    "u",
);

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
                    if (isDirective && !ALLOWED.test(comment.value)) {
                        throw new CheckError(
                            "configuration",
                            `${context.filename}:${comment.loc.start.line}: use 'oxlint-disable[-next-line] <judgment rule> -- <reason>'`,
                        );
                    }
                }
            },
        };
    },
};
