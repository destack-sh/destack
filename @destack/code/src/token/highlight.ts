import { tagHighlighter, tags } from "@lezer/highlight";
import type { TokenKind } from "./token.ts";

/** The kind of token each of Lezer's tags stands for, a tag without one taking its parent's. */
const KINDS = [
    // text
    { tag: tags.comment, class: "comment" },
    { tag: [tags.docString, tags.documentMeta], class: "comment" },
    {
        tag: [tags.punctuation, tags.derefOperator, tags.processingInstruction],
        class: "punctuation",
    },
    { tag: tags.operator, class: "operator" },

    // values
    { tag: [tags.keyword, tags.operatorKeyword, tags.meta, tags.annotation], class: "keyword" },
    { tag: [tags.bool, tags.null, tags.atom, tags.self, tags.color], class: "constant" },
    { tag: tags.constant(tags.name), class: "constant" },
    { tag: [tags.number, tags.unit], class: "number" },
    { tag: [tags.string, tags.regexp, tags.escape, tags.attributeValue], class: "string" },
    { tag: tags.special(tags.string), class: "string" },

    // names
    { tag: [tags.variableName, tags.labelName], class: "variable" },
    { tag: tags.propertyName, class: "property" },
    { tag: tags.function(tags.variableName), class: "function" },
    { tag: tags.function(tags.propertyName), class: "function" },
    { tag: tags.function(tags.definition(tags.variableName)), class: "function" },
    { tag: tags.function(tags.definition(tags.propertyName)), class: "function" },
    { tag: [tags.macroName, tags.standard(tags.variableName)], class: "function" },
    { tag: [tags.typeName, tags.className, tags.namespace], class: "type" },
    { tag: tags.tagName, class: "tag" },
    { tag: tags.attributeName, class: "attribute" },

    // prose
    { tag: tags.heading, class: "heading" },
    { tag: [tags.link, tags.url], class: "link" },
    { tag: tags.strong, class: "strong" },
    { tag: tags.emphasis, class: "emphasis" },
    { tag: tags.monospace, class: "string" },

    // changes
    { tag: tags.inserted, class: "inserted" },
    { tag: tags.deleted, class: "deleted" },
] as const satisfies readonly {
    readonly tag: Parameters<typeof tags.special>[0] | Parameters<typeof tags.special>[0][];
    readonly class: TokenKind;
}[];

/** The kinds by the class the highlighter gives them. */
const CLASSES: ReadonlyMap<string, TokenKind> = new Map(
    KINDS.map((each) => [each.class, each.class]),
);

/** Reads the kind of tokens off Lezer's tags. */
export const HIGHLIGHTER = tagHighlighter(KINDS);

/** Read the kind of the classes of a token, the last for one with several. */
export function kindOf(classes: string | null): TokenKind {
    return CLASSES.get(classes?.split(" ").at(-1) ?? "") ?? "plain";
}
