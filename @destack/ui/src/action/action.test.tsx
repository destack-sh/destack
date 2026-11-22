import { expect, test } from "@destack/test";
import { Action } from "./action.ts";

test("run the first enabled action a key press matches, taking the press, and leave other presses alone", () => {
    // a disabled delete before an enabled one, a today action, and presses for each
    const ran: string[] = [];
    const actions: readonly Action[] = [
        {
            id: "delete-all",
            title: "Delete all",
            shortcut: "delete",
            isDisabled: true,
            run: () => void ran.push("all"),
        },
        { id: "delete", title: "Delete", shortcut: "delete", run: () => void ran.push("delete") },
        { id: "today", title: "Today", shortcut: "t", run: () => void ran.push("today") },
        {
            id: "duplicate",
            title: "Duplicate",
            shortcut: "mod+d",
            run: () => void ran.push("duplicate"),
        },
    ];
    const press = (options: KeyboardEventInit) => {
        const event = new KeyboardEvent("keydown", { cancelable: true, ...options });

        return [Action.handle(actions, event, false), event.defaultPrevented];
    };

    expect({
        presses: [
            press({ key: "Delete" }),
            press({ key: "t" }),
            press({ key: "d", ctrlKey: true }),
            press({ key: "d", metaKey: true }),
            press({ key: "x" }),
        ],
        ran,
    }).toEqual({
        // command's mod is control off Apple platforms, so the meta press runs nothing
        presses: [
            [true, true],
            [true, true],
            [true, true],
            [false, false],
            [false, false],
        ],
        ran: ["delete", "today", "duplicate"],
    });
});

/** Press a key on an element, reporting whether the press counts as typing. */
function pressTyping(element: HTMLElement): boolean {
    // listen on the element while it sits in the page
    document.body.append(element);
    let isTyping = false;
    element.addEventListener("keydown", (each) => (isTyping = Action.isTyping(each)));
    element.dispatchEvent(new KeyboardEvent("keydown", { key: "d", bubbles: true }));
    element.remove();

    return isTyping;
}

test("report presses into text fields as typing, and presses on buttons as keys", () => {
    // a press on an input, a combobox, an editable div and a button
    const combobox = document.createElement("div");
    combobox.setAttribute("role", "combobox");
    const editable = document.createElement("div");
    editable.contentEditable = "true";

    expect([
        pressTyping(document.createElement("input")),
        pressTyping(combobox),
        pressTyping(editable),
        pressTyping(document.createElement("button")),
    ]).toEqual([true, true, true, false]);
});
