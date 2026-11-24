import { expect, onTestFinished, test } from "@destack/test";
import { Viewport } from "./viewport.ts";

/** Read the box of a trigger at a place. */
function at(x: number, y: number): DOMRect {
    return new DOMRect(x, y, 10, 10);
}

test("slide the content of a trigger out and the next in, the way between the triggers", () => {
    // read the motions, which the test page does not play
    const moves: unknown[] = [];
    Object.defineProperty(HTMLElement.prototype, "animate", {
        configurable: true,
        value(this: HTMLElement, frames: readonly { readonly translate: string }[]) {
            moves.push([this.textContent, frames.map(({ translate }) => translate)]);

            return new EventTarget();
        },
    });
    onTestFinished(() => void Reflect.deleteProperty(HTMLElement.prototype, "animate"));
    const element = document.createElement("div");
    element.innerHTML = '<div><p id="name">Ada</p></div>';
    const viewport = new Viewport(element);

    // move right, down, and without anything kept
    for (const [name, to] of [
        ["Ben", at(50, 4)],
        ["Cleo", at(5, 90)],
    ] as const) {
        viewport.keep();
        element.querySelector("p")?.replaceChildren(name);
        viewport.slide(at(0, 0), to);
    }
    const previous = [...element.children].slice(1);
    viewport.slide(at(0, 0), at(0, 50));

    expect({
        moves,
        previous: previous.map((each) => [
            each.textContent,
            each.getAttribute("aria-hidden"),
            each.querySelector("[id]"),
        ]),
    }).toEqual({
        moves: [
            ["Ben", ["32px 0px", "0px 0px"]],
            ["Ada", ["0px 0px", "-32px 0px"]],
            ["Cleo", ["0px 32px", "0px 0px"]],
            ["Ben", ["0px 0px", "0px -32px"]],
        ],
        previous: [["Ben", "true", null]],
    });
});

test("leave a form's own fields as they are beside the content that leaves", () => {
    Object.defineProperty(HTMLElement.prototype, "animate", {
        configurable: true,
        value: () => new EventTarget(),
    });
    onTestFinished(() => void Reflect.deleteProperty(HTMLElement.prototype, "animate"));
    const form = document.createElement("form");
    form.innerHTML = '<div id="frame"><div><input name="day" value="sat"></div></div>';
    document.body.append(form);
    onTestFinished(() => form.remove());
    const frame = form.querySelector<HTMLElement>("#frame");
    const viewport = new Viewport(frame ?? form);
    viewport.keep();
    viewport.slide(at(0, 0), at(0, 50));

    // the copy has no name, so the form has its one field
    expect([...form.querySelectorAll("input")].map(({ name, value }) => [name, value])).toEqual([
        ["day", "sat"],
        ["", "sat"],
    ]);
});
