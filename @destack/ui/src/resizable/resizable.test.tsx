import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { createSignal, flush, type JSX, Show } from "@destack/view";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./index.ts";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
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

test("move the panels beside a handle after a panel mounts between others", async () => {
    // show the list between the notebooks and the note, then move its handle a step
    const [isListed, setListed] = createSignal(false);
    const layouts: (readonly number[])[] = [];
    const { container } = render(() => (
        <ResizablePanelGroup onLayout={(layout) => layouts.push(layout)}>
            <ResizablePanel defaultSize={20}>Notebooks</ResizablePanel>
            <ResizableHandle />
            <Show when={isListed()}>
                <ResizablePanel defaultSize={30}>List</ResizablePanel>
                <ResizableHandle />
            </Show>
            <ResizablePanel defaultSize={50}>Note</ResizablePanel>
        </ResizablePanelGroup>
    ));
    setListed(true);
    flush();
    await Promise.resolve();
    flush();
    const panels = [...container.querySelectorAll("[data-slot=resizable-panel]")];
    const handles = [...container.querySelectorAll("[role=separator]")];
    const controlled = handles.map((handle) =>
        panels.findIndex((panel) => panel.id === handle.getAttribute("aria-controls")),
    );
    handles[1]?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    flush();

    expect({ controlled, shares: shares(container), layouts }).toEqual({
        controlled: [0, 1],
        shares: ["20", "40", "40"],
        layouts: [[20, 40, 40]],
    });
});

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
        ?.hasAttribute("data-collapsed");
    press(container, "Enter");
    expect([collapsed, layouts]).toEqual([
        true,
        [
            [0, 100],
            [20, 80],
        ],
    ]);
});

test("restore the layout a group saved under its id", async () => {
    // save a layout in a tab's storage, and render the group in that tab
    await using simulation = testSimulation();
    const tab = new BrowserSimulation(simulation).environment("tab");
    tab.storage.setItem("destack-resizable:notes", JSON.stringify([45, 55]));
    const { container } = render(
        () => (
            <ResizablePanelGroup autoSaveId="notes">
                <ResizablePanel defaultSize={30}>Notebooks</ResizablePanel>
                <ResizableHandle />
                <ResizablePanel>Note</ResizablePanel>
            </ResizablePanelGroup>
        ),
        { environment: tab },
    );
    press(container, "ArrowRight");
    expect([shares(container), tab.storage.getItem("destack-resizable:notes")]).toEqual([
        ["55", "45"],
        "[55,45]",
    ]);
});

test("keep a panel's content as its share changes", () => {
    let built = 0;
    const Count = (): JSX.Element => {
        built += 1;

        return null;
    };
    const { container } = render(() => (
        <ResizablePanelGroup>
            <ResizablePanel>
                <Count />
            </ResizablePanel>
            <ResizableHandle />
            <ResizablePanel />
        </ResizablePanelGroup>
    ));
    const handle = container.querySelector<HTMLElement>("[role=separator]");
    handle?.focus();
    handle?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    flush();

    expect({ built, now: handle?.getAttribute("aria-valuenow") }).toEqual({ built: 1, now: "60" });
});
