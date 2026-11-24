import { argument, Message, plural, select } from "@destack/locale";
import type { ESTree, Rule } from "@oxlint/plugins";
import { isPackageFile } from "./package.ts";
import { isTestFile, isTestKitFile } from "./word.ts";

/** A module whose messages no reader of the package sees: an example or a scenario. */
const SAMPLE = /\.(?:example|scenario)\.[jt]sx?$/u;

/** The character between a message's context and its source in a finding. */
export const CONTEXT_END = "\u0000";

/** Find each message of a package's sources, which the check requires its catalogs to translate. */
export const noMissingTranslations: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "translate each message into every language of its package" },
        messages: { message: "{{text}}" },
    },
    create(context) {
        // apply only to a package's own messages, leaving tests and samples their texts
        const { filename, settings } = context;
        const isSample = SAMPLE.test(filename) || isTestFile(filename) || isTestKitFile(filename);
        if (isSample || !isPackageFile(filename, settings)) {
            return {};
        }

        // collect the tags that write messages with their contexts, and the templates of every tag
        const contexts = new Map([["t", ""]]);
        const templates: ESTree.TaggedTemplateExpression[] = [];

        return {
            VariableDeclarator(node) {
                const name = node.init === null ? undefined : contextOf(node.init);
                if (node.id.type === "Identifier" && name !== undefined) {
                    contexts.set(node.id.name, name);
                }
            },
            TaggedTemplateExpression(node) {
                templates.push(node);
            },
            "Program:exit"() {
                // report each template of a message by its context and source, as it is identified
                for (const node of templates) {
                    const { tag, quasi } = node;
                    const isNamed = tag.type === "Identifier";
                    const name = isNamed ? contexts.get(tag.name) : contextOf(tag);
                    if (name !== undefined) {
                        const strings = quasi.quasis.map(({ value }) => value.cooked ?? value.raw);
                        const { source } = Message.template(strings, quasi.expressions.map(partOf));
                        const text = `${name}${CONTEXT_END}${source}`;
                        context.report({ node, messageId: "message", data: { text } });
                    }
                }
            },
        };
    },
};

/** Read the context of a tag's `Message.context(…)`, absent for another expression. */
function contextOf(expression: ESTree.Expression): string | undefined {
    // take a call of a method
    if (expression.type !== "CallExpression" || expression.callee.type !== "MemberExpression") {
        return undefined;
    }

    // read the context of a `Message.context` call
    const { object, property } = expression.callee;
    const isMessage = object.type === "Identifier" && object.name === "Message";
    const isContext = property.type === "Identifier" && property.name === "context";
    const [name] = expression.arguments;

    return isMessage && isContext && name?.type === "Literal" && typeof name.value === "string"
        ? name.value
        : undefined;
}

/** Read the part of a message an expression is: an argument, a plural, a selection or a value. */
function partOf(expression: ESTree.Expression): unknown {
    // take another expression as a value, whose text the message leaves open
    const call = expression.type === "CallExpression" ? expression : undefined;
    const name = call?.callee.type === "Identifier" ? call.callee.name : undefined;
    const [value, forms] = call?.arguments ?? [];
    if (value === undefined || value.type === "SpreadElement") {
        return 0;
    }

    // take an argument by its name, and a variant by its value and forms
    if (name === "argument" && value.type === "Literal" && typeof value.value === "string") {
        return argument(value.value);
    } else if ((name === "plural" || name === "select") && forms?.type === "ObjectExpression") {
        const variant = name === "plural" ? plural(0, formsOf(forms)) : select("", formsOf(forms));

        return { ...variant, value: partOf(value) };
    }

    return 0;
}

/** Read the text of each form of a variant. */
function formsOf(forms: ESTree.ObjectExpression): { readonly other: string } {
    const written = forms.properties.flatMap((property): [string, string][] => {
        // take the forms written as text
        if (property.type !== "Property" || property.value.type !== "Literal") {
            return [];
        }
        const { key, value } = property;
        const name =
            key.type === "Identifier" ? key.name : key.type === "Literal" ? String(key.value) : "";

        return [[name, String(value.value)]];
    });

    return { other: "", ...Object.fromEntries(written) };
}
