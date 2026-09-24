import type { Rule } from "@oxlint/plugins";
import { isDeclaredName, splitWords } from "./word.ts";

/** Abstraction words that usually signal an unclear model. */
const SLUDGE = new Set([
    "seam",
    "lane",
    "parts",
    "info",
    "factory",
    "syntax",
    "semantics",
    "data",
    "inner",
    "wrapper",
    "facts",
    "seat",
    "summary",
    "channel",
    "boundary",
    "contract",
    "surface",
    "currency",
    "accounting",
    "bearing",
    "spine",
    "spelling",
    "computation",
    "recipe",
    "glue",
    "judge",
    "proof",
    "evidence",
    "drive",
    "carry",
    "own",
    "demand",
    "grammar",
    "reach",
    "truth",
    "product",
    "atom",
    "axes",
    "coordinates",
    "transcribe",
    "law",
    "knot",
    "tie",
    "seal",
    "pin",
    "tighten",
    "slot",
    "mint",
    "helper",
    "helpers",
    "util",
    "utils",
    "support",
    "misc",
]);

/** Listed words that are also plain English in prose, checked in names only. */
const NAME_ONLY = new Set(["own"]);

/** Reject abstraction words in declared names and comments. */
export const noSludge: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "name things with concrete nouns and verbs" },
        messages: {
            word: "[WN07] replace the abstraction word '{{word}}' with a concrete noun or verb",
        },
    },
    create(context) {
        return {
            Program() {
                // check prose words in every comment
                for (const comment of context.sourceCode.getAllComments()) {
                    // skip lint directives, whose reasons name the excused word
                    if (/^\s*(?:oxlint|eslint)-/.test(comment.value)) {
                        continue;
                    }
                    const words = comment.value.toLowerCase().match(/[a-z]+/g) ?? [];
                    const word = words.find((entry) => SLUDGE.has(entry) && !NAME_ONLY.has(entry));
                    if (word) {
                        context.report({ loc: comment.loc, messageId: "word", data: { word } });
                    }
                }
            },
            Identifier(node) {
                // check names where they are declared
                const word = isDeclaredName(node)
                    ? splitWords(node.name).find((entry) => SLUDGE.has(entry))
                    : undefined;
                if (word) {
                    context.report({ node, messageId: "word", data: { word } });
                }
            },
        };
    },
};
