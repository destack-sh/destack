import { expect, test } from "@destack/test";
import { Textarea } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("render a native textarea with its attributes", () => {
    const { container } = render(() => <Textarea name="body" rows={4} placeholder="Write" />);
    expect(markup(container)).toBe(
        '<textarea data-slot="textarea" name="body" rows="4" placeholder="Write"></textarea>',
    );
});

test("submit the form by the command key and Enter, and break the line by Enter alone", () => {
    let submits = 0;
    const { container } = render(() => (
        <form
            onSubmit={(event) => {
                event.preventDefault();
                submits += 1;
            }}
        >
            <Textarea />
        </form>
    ));
    const press = (held: KeyboardEventInit): boolean =>
        container.querySelector("textarea")?.dispatchEvent(
            new KeyboardEvent("keydown", {
                key: "Enter",
                bubbles: true,
                cancelable: true,
                ...held,
            }),
        ) === false;

    expect([
        press({}),
        press({ ctrlKey: true, isComposing: true }),
        press({ ctrlKey: true }),
        submits,
    ]).toEqual([false, false, true, 1]);
});
