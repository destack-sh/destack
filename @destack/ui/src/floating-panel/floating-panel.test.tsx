import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import type { FrameBounds } from "../interaction/index.ts";
import {
    FloatingPanel,
    FloatingPanelBody,
    FloatingPanelContent,
    FloatingPanelHeader,
    FloatingPanelTitle,
} from "./index.ts";

/** Render an open panel, recording its bounds as they change. */
function drawPanel() {
    const changes: FrameBounds[] = [];
    const { container } = render(() => (
        <FloatingPanel defaultOpen>
            <FloatingPanelContent
                defaultBounds={{ start: 100, top: 50, width: 300, height: 200 }}
                onBoundsChange={(bounds) => changes.push(bounds)}
            >
                <FloatingPanelHeader>
                    <FloatingPanelTitle>Notes</FloatingPanelTitle>
                </FloatingPanelHeader>
                <FloatingPanelBody>Ask Grace</FloatingPanelBody>
            </FloatingPanelContent>
        </FloatingPanel>
    ));
    flush();
    const page = container.ownerDocument;
    const press = (slot: string, key: string): void => {
        page.querySelector(slot)?.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
        );
        flush();
    };

    return { page, changes, press };
}

test("move by the header's keys within the window, and resize by the last corner's", () => {
    const { changes, press } = drawPanel();
    press("[data-slot=floating-panel-header]", "ArrowRight");
    press("[data-slot=floating-panel-header]", "ArrowUp");
    for (let count = 0; count < 4; count += 1) {
        press("[data-slot=floating-panel-header]", "ArrowUp");
    }
    press("[data-side=bottom-end]", "ArrowDown");
    press("[data-side=bottom-end]", "ArrowLeft");

    expect([changes[0], changes[1], changes[5], changes[6], changes[7]]).toEqual([
        { start: 116, top: 50, width: 300, height: 200 },
        { start: 116, top: 34, width: 300, height: 200 },
        { start: 116, top: 0, width: 300, height: 200 },
        { start: 116, top: 0, width: 300, height: 216 },
        { start: 116, top: 0, width: 284, height: 216 },
    ]);
});

test("name the panel by its title, and leave the page in reach", () => {
    const { page } = drawPanel();
    const panel = page.querySelector("dialog");
    const title = page.querySelector("[data-slot=floating-panel-title]");

    expect({
        isNamed: panel?.getAttribute("aria-labelledby") === title?.id,
        closedby: panel?.getAttribute("closedby"),
        isOpen: panel?.hasAttribute("open"),
        isModal: panel?.matches(":modal"),
    }).toEqual({ isNamed: true, closedby: "closerequest", isOpen: true, isModal: false });
});
