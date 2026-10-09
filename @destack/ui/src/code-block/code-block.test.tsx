import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { CodeBlock, CodeBlockContent, CodeBlockLine, CodeBlockTitle } from "./index.ts";

test("render the lines of a code block in a labelled region, numbered when asked", () => {
    const { container } = render(() => (
        <CodeBlock>
            <CodeBlockTitle>export.ts</CodeBlockTitle>
            <CodeBlockContent isNumbered label="export.ts">
                <CodeBlockLine>const notes = 1;</CodeBlockLine>
                <CodeBlockLine render={(part) => <mark {...part} />}>write(notes);</CodeBlockLine>
            </CodeBlockContent>
        </CodeBlock>
    ));

    // the region names the code, and each line is a row, the second rendered as a mark
    expect(markup(container)).toBe(
        '<figure data-slot="code-block"><figcaption data-slot="code-block-title">export.ts</figcaption>' +
            '<pre data-slot="code-block-content" tabindex="0" aria-label="export.ts"><code>' +
            '<span data-slot="code-block-line">const notes = 1;</span>' +
            '<mark data-slot="code-block-line">write(notes);</mark>' +
            "</code></pre></figure>",
    );
});
