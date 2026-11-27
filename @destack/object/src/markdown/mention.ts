import { Subject } from "@destack/sync";
import type { TextNode } from "../sequence/index.ts";

/** The scheme of a mention's address, which a Markdown link to a subject carries. */
const MENTION_SCHEME = "destack:";

/** A mention node of rich text, and its Markdown form: a link from its label to its subject. */
export const Mention = {
    /** Make the node of a mention of a subject. */
    node(subject: Subject): TextNode {
        return { type: "mention", value: subject };
    },

    /** Write a mention as Markdown: its label after an at sign, linked to its subject's address. */
    markdown(subject: Subject, label: string): string {
        const escaped = label.replace(/[\\[\]]/gu, (character) => `\\${character}`);

        return `[@${escaped}](${Mention.address(subject)})`;
    },

    /** Write the address of a subject. */
    address(subject: Subject): string {
        return `${MENTION_SCHEME}${encodeURIComponent(Subject.key(subject))}`;
    },

    /** Read the subject of an address, none for an address of anything else. */
    subject(href: string): Subject | undefined {
        return href.startsWith(MENTION_SCHEME)
            ? Subject.read(decodeURIComponent(href.slice(MENTION_SCHEME.length)))
            : undefined;
    },

    /** Read the subject a node mentions, none for another node. */
    of(node: TextNode): Subject | undefined {
        return node.type === "mention" ? Subject.parse(node.value) : undefined;
    },
};
