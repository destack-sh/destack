import { afterEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createSelection, getTextNodes } from "./selection.ts";

afterEach(() => {
    window.getSelection()?.removeAllRanges();
    document.body.replaceChildren();
});

/** Make an editable paragraph of two text runs. */
function editable(): HTMLElement {
    const element = document.createElement("p");
    element.contentEditable = "true";
    const strong = document.createElement("strong");
    strong.append("world");
    element.append("hello ", strong, "!");
    document.body.append(element);

    return element;
}

test("list the text nodes inside an element only", () => {
    const element = editable();
    document.body.append("after");

    expect(getTextNodes(element).map((text) => text.data)).toEqual(["hello ", "world", "!"]);
});

test("select text in a field and read it back", () => {
    const input = document.createElement("input");
    input.value = "selectable";
    document.body.append(input);
    const observed = createRoot((disposeRoot) => {
        const [selection, setSelection] = createSelection();
        setSelection([input, 2, 6]);
        flush();
        document.dispatchEvent(new Event("selectionchange"));
        flush();
        const read = selection();
        disposeRoot();

        return [read, document.activeElement === input, input.selectionStart, input.selectionEnd];
    });

    expect(observed).toEqual([[input, 2, 6], true, 2, 6]);
});

test("select text across an editable element's runs and read it back as offsets", () => {
    const element = editable();
    const observed = createRoot((disposeRoot) => {
        const [selection, setSelection] = createSelection();
        setSelection([element, 3, 8]);
        flush();
        document.dispatchEvent(new Event("selectionchange"));
        flush();
        const read = selection();
        const text = window.getSelection()?.toString();
        setSelection([null, null, null]);
        flush();
        disposeRoot();

        return { read, text, ranges: window.getSelection()?.rangeCount };
    });

    expect(observed).toEqual({ read: [element, 3, 8], text: "lo wo", ranges: 0 });
});

test("read no selection outside fields and editable elements", () => {
    const plain = document.createElement("p");
    plain.append("plain");
    document.body.append(plain);
    const observed = createRoot((disposeRoot) => {
        const [selection] = createSelection();
        const range = document.createRange();
        range.selectNodeContents(plain);
        window.getSelection()?.addRange(range);
        document.dispatchEvent(new Event("selectionchange"));
        flush();
        const read = selection();
        disposeRoot();

        return read;
    });

    expect(observed).toEqual([null, null, null]);
});
