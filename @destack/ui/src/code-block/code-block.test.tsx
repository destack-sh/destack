import type { LineRange } from "@destack/code";
import typescript from "@destack/code/grammar/typescript";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import { CodeBlock, CodeBlockContent, CodeBlockTitle } from "./index.ts";

test("render code plain until its grammar is there, then in the colors of its tokens", () => {
    const [grammar, setGrammar] = createSignal<typeof typescript | undefined>(undefined);
    const { container } = render(() => (
        <CodeBlockContent code={"const port = 1;\n\n"} grammar={grammar()} />
    ));
    flush();
    const lines = () =>
        [...container.querySelectorAll("[data-slot=code-block-line]")].map((line) =>
            [...(line.lastElementChild?.childNodes ?? [])]
                .filter((node) => node.textContent !== "")
                .map((node) => [
                    node instanceof HTMLElement ? node.getAttribute("data-token") : "plain",
                    node.textContent,
                ]),
        );
    const before = lines();
    setGrammar(typescript);
    flush();

    expect({ before, after: lines() }).toEqual({
        before: [[["plain", "const port = 1;"]], []],
        after: [
            [
                ["keyword", "const"],
                ["plain", " port "],
                ["operator", "="],
                ["plain", " "],
                ["number", "1"],
                ["punctuation", ";"],
            ],
            [],
        ],
    });
});

test("render code in a labelled region its title captions", () => {
    const { container } = render(() => (
        <CodeBlock>
            <CodeBlockTitle>export.ts</CodeBlockTitle>
            <CodeBlockContent code={"const notes = 1;\n"} isNumbered label="export.ts" />
        </CodeBlock>
    ));

    expect(
        markup(container)
            .replaceAll(/ style="[^"]*"/gu, "")
            .replaceAll(/<!--.*?-->/gu, ""),
    ).toBe(
        '<figure data-slot="code-block"><figcaption data-slot="code-block-title">export.ts</figcaption>' +
            '<pre data-slot="code-block-content" dir="ltr" tabindex="0" aria-label="export.ts"><code>' +
            '<span data-slot="code-block-chunk"><span data-slot="code-block-line">' +
            '<span aria-hidden="true" data-line="0" data-line-number="1">1</span>' +
            "<span>const notes = 1;</span></span></span></code></pre></figure>",
    );
});

test("number lines from where they start, mark some, and select those whose numbers are pressed", () => {
    const selections: string[] = [];
    const [selection, setSelection] = createSignal<LineRange | undefined>(undefined);
    const { container } = render(() => (
        <CodeBlockContent
            code={"a\nb\nc\nd\n"}
            isNumbered
            start={41}
            marked={[{ start: 3, end: 4 }]}
            selection={selection()}
            onSelectionChange={(next) => {
                selections.push(`${String(next.start)}-${String(next.end)}`);
                setSelection(next);
            }}
        />
    ));
    flush();
    const rows = (): string[] =>
        [...container.querySelectorAll("[data-slot=code-block-line]")].map(
            (row) => `${row.textContent}${row.hasAttribute("data-marked") ? "*" : ""}`,
        );
    const before = rows();
    const numbers = [...container.querySelectorAll("[data-line-number]")];
    numbers[2]?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0 }));
    flush();
    numbers[0]?.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true, button: 0, shiftKey: true }),
    );
    flush();

    expect({ before, after: rows(), selections }).toEqual({
        before: ["41a", "42b", "43c", "44d*"],
        after: ["41a*", "42b*", "43c*", "44d*"],
        selections: ["2-3", "0-3"],
    });
});
