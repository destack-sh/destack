import { Message } from "@destack/locale";
import type { Rule } from "@oxlint/plugins";
import { isPackageFile } from "./package.ts";
import { isTestFile, isTestKitFile } from "./word.ts";

/** The most words of a message that needs a context: past two, its phrase tells enough. */
const BARE_WORDS = 2;

/** Require a context on a message of a word or two, which tells its translators too little. */
export const messageContext: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "say what a message of a word or two is where it shows" },
        messages: {
            context:
                'say what \'{{source}}\' is where it shows: t.context("action" | "label" | "option" | "title")',
        },
    },
    create(context) {
        // apply to a package's own messages, leaving tests their texts
        const { filename, settings } = context;
        const isTest = isTestFile(filename) || isTestKitFile(filename);
        if (isTest || !isPackageFile(filename, settings)) {
            return {};
        }

        return {
            TaggedTemplateExpression(node) {
                // take a message of `t` without placeholders
                const { tag, quasi } = node;
                if (tag.type !== "Identifier" || tag.name !== "t" || quasi.expressions.length > 0) {
                    return;
                }

                // require a context on one of a word or two
                const strings = quasi.quasis.map(({ value }) => value.cooked ?? value.raw);
                const { source } = Message.template(strings, []);
                if (source.trim().split(/\s+/u).length <= BARE_WORDS) {
                    context.report({ node, messageId: "context", data: { source } });
                }
            },
        };
    },
};
