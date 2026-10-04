import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "./index.ts";
import { draw, focused, stubPopovers } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches ${selector}`);
    }

    return element;
}

test("open a context menu at the pointer onto its first item", () => {
    stubPopovers();
    const container = draw(() => (
        <ContextMenu>
            <ContextMenuTrigger>Groceries</ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem>Open</ContextMenuItem>
                <ContextMenuItem>Archive</ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    ));
    const area = find(container, "[data-slot=context-menu-trigger]");
    area.dispatchEvent(
        new MouseEvent("contextmenu", {
            clientX: 40,
            clientY: 120,
            bubbles: true,
            cancelable: true,
        }),
    );
    flush();
    const menu = find(container, "[role=menu]");

    // the menu shows without an anchor, at the pointer
    expect([
        menu.getAttribute("data-popover-open"),
        menu.style.left,
        menu.style.top,
        focused(),
    ]).toEqual(["undefined", "40px", "120px", "Open"]);
});
