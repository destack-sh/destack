import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import {
    Tree,
    TreeItem,
    TreeItemGroup,
    TreeItemGuide,
    TreeItemRow,
    TreeItemToggle,
} from "./index.ts";
import { render } from "@destack/view/test";
import { treeWorkExpanded } from "./tree.example.tsx";

/** Render the selected Porto under Portugal under Trips, reading the items with the tab stop. */
function drawTabStops(isTripsExpanded: boolean, isPortugalExpanded: boolean): (string | null)[] {
    const { container } = render(() => (
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

test("expose items with their level, expansion and selection, the first with the tab stop", () => {
    const { container } = render(treeWorkExpanded);
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

test("put the tab stop on the nearest visible ancestor of a selected item inside a collapsed parent", () => {
    // the outermost collapsed ancestor shows, and a fully expanded path shows the item itself
    expect([
        drawTabStops(true, false),
        drawTabStops(false, true),
        drawTabStops(true, true),
    ]).toEqual([["portugal"], ["trips"], ["porto"]]);
});

test("leave a controlled item at its owner's expansion, reporting the person's toggle", () => {
    // render Trips expanded only while its owner says so
    const changes: boolean[] = [];
    const [isExpanded, setExpanded] = createSignal(false);
    const { container } = render(() => (
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
    const before = expansion();
    setExpanded(true);
    flush();
    expect([before, expansion(), changes]).toEqual(["false", "true", [true]]);
});

test("render an item from its parts, a row and a group", () => {
    const { container } = render(() => (
        <Tree aria-label="Notebooks">
            <TreeItem value="trips" label="Trips" defaultExpanded>
                <TreeItem value="porto">
                    <TreeItemRow>
                        <TreeItemGuide />
                        <TreeItemToggle />
                        <em>Porto</em>
                    </TreeItemRow>
                    <TreeItemGroup>
                        <TreeItem value="ribeira" label="Ribeira" />
                    </TreeItemGroup>
                </TreeItem>
            </TreeItem>
        </Tree>
    ));
    const rows = [...container.querySelectorAll("[data-slot=tree-item-row]")].map((row) =>
        [...row.children].map(
            (child) => child.getAttribute("data-slot") ?? child.tagName.toLowerCase(),
        ),
    );

    expect(rows).toEqual([
        ["tree-item-toggle"],
        ["tree-item-guide", "tree-item-toggle", "em"],
        ["tree-item-toggle"],
    ]);
});

test("run a caller's handlers on an item beside its own", () => {
    const calls: string[] = [];
    const { container } = render(() => (
        <Tree aria-label="Files">
            <TreeItem
                value="plan"
                label="Plan"
                onClick={() => calls.push("click")}
                onKeyDown={() => calls.push("key")}
            />
        </Tree>
    ));
    flush();
    const item = container.querySelector<HTMLElement>("[role=treeitem]");
    item?.click();
    item?.dispatchEvent(new KeyboardEvent("keydown", { key: "x", bubbles: true }));
    flush();

    expect([calls, item?.getAttribute("aria-selected")]).toEqual([["click", "key"], "true"]);
});
