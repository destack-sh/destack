import { expect, test } from "@destack/test";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./index.ts";
import { draw } from "@destack/view/test";

/** The custom property StyleX sets to a panel's dynamic flex grow, its share of the group. */
const SHARE_PROPERTY = "--x-flex-grow";

/** Read the computed share of each panel. */
function shares(container: Element): string[] {
    return [...container.querySelectorAll<HTMLElement>("[data-slot=resizable-panel]")].map(
        (panel) => getComputedStyle(panel).getPropertyValue(SHARE_PROPERTY),
    );
}

test("split a group by its panels' shares, the splitter reporting the first panel's share and limits", () => {
    const container = draw(() => (
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
