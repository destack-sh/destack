import { expect, test } from "@destack/test";
import { flush } from "solid-js";
import { Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger } from "./index.ts";
import { draw, focused, press, stubPopovers } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches ${selector}`);
    }

    return element;
}

/** Press a key and let the menus follow. */
function step(key: string): string {
    press(key);
    flush();

    return focused();
}

test("move between a menubar's triggers and cross from one open menu to the next", () => {
    stubPopovers();
    const container = draw(() => (
        <Menubar>
            <MenubarMenu>
                <MenubarTrigger>File</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>New note</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
            <MenubarMenu>
                <MenubarTrigger>Edit</MenubarTrigger>
                <MenubarContent>
                    <MenubarItem>Undo</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
    ));
    find(container, "[data-slot=menubar-trigger]").focus();
    const moves = [
        step("ArrowRight"),
        step("ArrowRight"),
        step("ArrowDown"),
        step("ArrowRight"),
        step("Escape"),
    ];
    const stops = [...container.querySelectorAll("[data-slot=menubar-trigger]")].map((trigger) =>
        trigger.getAttribute("tabindex"),
    );

    // the focused trigger holds the tab stop, and Escape returns to the open menu's trigger
    expect(moves).toEqual(["Edit", "File", "New note", "Undo", "Edit"]);
    expect(stops).toEqual(["-1", "0"]);
});
