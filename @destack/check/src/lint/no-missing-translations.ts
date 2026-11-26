import { argument, Message, plural, select } from "@destack/locale";
import type { ESTree, Rule } from "@oxlint/plugins";
import { isPackageFile } from "./package.ts";
import { isTestFile, isTestKitFile } from "./word.ts";

/** The character between a message's context, note and source in a finding. */
export const MESSAGE_SEPARATOR = "\u0000";

/** Find each message of a package's sources, which the check requires its catalogs to translate. */
export const noMissingTranslations: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "translate each message into every language of its package" },
        messages: { message: "{{text}}" },
    },
    create(context) {
        // apply to a package's own messages, examples and scenarios included, leaving tests their texts
        const { filename, settings } = context;
        const isTest = isTestFile(filename) || isTestKitFile(filename);
        if (isTest || !isPackageFile(filename, settings)) {
            return {};
        }

        // collect the tags that write messages with their contexts and notes, and every tag's templates
        const contexts = new Map<string, Tagged>([["t", { context: "", note: "" }]]);
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
                // report each template of a message by its context, note and source
                for (const node of templates) {
                    const { tag, quasi } = node;
                    const isNamed = tag.type === "Identifier";
                    const tagged = isNamed ? contexts.get(tag.name) : contextOf(tag);
                    if (tagged === undefined) {
                        continue;
                    }
                    const strings = quasi.quasis.map(({ value }) => value.cooked ?? value.raw);
                    const { source } = Message.template(strings, quasi.expressions.map(partOf));
                    const text = [tagged.context, tagged.note, source].join(MESSAGE_SEPARATOR);
                    context.report({ node, messageId: "message", data: { text } });
                }
            },
        };
    },
};

/** A tag's context and note, each empty without one. */
interface Tagged {
    /** The context. */
    readonly context: string;
    /** The note. */
    readonly note: string;
}

/** Read the context and note of a tag's `t.context(…)`, absent for another expression. */
function contextOf(expression: ESTree.Expression): Tagged | undefined {
    // take a call of a method
    if (expression.type !== "CallExpression" || expression.callee.type !== "MemberExpression") {
        return undefined;
    }

    // read the context of a `t.context` call
    const { object, property } = expression.callee;
    const isTag = object.type === "Identifier" && object.name === "t";
    const isContext = property.type === "Identifier" && property.name === "context";
    const [name, options] = expression.arguments;
    if (!isTag || !isContext || name?.type !== "Literal" || typeof name.value !== "string") {
        return undefined;
    }

    // read the note of an options literal
    const note =
        options?.type === "ObjectExpression"
            ? options.properties.find(
                  (entry) =>
                      entry.type === "Property" &&
                      entry.key.type === "Identifier" &&
                      entry.key.name === "note",
              )
            : undefined;
    const text =
        note?.type === "Property" &&
        note.value.type === "Literal" &&
        typeof note.value.value === "string"
            ? note.value.value
            : "";

    return { context: name.value, note: text };
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
