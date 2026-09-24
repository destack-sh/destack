import type { Rule } from "@oxlint/plugins";

/** Keywords that mark notes and their required tags. */
const KEYWORD = /^(?:NOTE|TODO|FUGU)\b/;
/** A keyword followed by one of the documented tags. */
const TAGGED =
    /^(?:NOTE|TODO|FUGU) #(?:Performance|Robustness|Broken|Cleanup|Incomplete|Suspicious|Security|Architecture)\b/;

/** Require short lowercase line comments in the Destack style. */
export const commentStyle: Rule = {
    meta: {
        type: "suggestion",
        fixable: "code",
        schema: [],
        docs: { description: "write short lowercase line comments without final periods" },
        messages: {
            space: "[WC01] start the comment with a space",
            lowercase: "[WC01] start the comment with a lowercase letter",
            period: "[WC04] remove the final period",
            tag: "[WC17] tag the keyword comment, like 'TODO #Cleanup: ...'",
            dash: "[WC07] use a colon or comma instead of a dash",
            continuation: "[WC05] indent the continued comment line by one extra space",
        },
    },
    create(context) {
        return {
            Program() {
                let previousLine = -1;
                for (const comment of context.sourceCode.getAllComments()) {
                    // select ordinary line comments
                    const value = comment.value;
                    const isContinued = comment.loc.start.line === previousLine + 1;
                    previousLine = comment.type === "Line" ? comment.loc.end.line : -1;
                    if (
                        comment.type !== "Line" ||
                        /^(?:\/|#|!|\s*@ts-|\s*=+|\s*(?:oxlint|eslint|region|endregion)\b)/.test(
                            value,
                        )
                    ) {
                        continue;
                    }
                    const loc = comment.loc;
                    const range = comment.range as [number, number];

                    // require a leading space and an extra one on continued lines
                    if (!value.startsWith(" ")) {
                        context.report({ loc, messageId: "space" });
                        continue;
                    }
                    if (isContinued && !value.startsWith("  ") && !KEYWORD.test(value.trim())) {
                        context.report({ loc, messageId: "continuation" });
                        continue;
                    }
                    const text = value.trim();
                    if (isContinued || !text) {
                        continue;
                    }

                    // require tagged keywords, lowercase prose and no final period
                    const firstWord = text.split(/\s/)[0];
                    if (KEYWORD.test(text)) {
                        if (!TAGGED.test(text)) {
                            context.report({ loc, messageId: "tag" });
                        }
                    } else if (/^[A-Z]/.test(text) && !isIdentifier(firstWord)) {
                        context.report({
                            loc,
                            messageId: "lowercase",
                            fix: (fixer) =>
                                fixer.replaceTextRange(
                                    range,
                                    `// ${text[0].toLowerCase()}${text.slice(1)}`,
                                ),
                        });
                    }
                    if (text.endsWith(".") && !text.endsWith("..")) {
                        context.report({
                            loc,
                            messageId: "period",
                            fix: (fixer) =>
                                fixer.replaceTextRange(range, `//${value.trimEnd().slice(0, -1)}`),
                        });
                    }

                    // prefer colons and commas over dashes between clauses
                    if (/\s[-–—]\s/.test(text)) {
                        context.report({ loc, messageId: "dash" });
                    }
                }
            },
        };
    },
};

/** Report whether a word names code, like SpaceService, HTTP or Server.start. */
function isIdentifier(word: string): boolean {
    return (
        /^[A-Z][a-z0-9]*[A-Z]/.test(word) || /^[A-Z0-9]{2,}\b/.test(word) || /[._(`<]/.test(word)
    );
}
