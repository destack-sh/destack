import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { ToggleGroup, ToggleGroupItem } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("leave one item of a single toggle group on, exposed as radios", () => {
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

    // the clicked item replaces the pressed one, and the pressed one has the tab stop
    expect(markup(container)).toBe(
        '<div data-slot="toggle-group" role="radiogroup" aria-orientation="horizontal" aria-label="Alignment">' +
            '<button data-slot="toggle-group-item" type="button" role="radio" aria-checked="false" data-value="left" tabindex="-1">Left</button>' +
            '<button data-slot="toggle-group-item" type="button" role="radio" aria-checked="true" data-value="center" tabindex="0" data-pressed="">Center</button></div>',
    );
    expect(changes).toEqual(["center"]);
});

test("report none once a single group's pressed item turns off, and follow a controlled group's value", () => {
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

test("make every item of a disabled toggle group unavailable and report the group as disabled", () => {
    const changes: (string | undefined)[] = [];
    const { container } = render(() => (
        <ToggleGroup disabled aria-label="Alignment" onValueChange={(value) => changes.push(value)}>
            <ToggleGroupItem value="left">Left</ToggleGroupItem>
            <ToggleGroupItem value="center">Center</ToggleGroupItem>
        </ToggleGroup>
    ));
    container.querySelectorAll("button")[1]?.click();
    flush();

    // a click turns nothing on, and the group and both items say they are unavailable
    expect(changes).toEqual([]);
    const group = container.querySelector("[data-slot=toggle-group]");
    expect([group?.getAttribute("aria-disabled"), group?.getAttribute("data-disabled")]).toEqual([
        "true",
        "",
    ]);
    expect([...container.querySelectorAll("button")].map((item) => item.disabled)).toEqual([
        true,
        true,
    ]);
});
