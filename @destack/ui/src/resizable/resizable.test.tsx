import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./index.ts";
import { render } from "@destack/view/test";

/** The custom property StyleX sets to a panel's dynamic flex grow, its share of the group. */
const SHARE_PROPERTY = "--x-flexGrow";

/** Read the computed share of each panel. */
function shares(container: Element): string[] {
    return [...container.querySelectorAll<HTMLElement>("[data-slot=resizable-panel]")].map(
        (panel) => getComputedStyle(panel).getPropertyValue(SHARE_PROPERTY),
    );
}

test("split a group by its panels' shares, the splitter reporting the first panel's share and limits", () => {
    const { container } = render(() => (
        <ResizablePanelGroup direction="horizontal">
            <ResizablePanel defaultSize={30} minSize={20} maxSize={60}>
                Notebooks
            </ResizablePanel>
            <ResizableHandle withHandle />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    ));
    const handle = container.querySelector<HTMLElement>("[role=separator]");
    expect([
        shares(container),
        handle?.getAttribute("aria-valuenow"),
        handle?.getAttribute("aria-valuemin"),
        handle?.getAttribute("aria-valuemax"),
        handle?.getAttribute("aria-orientation"),
    ]).toEqual([["30", "70"], "30", "20", "60", "vertical"]);
});

/** Press a key on a container's first handle. */
function press(container: Element, key: string): void {
    container
        .querySelector("[role=separator]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

test("collapse a collapsible panel on Enter and restore it to its smallest share, reporting each layout", () => {
    const layouts: (readonly number[])[] = [];
    const { container } = render(() => (
        <ResizablePanelGroup onLayout={(layout) => layouts.push(layout)}>
            <ResizablePanel defaultSize={30} minSize={20} collapsible>
                Notebooks
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    ));
    press(container, "Enter");
    const collapsed = container
        .querySelector("[data-slot=resizable-panel]")
        ?.getAttribute("data-state");
    press(container, "Enter");
    expect([collapsed, layouts]).toEqual([
        "collapsed",
        [
            [0, 100],
            [20, 80],
        ],
    ]);
});

test("restore the layout a group saved under its id", () => {
    localStorage.setItem("destack-resizable:notes", JSON.stringify([45, 55]));
    const { container } = render(() => (
        <ResizablePanelGroup autoSaveId="notes">
            <ResizablePanel defaultSize={30}>Notebooks</ResizablePanel>
            <ResizableHandle />
            <ResizablePanel>Note</ResizablePanel>
        </ResizablePanelGroup>
    ));
    press(container, "ArrowRight");
    expect([shares(container), localStorage.getItem("destack-resizable:notes")]).toEqual([
        ["55", "45"],
        "[55,45]",
    ]);
});
