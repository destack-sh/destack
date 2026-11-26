import { flush } from "solid-js";

/** The modifiers a key step lists before its key, with the keyboard event flag each sets. */
const MODIFIERS = new Map([
    ["Control", "ctrlKey"],
    ["Meta", "metaKey"],
    ["Alt", "altKey"],
    ["Shift", "shiftKey"],
]);

/** A keyboard that presses keys and fills fields on a document as a person's does. */
export class Keyboard {
    /** The document it types into. */
    readonly document: Document;

    /** Create a keyboard on a document. */
    constructor(document: Document) {
        this.document = document;
    }

    /** Press a key with its modifiers joined by plus signs, such as `Control+r`. */
    press(shortcut: string): void {
        // read the modifiers before the key
        const [key, ...held] = shortcut.split("+").toReversed();
        if (key === undefined || key === "") {
            throw new TypeError(`no key in '${shortcut}'`);
        }
        const flags: Record<string, boolean> = {};
        for (const modifier of held) {
            const flag = MODIFIERS.get(modifier);
            if (flag === undefined) {
                throw new TypeError(`unknown modifier '${modifier}' in '${shortcut}'`);
            }
            flags[flag] = true;
        }

        // dispatch the key down and up on the focused element, each settling as its own task
        const target = this.document.activeElement ?? this.document.body;
        for (const type of ["keydown", "keyup"]) {
            flush();
            target.dispatchEvent(
                new KeyboardEvent(type, {
                    key,
                    code: keyCode(key),
                    ...flags,
                    bubbles: true,
                    cancelable: true,
                }),
            );
        }
    }

    /** Replace a field's value as a person typing it would, focusing it first. */
    fill(element: HTMLElement, value: string): void {
        if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) {
            throw new TypeError("only a field can be filled");
        }
        element.focus();
        element.value = value;
        element.dispatchEvent(new InputEvent("input", { bubbles: true, data: value }));
    }
}

/** Read the physical key a key value sits on in a US layout, such as `KeyT` for `t`. */
function keyCode(key: string): string {
    if (/^[a-z]$/iu.test(key)) {
        return `Key${key.toUpperCase()}`;
    } else if (/^[0-9]$/u.test(key)) {
        return `Digit${key}`;
    }

    return key === " " ? "Space" : key;
}
