import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createSignal } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { TRANSPARENT } from "./utils.ts";

/** A text selection: the field or editable element holding it with its start and end offsets, or no selection. */
export type HTMLSelection =
    | [node: HTMLElement, start: number, end: number]
    | [node: null, start: null, end: null];

/** No selection. */
const NO_SELECTION: HTMLSelection = [null, null, null];

/** List the text nodes inside a node, in document order. */
export function getTextNodes(startNode: Node): Text[] {
    // walk the text nodes under the node
    const walker = document.createTreeWalker(startNode, NodeFilter.SHOW_TEXT);
    const texts: Text[] = [];
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        if (node instanceof Text) {
            texts.push(node);
        }
    }

    return texts;
}

/** Follow the text selection in fields and editable elements, and select text by writing it. */
export function createSelection(): [Accessor<HTMLSelection>, (selection: HTMLSelection) => void] {
    // select nothing on the server
    if (isServer) {
        return [() => NO_SELECTION, () => {}];
    }
    const [current, setCurrent] = createSignal<HTMLSelection>(NO_SELECTION, { ownedWrite: true });
    const [wanted, setWanted] = createSignal<HTMLSelection>(NO_SELECTION, { ownedWrite: true });

    // read the selection from the focused field, or from the editable element holding the range
    const read = (): void => {
        const active = document.activeElement;
        if (active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement) {
            const start = active.selectionStart;
            const end = active.selectionEnd;
            setCurrent(start === null || end === null ? NO_SELECTION : [active, start, end]);
        } else {
            setCurrent(rangeSelection(window.getSelection()));
        }
    };
    read();
    makeEventListener(document, "selectionchange", read);
    makeEventListener(document, "click", read);
    makeEventListener(document, "keyup", read);

    // select the written text in its field or editable element, or clear the selection
    createEffect(
        wanted,
        ([node, start, end]) => {
            const selection = window.getSelection();
            if (node === null) {
                selection?.removeAllRanges();
            } else if (node instanceof HTMLInputElement || node instanceof HTMLTextAreaElement) {
                if (document.activeElement !== node) {
                    node.focus();
                }
                node.setSelectionRange(start, end);
            } else {
                selection?.removeAllRanges();
                const texts = getTextNodes(node);
                const [startNode, startOffset] = rangeArgument(start, texts);
                const [endNode, endOffset] =
                    start === end ? [startNode, startOffset] : rangeArgument(end, texts);
                if (startNode !== undefined && endNode !== undefined) {
                    const range = document.createRange();
                    range.setStart(startNode, startOffset);
                    range.setEnd(endNode, endOffset);
                    selection?.addRange(range);
                }
            }
        },
        TRANSPARENT,
    );

    return [current, setWanted];
}

/** Read a document range as offsets into the editable element holding it, or no selection. */
function rangeSelection(selection: Selection | null): HTMLSelection {
    // read nothing without a range inside an editable element
    if (selection === null || selection.rangeCount === 0) {
        return NO_SELECTION;
    }
    const range = selection.getRangeAt(0);
    const parent = editableAncestor(range.commonAncestorContainer);
    if (parent === null) {
        return NO_SELECTION;
    }

    // count the characters before each end of the range
    const texts = getTextNodes(parent);
    const start = offsetIn(range.startContainer, range.startOffset, texts);
    const end = range.collapsed ? start : offsetIn(range.endContainer, range.endOffset, texts);

    return start === undefined || end === undefined ? NO_SELECTION : [parent, start, end];
}

/** Find the nearest editable element holding a node, null outside any. */
function editableAncestor(node: Node | null): HTMLElement | null {
    for (let current = node; current !== null; current = current.parentNode) {
        if (current instanceof HTMLElement && current.contentEditable === "true") {
            return current;
        }
    }

    return null;
}

/** Count the characters before an offset in a container, across the text nodes of its editable element. */
function offsetIn(container: Node, offset: number, texts: readonly Text[]): number | undefined {
    const index = texts.findIndex((text) => text === container || text.parentElement === container);
    if (index === -1) {
        return undefined;
    }

    return texts.slice(0, index).reduce((length, text) => length + text.data.length, 0) + offset;
}

/** Find the text node and its offset that an offset counted across text nodes falls in. */
function rangeArgument(
    offset: number,
    texts: readonly Text[],
): [node: Text | undefined, offset: number] {
    let remaining = offset;
    for (const text of texts) {
        if (remaining <= text.data.length) {
            return [text, remaining];
        }
        remaining -= text.data.length;
    }

    return [undefined, remaining];
}
