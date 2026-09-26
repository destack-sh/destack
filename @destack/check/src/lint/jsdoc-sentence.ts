import type { Rule } from "@oxlint/plugins";

/** Require documentation comments to be sentences placed directly above their subject. */
export const jsdocSentence: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "write documentation as sentences, one per line" },
        messages: {
            capital: "[WD03] start the documentation with a capital letter",
            period: "[WD03] end the documentation with a period",
            sentence: "[WD06] start each sentence on its own line",
            header: "[WD07] separate the header line from the paragraph with a blank line",
            detached: "[WD04] place the documentation directly above the code it describes",
        },
    },
    create(context) {
        return {
            Program() {
                const lines = context.sourceCode.lines;
                for (const comment of context.sourceCode.getAllComments()) {
                    // select documentation comments
                    if (comment.type !== "Block" || !comment.value.startsWith("*")) {
                        continue;
                    }
                    const text = comment.value
                        .slice(1)
                        .split("\n")
                        .map((line) => line.replace(/^\s*(?:\* ?)?/, "").trimEnd());
                    while (text.length && !text[0].trim()) {
                        text.shift();
                    }
                    while (text.length && !text.at(-1)!.trim()) {
                        text.pop();
                    }
                    const prose = text.filter((line) => !line.startsWith("@"));
                    if (!prose.length) {
                        continue;
                    }
                    const loc = comment.loc;

                    // require a capitalized first sentence and a final period
                    if (
                        !/^(?:[A-Z0-9`"'[(]|(?:npm|oRPC|macOS|iOS|tsgo|tsc|git)\b)/.test(prose[0])
                    ) {
                        context.report({ loc, messageId: "capital" });
                    }
                    if (!/[.?!]$|```$/.test(prose.at(-1)!)) {
                        context.report({ loc, messageId: "period" });
                    }

                    // require one sentence per line and a header before paragraphs
                    if (prose.some((line) => /[a-z0-9)`][.?!] +[A-Z]/.test(line))) {
                        context.report({ loc, messageId: "sentence" });
                    }
                    if (prose.length > 1 && prose[1].trim()) {
                        context.report({ loc, messageId: "header" });
                    }

                    // reject documentation separated from its subject by a blank line
                    const next = lines[comment.loc.end.line];
                    if (next !== undefined && !next.trim()) {
                        context.report({ loc, messageId: "detached" });
                    }
                }
            },
        };
    },
};
