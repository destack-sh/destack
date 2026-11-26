import { t } from "@destack/locale";
import type { LineRange } from "@destack/code";
import tsx from "@destack/code/grammar/tsx";
import { defineExample } from "@destack/package/declare";
import { CopyButton } from "../copy-button/index.ts";
import { createSignal } from "@destack/view";
import { CodeBlock, CodeBlockContent, CodeBlockHeader, CodeBlockTitle } from "./code-block.tsx";

/** A view of a note. */
const VIEW = `import { t } from "@destack/locale";
import { defineView, useObject } from "@destack/view";
import { Badge } from "@destack/ui/badge";
import { note } from "../object/note.ts";

/** The most words shown before a note is cut. */
const MAX_WORDS = 120;

/** Render a note with its title and how long it takes to read. */
export default defineView({ type: note }, (properties: { readonly id: string }) => {
    // follow the note as anyone changes it
    const [current, { update }] = useObject(note, () => properties.id);
    const minutes = () => Math.ceil(current().words / 220);
    const isLong = () => current().words > MAX_WORDS && !current().isOpen;

    return (
        <article data-long={isLong()} onClick={() => update({ isOpen: true })}>
            <h1>{current().title ?? t\`Untitled\`}</h1>
            <Badge variant="outline">{\`\${minutes()} min\`}</Badge>
        </article>
    );
});
`;

/** A note's view, numbered under its file name. */
export const codeBlockView = defineExample({
    of: CodeBlock,
    name: "view",
    description: t`A note's view, numbered under its file name.`,
    render: () => (
        <CodeBlock>
            <CodeBlockHeader>
                <CodeBlockTitle>note.tsx</CodeBlockTitle>
                <CopyButton value={VIEW} label="Copy the view" isIconOnly size="sm" />
            </CodeBlockHeader>
            <CodeBlockContent code={VIEW} grammar={tsx} isNumbered label="note.tsx" />
        </CodeBlock>
    ),
});

/** A note's view without a grammar or line numbers. */
export const codeBlockViewPlain = defineExample({
    of: CodeBlock,
    name: "viewPlain",
    description: t`A note's view without a grammar or line numbers.`,
    render: () => (
        <CodeBlock>
            <CodeBlockHeader>
                <CodeBlockTitle>note.tsx</CodeBlockTitle>
            </CodeBlockHeader>
            <CodeBlockContent code={VIEW} label="note.tsx" />
        </CodeBlock>
    ),
});

/** Some lines of a file whose selected lines stand out, long ones wrapped. */
export const codeBlockLines = defineExample({
    of: CodeBlockContent,
    name: "lines",
    description: t`Some lines of a file, with a marked one and those a person selects by their numbers.`,
    properties: { isNumbered: true, overflow: "wrap" },
    render: (properties) => {
        const [selection, setSelection] = createSignal<LineRange | undefined>({
            start: 7,
            end: 10,
        });

        return (
            <CodeBlock>
                <CodeBlockHeader>
                    <CodeBlockTitle>src/view/note.tsx</CodeBlockTitle>
                    <CopyButton value={VIEW} label="Copy the view" isIconOnly size="sm" />
                </CodeBlockHeader>
                <CodeBlockContent
                    {...properties}
                    code={VIEW}
                    grammar={tsx}
                    label="src/view/note.tsx"
                    start={98}
                    marked={[{ start: 13, end: 14 }]}
                    selection={selection()}
                    onSelectionChange={setSelection}
                />
            </CodeBlock>
        );
    },
});
