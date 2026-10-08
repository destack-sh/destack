import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { ToggleGroup, ToggleGroupItem } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("keep one item of a single toggle group on, exposed as radios", () => {
    const changes: (string | undefined)[] = [];
    const { container } = render(() => (
        <ToggleGroup
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
    const { container } = render(() => (
        <>
            <ToggleGroup
                defaultValue="left"
                aria-label="Alignment"
                onValueChange={(value) => changes.push(value)}
            >
                <ToggleGroupItem value="left">Left</ToggleGroupItem>
            </ToggleGroup>
            <ToggleGroup value={undefined} aria-label="Fixed">
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
    const { container } = render(() => (
        <ToggleGroup multiple aria-label="Style">
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
