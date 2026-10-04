import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { ToggleGroup, ToggleGroupItem } from "./index.ts";
import { draw, focused, markup, press } from "@destack/view/test";

test("keep one item of a single toggle group on, exposed as radios", () => {
    const changes: (string | undefined)[] = [];
    const container = draw(() => (
        <ToggleGroup
            type="single"
            defaultValue="left"
            aria-label="Alignment"
            onValueChange={(value) => changes.push(value)}
        >
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
        </ToggleGroup>
    ));
    container.querySelectorAll("button")[1]?.click();
    flush();

    // the clicked item replaces the pressed one, and the pressed one holds the tab stop
    expect(markup(container)).toBe(
        '<div role="radiogroup" data-slot="toggle-group" aria-orientation="horizontal" aria-label="Alignment">' +
            '<button type="button" role="radio" aria-checked="false" data-slot="toggle-group-item" data-state="off" data-value="left" tabindex="-1">Left</button>' +
            '<button type="button" role="radio" aria-checked="true" data-slot="toggle-group-item" data-state="on" data-value="center" tabindex="0">Center</button></div>',
    );
    expect(changes).toEqual(["center"]);
});

test("report none once a single group's pressed item turns off, and keep a controlled group's value", () => {
    const changes: (string | undefined)[] = [];
    const container = draw(() => (
        <>
            <ToggleGroup
                type="single"
                defaultValue="left"
                aria-label="Alignment"
                onValueChange={(value) => changes.push(value)}
            >
                <ToggleGroupItem value="left">Left</ToggleGroupItem>
            </ToggleGroup>
            <ToggleGroup type="single" value={undefined} aria-label="Fixed">
                <ToggleGroupItem value="right">Right</ToggleGroupItem>
            </ToggleGroup>
        </>
    ));
    const [first, second] = container.querySelectorAll("button");
    first?.click();
    second?.click();
    flush();

    // the uncontrolled group reports none, and the controlled one stays as its value says
    expect(changes).toEqual([undefined]);
    expect([first?.getAttribute("aria-checked"), second?.getAttribute("aria-checked")]).toEqual([
        "false",
        "false",
    ]);
});

test("press several items of a multiple toggle group", () => {
    const container = draw(() => (
        <ToggleGroup type="multiple" aria-label="Style">
            <ToggleGroupItem value="bold">B</ToggleGroupItem>
            <ToggleGroupItem value="italic">I</ToggleGroupItem>
        </ToggleGroup>
    ));
    for (const button of container.querySelectorAll("button")) {
        button.click();
        flush();
    }
    expect(
        [...container.querySelectorAll("button")].map((button) =>
            button.getAttribute("aria-pressed"),
        ),
    ).toEqual(["true", "true"]);
});

test("move the focus between a toggle group's items with arrow keys, Home and End, wrapping", () => {
    const container = draw(() => (
        <ToggleGroup type="single" aria-label="Alignment">
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
            <ToggleGroupItem value="right" disabled>
                Right
            </ToggleGroupItem>
            <ToggleGroupItem value="justify">Justify</ToggleGroupItem>
        </ToggleGroup>
    ));
    container.querySelector("button")?.focus();
    const visited = [focused()];
    for (const key of ["ArrowRight", "ArrowRight", "ArrowRight", "ArrowLeft", "End", "Home"]) {
        press(key);
        visited.push(focused());
    }
    flush();

    // the disabled item is skipped, and the focused item takes the tab stop
    expect(visited).toEqual(["Left", "Center", "Justify", "Left", "Justify", "Justify", "Left"]);
    expect(container.querySelector("[tabindex='0']")?.textContent).toBe("Left");
});
