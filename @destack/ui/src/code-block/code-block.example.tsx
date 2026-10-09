import { defineExample } from "@destack/package/declare";
import {
    CodeBlock,
    CodeBlockContent,
    CodeBlockHeader,
    CodeBlockLine,
    CodeBlockTitle,
} from "./code-block.tsx";

/** The lines of a note's export script, numbered under its file name. */
export const codeBlockExportScript = defineExample({
    of: CodeBlock,
    name: "export-script",
    description: "the lines of a note's export script, numbered under its file name",
    render: () => (
        <CodeBlock>
            <CodeBlockHeader>
                <CodeBlockTitle>export.ts</CodeBlockTitle>
            </CodeBlockHeader>
            <CodeBlockContent isNumbered>
                <CodeBlockLine>
                    <span class="hljs-keyword">const</span> notes = await space.find("note");
                </CodeBlockLine>
                <CodeBlockLine>
                    <span class="hljs-keyword">await</span> write(
                    <span class="hljs-string">"notes.json"</span>, notes);
                </CodeBlockLine>
            </CodeBlockContent>
        </CodeBlock>
    ),
});
