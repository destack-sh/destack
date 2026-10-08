import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, createSignal, flush, resolve } from "solid-js";
import { copyToClipboard, createClipboard, input, writeClipboard } from "./clipboard.ts";

/** A stand-in clipboard holding text items in memory. */
class StubClipboard extends EventTarget implements Clipboard {
    /** The texts written, the latest last. */
    readonly texts: string[] = ["initial"];

    /** Read the latest text as one item. */
    async read(): Promise<ClipboardItem[]> {
        const text = this.texts.at(-1) ?? "";
        const item: ClipboardItem = {
            types: ["text/plain"],
            presentationStyle: "unspecified",
            getType: async () => new Blob([text], { type: "text/plain" }),
        };

        return [item];
    }

    /** Read the latest text. */
    async readText(): Promise<string> {
        return this.texts.at(-1) ?? "";
    }

    /** Refuse items, which the tests write as text only. */
    async write(): Promise<void> {
        throw new TypeError("the stand-in clipboard takes text only");
    }

    /** Write a text. */
    async writeText(text: string): Promise<void> {
        this.texts.push(text);
    }
}

/** The clipboard of the current test. */
let clipboard = new StubClipboard();

beforeEach(() => {
    clipboard = new StubClipboard();
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: clipboard });
});

afterEach(() => {
    document.body.replaceChildren();
});

test("write text to the clipboard, rejecting what the browser refuses", async () => {
    await writeClipboard("written");
    const refused = await writeClipboard([]).then(
        () => "written",
        (error: unknown) => (error instanceof TypeError ? error.message : "other"),
    );

    expect([clipboard.texts, refused]).toEqual([
        ["initial", "written"],
        "the stand-in clipboard takes text only",
    ]);
});

test("start empty, then read the clipboard's items on refetch", async () => {
    const { items, refetch, dispose } = createRoot((disposeRoot) => {
        const [read, readAgain] = createClipboard();

        return { items: read, refetch: readAgain, dispose: disposeRoot };
    });
    const before = items();
    refetch();
    flush();
    await resolve(() => items());
    const after = items().map((item) => [item.type, item.text]);
    dispose();

    expect([before, after]).toEqual([[], [["text/plain", "initial"]]]);
});

test("write a signal to the clipboard as it changes, after its first value", async () => {
    const [data, setData] = createSignal("first");
    const dispose = createRoot((disposeRoot) => {
        createClipboard(data);

        return disposeRoot;
    });
    flush();
    setData("second");
    flush();
    await Promise.resolve();
    dispose();

    expect(clipboard.texts).toEqual(["initial", "second"]);
});

test("copy a field's value, or an element's text, on each click, highlighting when asked", async () => {
    // copy from a field, highlighting it, and from a paragraph
    const field = document.createElement("input");
    field.value = "from the field";
    const paragraph = document.createElement("p");
    paragraph.append("from ", document.createElement("b"), "the paragraph");
    document.body.append(field, paragraph);
    const dispose = createRoot((disposeRoot) => {
        copyToClipboard({ highlight: input() })(field);
        copyToClipboard()(paragraph);

        return disposeRoot;
    });
    field.click();
    paragraph.click();
    await Promise.resolve();
    const selected = [field.selectionStart, field.selectionEnd];
    dispose();
    field.click();
    await Promise.resolve();

    expect([clipboard.texts, selected]).toEqual([
        ["initial", "from the field", "from the paragraph"],
        [0, 14],
    ]);
});
