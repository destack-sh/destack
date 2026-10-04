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

test("render the default variant and size when neither is set", () => {
    const container = draw(() => <Button>Save</Button>);
    expect(markup(container)).toBe(
        '<button data-slot="button" data-variant="default" data-size="default">Save</button>',
    );
});

test("select different classes for each variant", () => {
    const container = draw(() => (
        <>
            <Button variant="default" />
            <Button variant="destructive" />
            <Button variant="outline" />
            <Button variant="secondary" />
            <Button variant="ghost" />
            <Button variant="link" />
        </>
    ));

    // six variants style six different ways
    expect(new Set(classes(container)).size).toBe(6);
});

test("select different classes for each size", () => {
    const container = draw(() => (
        <>
            <Button size="default" />
            <Button size="xs" />
            <Button size="sm" />
            <Button size="lg" />
            <Button size="icon" />
            <Button size="icon-xs" />
            <Button size="icon-sm" />
            <Button size="icon-lg" />
        </>
    ));

    // eight sizes style eight different ways
    expect(new Set(classes(container)).size).toBe(8);
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
