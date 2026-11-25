import type { Block } from "@destack/object";
import { MarkdownParser } from "@destack/object/markdown";
import { type Accessor, createMemo } from "@destack/view";

/** How Markdown is followed. */
export interface MarkdownOptions {
    /** Whether the text is still being written, whole by default. */
    readonly isStreaming?: Accessor<boolean>;
}

/** Read Markdown as blocks, finished ones staying the same while it is written. */
export function useMarkdown(
    text: Accessor<string>,
    options: MarkdownOptions = {},
): Accessor<readonly Block[]> {
    // read on from the last text, or over where another text starts
    let parser = new MarkdownParser();

    return createMemo(() => {
        // read what is new of the text
        const written = text();
        if (!written.startsWith(parser.text)) {
            parser = new MarkdownParser();
        }
        parser.append(written.slice(parser.text.length));

        return parser.blocks(options.isStreaming?.() === true);
    });
}
