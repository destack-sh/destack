import { Markdown, MarkdownParser } from "@destack/object/markdown";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { markup, render, testWorld } from "@destack/view/test";
import { RichText, type RichTextReveal } from "./index.ts";

/** Render Markdown, reading its markup without the prose around it. */
function drawn(text: string): string {
    const { container } = render(() => <RichText blocks={Markdown.parse(text)} />);
    flush();

    return markup(container).replace(/^<div[^>]*>|<\/div>$/gu, "");
}

/** Write a link as the page has it. */
function anchor(href: string, text: string): string {
    return `<a href="${href}" target="_blank" rel="noopener noreferrer">${text}</a>`;
}

test("draw text, lists and quotes by their elements", () => {
    expect([
        drawn("## Plan\n\n**bold _both_** ~~gone~~ `code`\\\nnext"),
        drawn("- one\n- two\n  1. nested"),
        drawn("> quoted\n\n---"),
    ]).toEqual([
        "<h2>Plan</h2><p><strong>bold </strong><strong><em>both</em></strong> <s>gone</s> <code>code</code><br>next</p>",
        "<ul><li>one</li><li>two<ol><li>nested</li></ol></li></ul>",
        "<blockquote><p>quoted</p></blockquote><hr>",
    ]);
});

test("link only addresses people open, and load no image unless allowed", () => {
    expect([
        drawn("[web](https://example.com) [mail](mailto:a@b.c)"),
        drawn("[script](javascript:alert) <b onclick=x>html</b>"),
        drawn("![words](https://example.com/a.png?secret=1)"),
    ]).toEqual([
        `<p>${anchor("https://example.com", "web")} <a href="mailto:a@b.c">mail</a></p>`,
        "<p>script html</p>",
        `<p>${anchor("https://example.com/a.png?secret=1", "words")}</p>`,
    ]);
});

test("leave what is shown as it is while text streams, a selection in it too", () => {
    // draw a list and a paragraph being written, and select a word of it
    const parser = new MarkdownParser();
    parser.append("# Title\n\n- one\n- two\n\nA long **sen");
    const [blocks, setBlocks] = createSignal(parser.blocks(true));
    const { container } = render(() => <RichText blocks={blocks()} isStreaming reveal="none" />);
    flush();
    const nodes = () => {
        const found: Node[] = [];
        const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
        while (walker.nextNode() !== null) {
            found.push(walker.currentNode);
        }

        return found;
    };
    const before = nodes();
    const word = document.createRange();
    const [, , , sentence] = before;
    word.setStart(sentence ?? container, 2);
    word.setEnd(sentence ?? container, 6);

    // write on, and into another block
    parser.append("tence** goes on.\n\nNext");
    setBlocks(parser.blocks(true));
    flush();
    const after = nodes();

    expect({
        before: before.map((node) => node.textContent),
        same: before.map((node) => after.includes(node)),
        after: after.map((node) => node.textContent),
        selected: word.toString(),
    }).toEqual({
        before: ["Title", "one", "two", "A long ", "sen"],
        same: [true, true, true, true, true],
        after: ["Title", "one", "two", "A long ", "sen", "tence", " goes on.", "Next"],
        selected: "long",
    });
});

test("draw a block anew that turns out to be of another type", () => {
    const parser = new MarkdownParser();
    parser.append("Title");
    const [blocks, setBlocks] = createSignal(parser.blocks(true));
    const { container } = render(() => <RichText blocks={blocks()} />);
    flush();
    const before = container.querySelector("p, h1")?.outerHTML;
    parser.append("\n=====\n\nText");
    setBlocks(parser.blocks(true));
    flush();

    expect([before, container.querySelector("p, h1")?.outerHTML]).toEqual([
        "<p>Title</p>",
        "<h1>Title</h1>",
    ]);
});

test("bring text up by the line, word or character, and go on from what shows", async () => {
    // write a text that another is added to once it is whole
    const { simulation } = testWorld();
    const shownBy = async (reveal: RichTextReveal) => {
        const [blocks, setBlocks] = createSignal(Markdown.parse("One two"));
        const [isStreaming, setStreaming] = createSignal(true);
        const { container } = render(() => (
            <RichText blocks={blocks()} isStreaming={isStreaming()} reveal={reveal} />
        ));
        const shown: string[] = [];
        const run = async () => {
            for (let frame = 0; frame < 60; frame += 1) {
                await simulation.sleep(17);
                flush();
                const waiting = [...container.querySelectorAll("[data-slot=rich-text-stretch]")];
                const hidden = waiting.filter(
                    (each) => reveal === "line" && !each.hasAttribute("data-revealed"),
                );
                const length = hidden.reduce((sum, { textContent }) => sum + textContent.length, 0);
                const text = container.textContent;
                shown.push(text.slice(0, text.length - length));
            }
        };
        flush();
        await run();
        setStreaming(false);
        flush();
        await run();
        setBlocks(Markdown.parse("One two\n\nthree"));
        setStreaming(true);
        flush();
        const again = container.textContent;
        setStreaming(false);
        flush();
        await run();

        return { steps: [...new Set(shown)], again };
    };

    expect({
        line: await shownBy("line"),
        word: await shownBy("word"),
        character: (await shownBy("character")).steps.length,
        none: await shownBy("none"),
    }).toEqual({
        line: { steps: ["", "One two", "One twothree"], again: "One two" },
        word: { steps: ["One ", "One two", "One twothree"], again: "One two" },
        character: 12,
        none: { steps: ["One two", "One twothree"], again: "One twothree" },
    });
});

test("draw a type by its owner's component", () => {
    const blocks = [
        {
            key: "a",
            type: "callout",
            value: { tone: "warning" },
            spans: [
                { text: "Mind ", marks: { tint: "red" } },
                { node: { type: "mention", value: "Ada" } },
            ],
            blocks: [],
        },
    ];
    const { container } = render(() => (
        <RichText
            blocks={blocks}
            components={{
                blocks: { callout: (properties) => <aside>{properties.children}</aside> },
                marks: { tint: (properties) => <b>{properties.children}</b> },
                nodes: { mention: (properties) => <i>{JSON.stringify(properties.value)}</i> },
            }}
        />
    ));
    flush();

    expect(container.querySelector("aside")?.outerHTML).toBe(
        '<aside><b>Mind </b><i>"Ada"</i></aside>',
    );
});

test("name a todo's box by its text, and check it where its owner handles that", () => {
    // draw todos no one handles, then those someone does
    const [todo] = Markdown.parse("- [ ] Ask the printer");
    const changes: unknown[] = [];
    const { container } = render(() => (
        <>
            <RichText blocks={Markdown.parse("- [x] Send the proof")} />
            <RichText
                blocks={todo === undefined ? [] : [todo]}
                onTodoChange={(block, isChecked) => changes.push([block === todo, isChecked])}
            />
        </>
    ));
    flush();
    const boxes = [...container.querySelectorAll<HTMLElement>("[role=checkbox]")];
    for (const box of boxes) {
        box.click();
    }
    flush();

    expect({
        boxes: boxes.map((box) => [
            document.getElementById(box.getAttribute("aria-labelledby") ?? "")?.textContent,
            box.getAttribute("aria-checked"),
            box.getAttribute("aria-readonly"),
        ]),
        changes,
    }).toEqual({
        boxes: [
            ["Send the proof", "true", "true"],
            ["Ask the printer", "false", null],
        ],
        changes: [[true, true]],
    });
});
