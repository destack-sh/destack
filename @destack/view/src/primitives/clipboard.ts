import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { access, type MaybeAccessor, TRANSPARENT } from "./utils.ts";

/** Write text or clipboard items to the clipboard. */
export type ClipboardSetter = (data: string | ClipboardItem[]) => Promise<void>;

/** Make a clipboard item of one type. */
export type NewClipboardItem = (
    type: string,
    data: string | Blob | PromiseLike<string | Blob>,
) => ClipboardItem;

/** Highlight the copied text of an element. */
export type HighlightModifier = (element: HTMLElement) => void;

/** Make a highlight of a range of characters. */
export type Highlighter = (start?: number, end?: number) => HighlightModifier;

/** What `copyToClipboard` copies, how it writes, and what it highlights. */
export type CopyToClipboardOptions = {
    /** The text to copy, the element's own by default. */
    readonly value?: string;
    /** How to write, text to the clipboard by default. */
    readonly setter?: ClipboardSetter;
    /** What to highlight when copying. */
    readonly highlight?: HighlightModifier;
};

/** A clipboard item read back: its type, its blob, and its text when it is plain text. */
export type ClipboardResourceItem = {
    /** The item's type. */
    readonly type: string;
    /** The item's text, for plain text. */
    readonly text: string | undefined;
    /** The item's data. */
    readonly blob: Blob;
};

/** Write text or clipboard items to the clipboard, rejecting when the browser refuses. */
export const writeClipboard: ClipboardSetter = async (data) => {
    // write nothing on the server
    if (isServer) {
        return;
    }
    await (typeof data === "string"
        ? navigator.clipboard.writeText(data)
        : navigator.clipboard.write(data));
};

/** Read the clipboard's items, rejecting when the browser refuses. */
export async function readClipboard(): Promise<ClipboardItem[]> {
    // read nothing on the server
    if (isServer) {
        return [];
    }

    return navigator.clipboard.read();
}

/** Follow the clipboard's items, read again on refetch and on clipboard changes, and write a signal to it as it changes. */
export function createClipboard(
    data?: Accessor<string | ClipboardItem[]>,
    deferInitial?: boolean,
): [
    clipboardItems: Accessor<ClipboardResourceItem[]>,
    refetch: () => void,
    write: ClipboardSetter,
] {
    // hold nothing on the server
    if (isServer) {
        return [() => [], () => {}, async () => {}];
    }

    // start empty, then read the clipboard on each refetch, surfacing a refused read
    const [trigger, setTrigger] = createSignal(undefined, { equals: false, ownedWrite: true });
    const clipboard = createMemo<ClipboardResourceItem[]>((previous) => {
        trigger();

        return previous === undefined ? [] : readClipboardItems();
    }, TRANSPARENT);
    const refetch = (): void => {
        setTrigger(undefined);
    };
    makeEventListener(navigator.clipboard, "clipboardchange", refetch);

    // write each change of the data after the first unless told otherwise, reporting a refused write
    if (data !== undefined) {
        let shouldSkip = deferInitial !== false;
        createEffect(
            data,
            (value) => {
                if (shouldSkip) {
                    shouldSkip = false;

                    return;
                }
                writeClipboard(value).catch(reportError);
            },
            TRANSPARENT,
        );
    }

    return [clipboard, refetch, writeClipboard];
}

/** Copy to the clipboard on each click of the element a ref receives, its field value or text by default. */
export function copyToClipboard(
    options?: MaybeAccessor<CopyToClipboardOptions>,
): (element: HTMLElement) => void {
    // copy the given value, else the field's value or the element's text, reporting a refused write
    let target: HTMLElement | undefined;
    const copy = (): void => {
        if (target === undefined) {
            return;
        }
        const current = access(options) ?? {};
        const value =
            current.value ??
            (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement
                ? target.value
                : (target.textContent ?? ""));
        current.highlight?.(target);
        (current.setter ?? writeClipboard)(value).catch(reportError);
    };
    onCleanup(() => target?.removeEventListener("click", copy));

    return (element) => {
        target = element;
        element.addEventListener("click", copy);
    };
}

/** Make a clipboard item of one type. */
export const newClipboardItem: NewClipboardItem = (type, data) =>
    new ClipboardItem({ [type]: data });

/** Highlight a range of an element's first text. */
export const element: Highlighter =
    (start = 0, end = 0) =>
    (node) => {
        // select the range in the element's first text
        const text = node.firstChild;
        const selection = document.getSelection();
        if (text === null || selection === null) {
            throw new TypeError(
                "an element highlight needs text in the element and a document selection",
            );
        }
        const range = new Range();
        range.setStart(text, start);
        range.setEnd(text, end);
        selection.removeAllRanges();
        selection.addRange(range);
    };

/** Highlight a range of a field's value, all of it by default. */
export const input: Highlighter = (start, end) => (node) => {
    if (!(node instanceof HTMLInputElement || node instanceof HTMLTextAreaElement)) {
        throw new TypeError("an input highlight needs an input or a text area");
    }
    node.setSelectionRange(start ?? 0, end ?? node.value.length);
};

/** Read the clipboard's items with each item's last type, and its text for plain text. */
async function readClipboardItems(): Promise<ClipboardResourceItem[]> {
    const items = await readClipboard();

    return Promise.all(
        items.map(async (item) => {
            // read the item's last type and its data
            const type = item.types.at(-1);
            if (type === undefined) {
                throw new TypeError("a clipboard item holds no type");
            }
            const blob = await item.getType(type);

            return { type, blob, text: type === "text/plain" ? await blob.text() : undefined };
        }),
    );
}
