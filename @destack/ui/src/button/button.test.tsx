import { expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import * as style from "@destack/style";
import { Button, buttonStyle } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("render a native button with its variant, size and attributes", () => {
    const container = draw(() => (
        <Button variant="outline" size="sm" type="submit" aria-label="Save note" disabled>
            Save
        </Button>
    ));
    expect(markup(container)).toBe(
        '<button data-slot="button" data-variant="outline" data-size="sm" type="submit" aria-label="Save note" disabled="">Save</button>',
    );
});

test("restyle a button when its variant changes", () => {
    const [variant, setVariant] = createSignal<"default" | "destructive">("default");
    const container = draw(() => <Button variant={variant()} />);
    const before = classes(container);
    setVariant("destructive");
    flush();

    // the button follows its variant without rendering again
    expect(classes(container)).not.toEqual(before);
    expect(markup(container)).toBe(
        '<button data-slot="button" data-variant="destructive" data-size="default"></button>',
    );
});

test("pass the native button to a ref", () => {
    let element: HTMLButtonElement | undefined;
    const container = draw(() => <Button ref={(button) => (element = button)} />);
    expect(element).toBe(container.firstElementChild);
});

test("style a link like a button of the same variant and size", () => {
    const container = draw(() => (
        <>
            <Button variant="outline" size="sm" />
            <a href="/pricing" {...style.attrs(buttonStyle({ variant: "outline", size: "sm" }))} />
        </>
    ));

    // the link takes the button's classes exactly
    const [button, link] = classes(container);
    expect(link).toBe(button);
});
