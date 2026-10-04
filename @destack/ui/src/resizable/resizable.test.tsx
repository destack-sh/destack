import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "./index.ts";
import { draw, press } from "@destack/view/test";

/** Read the flex grow of each panel, its share of the group. */
function shares(container: Element): string[] {
    return [...container.querySelectorAll<HTMLElement>("[data-slot=resizable-panel]")].map(
        (panel) => panel.style.flexGrow,
    );
}

test("move a window splitter with the arrow keys, Home and End, within both panels' limits", () => {
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
    handle?.focus();
    const steps = [shares(container)];
    for (const key of ["ArrowRight", "ArrowLeft", "ArrowLeft", "Home", "End"]) {
        press(key);
        flush();
        steps.push(shares(container));
    }

    // the splitter reports the first panel's share and limits
    expect(steps).toEqual([
        ["30", "70"],
        ["40", "60"],
        ["30", "70"],
        ["20", "80"],
        ["20", "80"],
        ["60", "40"],
    ]);
    expect([
        handle?.getAttribute("aria-valuenow"),
        handle?.getAttribute("aria-valuemin"),
        handle?.getAttribute("aria-valuemax"),
        handle?.getAttribute("aria-orientation"),
    ]).toEqual(["60", "20", "60", "vertical"]);
});
