import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { Tree, TreeItem } from "./index.ts";
import { draw } from "@destack/view/test";
import { treeWorkExpanded } from "./tree.example.tsx";

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
    const container = draw(treeWorkExpanded);
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

test("hold the tab stop on the nearest visible ancestor of a selected item inside a collapsed parent", () => {
    // the outermost collapsed ancestor shows, and a fully expanded path keeps the item itself
    expect([
        drawTabStops(true, false),
        drawTabStops(false, true),
        drawTabStops(true, true),
    ]).toEqual([["portugal"], ["trips"], ["porto"]]);
});

test("hold a controlled item at its owner's expansion, reporting the person's toggle", () => {
    // render Trips expanded only while its owner says so
    const changes: boolean[] = [];
    const [isExpanded, setExpanded] = createSignal(false);
    const container = draw(() => (
        <Tree aria-label="Notebooks">
            <TreeItem
                value="trips"
                label="Trips"
                expanded={isExpanded()}
                onExpandedChange={(expanded) => changes.push(expanded)}
            >
                <TreeItem value="porto" label="Porto" />
            </TreeItem>
        </Tree>
    ));
    const trips = container.querySelector("[data-value=trips]");
    const expansion = () => trips?.getAttribute("aria-expanded");

    // toggle by the person, then by the owner
    trips?.querySelector<HTMLElement>("[data-slot=tree-item-row]")?.click();
    flush();
    const held = expansion();
    setExpanded(true);
    flush();
    expect([held, expansion(), changes]).toEqual(["false", "true", [true]]);
});
