import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    Editable,
    EditableCancel,
    EditableInput,
    EditablePreview,
    EditableSubmit,
    EditableTrigger,
    type EditableProperties,
} from "./index.ts";

/** Render a text, and follow what it tells its owner. */
function text(properties: EditableProperties) {
    const told: string[] = [];
    const { container } = render(() => (
        <Editable
            defaultValue="Before"
            onValueCommitted={(value) => void told.push(`keep ${value}`)}
            onValueReverted={(value) => void told.push(`leave ${value}`)}
            {...properties}
        />
    ));
    const find = (slot: string): HTMLElement | null =>
        container.querySelector(`[data-slot=editable-${slot}]`);
    const send = (slot: string, event: Event): void => {
        find(slot)?.dispatchEvent(event);
        flush();
    };
    const press = (slot: string, key: string, held: KeyboardEventInit = {}): void =>
        send(slot, new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...held }));
    const write = (words: string): void => {
        const input = container.querySelector<HTMLInputElement | HTMLTextAreaElement>(
            "[data-slot=editable-input]",
        );
        if (input !== null) {
            input.value = words;
            input.dispatchEvent(new Event("input", { bubbles: true }));
        }
        flush();
    };
    const read = (): string | undefined => find("preview")?.textContent;

    return { told, find, send, press, write, read };
}

test("render the text as a button with its placeholder, and a text to read", () => {
    const { container } = render(() => (
        <>
            <Editable name="title" placeholder="Untitled" />
            <Editable readOnly defaultValue="Read" />
        </>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="editable">',
            '<span data-slot="editable-preview" role="button" tabindex="0" aria-description="Press Enter to edit">Untitled</span>',
            '<input type="hidden" name="title" value=""></div>',
            '<div data-slot="editable" data-readonly="">',
            '<span data-slot="editable-preview" data-disabled="">Read</span></div>',
        ].join(""),
    );
});

test("begin by a press or Enter, keep by Enter or leaving, and leave by Escape", () => {
    const title = text({});
    const read: (string | undefined)[] = [];

    // keep by Enter, then by leaving, then leave
    title.send("preview", new MouseEvent("click", { bubbles: true }));
    title.write("One");
    title.press("input", "Enter");
    read.push(title.read());
    title.press("preview", "Enter");
    title.write("Two");
    title.send("input", new FocusEvent("focusout", { bubbles: true }));
    title.find("input")?.dispatchEvent(new FocusEvent("blur"));
    flush();
    read.push(title.read());
    title.press("preview", "F2");
    title.write("Three");
    title.press("input", "Escape");
    read.push(title.read());

    expect({ read, told: title.told }).toEqual({
        read: ["One", "Two", "Two"],
        told: ["keep One", "keep Two", "leave Two"],
    });
});

test("put a required text back where nothing is left", () => {
    const title = text({ required: true });
    title.press("preview", "Enter");
    title.write("  ");
    title.press("input", "Enter");

    expect([title.read(), title.told]).toEqual(["Before", ["leave Before"]]);
});

test("break a line by Enter in a text with lines, and keep it by the command key", () => {
    const note = text({ multiline: true });
    note.press("preview", "Enter");
    note.write("One\nTwo");
    note.press("input", "Enter");
    const open = note.find("input")?.tagName;
    note.press("input", "Enter", { metaKey: true });

    expect([open, note.read()]).toEqual(["TEXTAREA", "One\nTwo"]);
});

test("begin, keep and leave by buttons alone", () => {
    const name = text({
        activationMode: "none",
        submitMode: "none",
        get children() {
            return (
                <>
                    <EditablePreview />
                    <EditableInput />
                    <EditableTrigger />
                    <EditableSubmit />
                    <EditableCancel />
                </>
            );
        },
    });
    const slots = (): boolean[] =>
        ["trigger", "submit", "cancel"].map((slot) => name.find(slot) !== null);
    const press = (slot: string): void => {
        name.find(slot)?.click();
        flush();
    };

    // nothing begins by a press on the text, nor is kept by Enter
    press("preview");
    const before = slots();
    press("trigger");
    const during = slots();
    name.write("One");
    name.press("input", "Enter");
    const kept = slots();
    press("submit");
    const one = name.read();
    press("trigger");
    name.write("Two");
    press("cancel");

    expect({ before, during, kept, one, two: name.read() }).toEqual({
        before: [true, false, false],
        during: [false, true, true],
        kept: [false, true, true],
        one: "One",
        two: "One",
    });
});
