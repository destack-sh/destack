import type { Rule } from "@oxlint/plugins";

/** The widest a documentation line may run, the formatter's line width. */
const DOCUMENTATION_WIDTH = 100;

/** Require documentation comments to be sentences placed directly above their subject. */
export const jsdocSentence: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "write documentation as sentences, one per line" },
        messages: {
            capital: "start the documentation with a capital letter",
            period: "end the documentation with a period",
            sentence: "start each sentence on its own line",
            header: "separate the header line from the paragraph with a blank line",
            detached: "place the documentation directly above the code it describes",
            width: "fit the documentation on one line within 100 characters: keep its first sentence",
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
                        .map((line) => line.replace(/^\s*(?:\* ?)?/u, "").trimEnd());
                    while (text[0]?.trim() === "") {
                        text.shift();
                    }
                    while (text.at(-1)?.trim() === "") {
                        text.pop();
                    }
                    const prose = text.filter((line) => !line.startsWith("@"));
                    while (prose.at(-1)?.trim() === "") {
                        prose.pop();
                    }
                    const [header, second] = prose;
                    const last = prose.at(-1);
                    if (header === undefined || last === undefined) {
                        continue;
                    }
                    const loc = comment.loc;

                    // require a capitalized first sentence and a final period
                    if (!/^(?:[A-Z0-9`"'[(]|(?:npm|oRPC|macOS|iOS|tsgo|tsc|git)\b)/u.test(header)) {
                        context.report({ loc, messageId: "capital" });
                    }
                    if (!/[.?!]$|```$/u.test(last)) {
                        context.report({ loc, messageId: "period" });
                    }

                    // require one sentence per line and a header before paragraphs
                    if (prose.some((line) => /[a-z0-9)`][.?!] +[A-Z]/u.test(line))) {
                        context.report({ loc, messageId: "sentence" });
                    }
                    if (second !== undefined && second.trim() !== "") {
                        context.report({ loc, messageId: "header" });
                    }

                    // require every line of the documentation to fit the line width
                    const written = lines.slice(comment.loc.start.line - 1, comment.loc.end.line);
                    if (written.some((line) => line.length > DOCUMENTATION_WIDTH)) {
                        context.report({ loc, messageId: "width" });
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
