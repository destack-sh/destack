import { Block } from "@destack/object";
import { Markdown } from "@destack/object/markdown";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { renderHook } from "@destack/view/test";
import { useMarkdown } from "./index.ts";

test("read text as it is written, and over where another starts", () => {
    // write a text on, finish it, and write another
    const [text, setText] = createSignal("# One\n\nIt is **bol");
    const [isStreaming, setStreaming] = createSignal(true);
    const { result } = renderHook(() => ({ blocks: useMarkdown(text, { isStreaming }) }));
    flush();
    const first = result.blocks();
    setText("# One\n\nIt is **bold** now");
    setStreaming(false);
    flush();
    const second = result.blocks();
    setText("Another");
    flush();

    expect({
        first: Block.plain(first),
        second,
        isKept: first[0] === second[0],
        third: Block.plain(result.blocks()),
    }).toEqual({
        first: "One\nIt is bol\n",
        second: Markdown.parse("# One\n\nIt is **bold** now"),
        isKept: true,
        third: "Another\n",
    });
});
