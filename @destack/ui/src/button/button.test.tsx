import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
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
        '<button data-slot="button" data-variant="outline" data-size="sm" disabled="" type="submit" aria-label="Save note">Save</button>',
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

test("render a link with the button's part attributes, its native button's attributes left out", () => {
    const container = draw(() => (
        <Button
            variant="outline"
            type="submit"
            aria-label="Open notes"
            render={(part) => <a href="/notes" {...part} />}
        >
            Notes
        </Button>
    ));
    expect(markup(container)).toBe(
        '<a href="/notes" aria-label="Open notes" data-slot="button" data-variant="outline" data-size="default">Notes</a>',
    );
});

test("show a spinner and mark a loading button busy, refusing clicks until it finishes", () => {
    const clicks: string[] = [];
    const container = draw(() => (
        <Button loading onClick={() => clicks.push("save")}>
            Save
        </Button>
    ));
    const button = container.querySelector("button");
    button?.click();
    expect([
        button?.getAttribute("aria-busy"),
        button?.disabled,
        button?.querySelector("[data-slot=spinner]")?.getAttribute("aria-hidden"),
        clicks,
    ]).toEqual(["true", true, "true", []]);
});
