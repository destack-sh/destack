import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Tree, TreeItem } from "./index.ts";
import { draw, press } from "@destack/view/test";

/** Render a tree of notebooks with notes. */
function drawNotebooks(selected: string[]): HTMLElement {
    return draw(() => (
        <Tree aria-label="Notebooks" onValueChange={(value) => selected.push(value)}>
            <TreeItem value="trips" label="Trips">
                <TreeItem value="lisbon" label="Lisbon" />
                <TreeItem value="porto" label="Porto" />
            </TreeItem>
            <TreeItem value="work" label="Work" defaultExpanded>
                <TreeItem value="plans" label="Plans" />
            </TreeItem>
            <TreeItem value="recipes" label="Recipes" />
        </Tree>
    ));
}

/** Press a key and read the value of the focused item. */
function step(key: string): string {
    press(key);
    flush();

    return document.activeElement?.getAttribute("data-value") ?? "none";
}

/** Render the selected Porto under Portugal under Trips, reading the items holding the tab stop. */
function drawTabStops(isTripsExpanded: boolean, isPortugalExpanded: boolean): (string | null)[] {
    const container = draw(() => (
        <Tree aria-label="Notebooks" defaultValue="porto">
            <TreeItem value="recipes" label="Recipes" />
            <TreeItem value="trips" label="Trips" defaultExpanded={isTripsExpanded}>
                <TreeItem value="portugal" label="Portugal" defaultExpanded={isPortugalExpanded}>
                    <TreeItem value="porto" label="Porto" />
                </TreeItem>
            </TreeItem>
        </Tree>
    ));

    return [...container.querySelectorAll("[tabindex='0']")].map((item) =>
        item.getAttribute("data-value"),
    );
}

test("expose items with their level, expansion and selection, the first holding the tab stop", () => {
    const container = drawNotebooks([]);
    const items = [...container.querySelectorAll("[role=treeitem]")].map((item) =>
        [
            item.getAttribute("data-value"),
            item.getAttribute("aria-level"),
            item.getAttribute("aria-expanded"),
            item.getAttribute("tabindex"),
        ].join(" "),
    );
    expect(items).toEqual([
        "trips 1 false 0",
        "lisbon 2  -1",
        "porto 2  -1",
        "work 1 true -1",
        "plans 2  -1",
        "recipes 1  -1",
    ]);
    expect(container.querySelector("[role=group]")?.hasAttribute("hidden")).toBe(true);
});

test("move through visible items, expand and enter parents, and return to them", () => {
    const container = drawNotebooks([]);
    container.querySelector<HTMLElement>("[role=treeitem]")?.focus();
    const visited = [
        "ArrowDown",
        "ArrowDown",
        "ArrowUp",
        "ArrowUp",
        "ArrowRight",
        "ArrowRight",
        "ArrowDown",
        "ArrowLeft",
        "ArrowLeft",
        "End",
        "Home",
        "r",
    ].map(step);

    // collapsed children are skipped until their parent expands, and a letter finds the next match
    expect(visited).toEqual([
        "work",
        "plans",
        "work",
        "trips",
        "trips",
        "lisbon",
        "porto",
        "trips",
        "trips",
        "recipes",
        "trips",
        "recipes",
    ]);
    expect(container.querySelector("[data-value=trips]")?.getAttribute("aria-expanded")).toBe(
        "false",
    );
});

test("select the focused item with Enter and a clicked item, reporting each", () => {
    const selected: string[] = [];
    const container = drawNotebooks(selected);
    container.querySelector<HTMLElement>("[data-value=recipes]")?.focus();
    step("Enter");
    container.querySelector<HTMLElement>("[data-value=plans]")?.click();
    flush();
    expect(selected).toEqual(["recipes", "plans"]);
    expect(container.querySelector("[aria-selected=true]")?.getAttribute("data-value")).toBe(
        "plans",
    );
});

test("leave the focus in place on a letter typed with a shortcut modifier", () => {
    const container = drawNotebooks([]);
    container.querySelector<HTMLElement>("[role=treeitem]")?.focus();
    const shortcuts = ["ctrlKey", "metaKey", "altKey"].map((modifier) => {
        document.activeElement?.dispatchEvent(
            new KeyboardEvent("keydown", { key: "r", [modifier]: true, bubbles: true }),
        );
        flush();

        return document.activeElement?.getAttribute("data-value");
    });
    expect(shortcuts).toEqual(["trips", "trips", "trips"]);
});

test("hold the tab stop on the nearest visible ancestor of a selected item inside a collapsed parent", () => {
    // the outermost collapsed ancestor shows, and a fully expanded path keeps the item itself
    expect([
        drawTabStops(true, false),
        drawTabStops(false, true),
        drawTabStops(true, true),
    ]).toEqual([["portugal"], ["trips"], ["porto"]]);
});
