import { expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuSub,
    DropdownMenuSubContent,
    DropdownMenuSubTrigger,
    DropdownMenuTrigger,
} from "./index.ts";
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

/** Render a note's dropdown menu with a disabled item and a submenu. */
function drawNoteMenu(selected: string[]): HTMLElement {
    return draw(() => (
        <DropdownMenu>
            <DropdownMenuTrigger>Note</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuItem onSelect={() => selected.push("rename")}>Rename</DropdownMenuItem>
                <DropdownMenuItem disabled>Duplicate</DropdownMenuItem>
                <DropdownMenuSub>
                    <DropdownMenuSubTrigger>Move to</DropdownMenuSubTrigger>
                    <DropdownMenuSubContent>
                        <DropdownMenuItem onSelect={() => selected.push("trips")}>
                            Trips
                        </DropdownMenuItem>
                        <DropdownMenuItem>Work</DropdownMenuItem>
                    </DropdownMenuSubContent>
                </DropdownMenuSub>
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive">Delete</DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    ));
}

test("open a dropdown menu from its trigger onto its first item and move through it", () => {
    stubPopovers();
    const container = drawNoteMenu([]);
    const trigger = find(container, "[data-slot=dropdown-menu-trigger]");
    trigger.click();
    flush();
    const opened = [
        trigger.getAttribute("aria-expanded"),
        find(container, "[role=menu]").getAttribute("data-popover-open"),
        focused(),
    ];

    // arrows skip the disabled item and wrap, Home and End jump, and a letter finds its item
    const visited = ["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp", "End", "Home", "d", "m"].map(
        step,
    );
    expect(opened).toEqual(["true", "dropdown-menu-trigger", "Rename"]);
    expect(visited).toEqual([
        "Move to",
        "Delete",
        "Rename",
        "Delete",
        "Delete",
        "Rename",
        "Delete",
        "Move to",
    ]);
});

test("close a dropdown menu on Escape and return the focus to its trigger", () => {
    stubPopovers();
    const container = drawNoteMenu([]);
    const trigger = find(container, "[data-slot=dropdown-menu-trigger]");
    trigger.focus();
    step("ArrowUp");
    const last = focused();
    step("Escape");
    expect([last, focused(), trigger.getAttribute("aria-expanded")]).toEqual([
        "Delete",
        "Note",
        "false",
    ]);
    expect(find(container, "[role=menu]").hasAttribute("data-popover-open")).toBe(false);
});

test("choose an item with Enter, running its action and closing the menu", () => {
    stubPopovers();
    const selected: string[] = [];
    const container = drawNoteMenu(selected);
    find(container, "[data-slot=dropdown-menu-trigger]").click();
    flush();
    step("Enter");
    expect([
        selected,
        focused(),
        find(container, "[role=menu]").hasAttribute("data-popover-open"),
    ]).toEqual([["rename"], "Note", false]);
});

test("open a submenu with the arrow key toward it and leave it with the arrow key back", () => {
    stubPopovers();
    const selected: string[] = [];
    const container = drawNoteMenu(selected);
    find(container, "[data-slot=dropdown-menu-trigger]").click();
    flush();
    step("ArrowDown");
    const inside = step("ArrowRight");
    const expanded = find(container, "[data-slot=dropdown-menu-sub-trigger]").getAttribute(
        "aria-expanded",
    );
    const back = step("ArrowLeft");
    step("ArrowRight");
    step(" ");

    // the submenu's item runs and closes every menu, returning the focus to the top trigger
    expect([inside, expanded, back, selected, focused()]).toEqual([
        "Trips",
        "true",
        "Move to",
        ["trips"],
        "Note",
    ]);
});

test("check items and select radio items, reporting their state", () => {
    stubPopovers();
    const [isShown, setShown] = createSignal(false);
    const [sort, setSort] = createSignal("title");
    const container = draw(() => (
        <DropdownMenu>
            <DropdownMenuTrigger>View</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuCheckboxItem checked={isShown()} onCheckedChange={setShown}>
                    Show archived
                </DropdownMenuCheckboxItem>
                <DropdownMenuRadioGroup value={sort()} onValueChange={setSort}>
                    <DropdownMenuRadioItem value="title">Title</DropdownMenuRadioItem>
                    <DropdownMenuRadioItem value="date">Date</DropdownMenuRadioItem>
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    ));
    find(container, "[role=menuitemcheckbox]").click();
    find(container, "[role=menuitemradio]:last-child").click();
    flush();
    const states = [...container.querySelectorAll("[aria-checked]")].map((item) =>
        item.getAttribute("aria-checked"),
    );
    expect(states).toEqual(["true", "false", "true"]);
});

test("name each menu by the trigger that opens it", () => {
    stubPopovers();
    const container = drawNoteMenu([]);
    const named = [...container.querySelectorAll("[role=menu]")].map(
        (menu) => document.getElementById(menu.getAttribute("aria-labelledby") ?? "")?.textContent,
    );
    expect(named).toEqual(["Note", "Move to"]);
});

test("leave disabled checkbox and radio items unchanged, and a disabled submenu closed", () => {
    stubPopovers();
    const changes: string[] = [];
    const container = draw(() => (
        <DropdownMenu>
            <DropdownMenuTrigger>View</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuCheckboxItem
                    checked={false}
                    disabled
                    onCheckedChange={() => changes.push("archived")}
                >
                    Show archived
                </DropdownMenuCheckboxItem>
                <DropdownMenuRadioGroup
                    value="title"
                    onValueChange={(value) => changes.push(value)}
                >
                    <DropdownMenuRadioItem value="date" disabled>
                        Date
                    </DropdownMenuRadioItem>
                </DropdownMenuRadioGroup>
                <DropdownMenuSub>
                    <DropdownMenuSubTrigger disabled>Move to</DropdownMenuSubTrigger>
                    <DropdownMenuSubContent>
                        <DropdownMenuItem>Trips</DropdownMenuItem>
                    </DropdownMenuSubContent>
                </DropdownMenuSub>
            </DropdownMenuContent>
        </DropdownMenu>
    ));

    // click each disabled item and press Enter and the arrow key on the disabled submenu's trigger
    find(container, "[role=menuitemcheckbox]").click();
    find(container, "[role=menuitemradio]").click();
    const submenu = find(container, "[data-slot=dropdown-menu-sub-trigger]");
    submenu.click();
    submenu.focus();
    step("Enter");
    step("ArrowRight");
    expect([changes, submenu.getAttribute("aria-expanded")]).toEqual([[], "false"]);
});

test("open and close a controlled dropdown menu by its open property, reporting requests", () => {
    stubPopovers();
    const [isOpen, setOpen] = createSignal(false);
    const changes: boolean[] = [];
    const container = draw(() => (
        <DropdownMenu open={isOpen()} onOpenChange={(open) => changes.push(open)}>
            <DropdownMenuTrigger>Note</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuItem>Rename</DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    ));
    const menu = find(container, "[role=menu]");
    setOpen(true);
    flush();
    const opened = [menu.hasAttribute("data-popover-open"), focused()];
    step("Escape");

    // pressing Escape asks to close, and the menu stays open until its owner closes it
    const asked = [changes, menu.hasAttribute("data-popover-open")];
    setOpen(false);
    flush();
    expect([opened, asked, menu.hasAttribute("data-popover-open")]).toEqual([
        [true, "Rename"],
        [[false], true],
        false,
    ]);
});
