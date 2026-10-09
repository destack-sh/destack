import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "./index.ts";
import { render, stubPopovers } from "@destack/view/test";
import { dropdownMenuNoteMenu } from "./dropdown-menu.example.tsx";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches ${selector}`);
    }

    return element;
}

test("check items and select radio items, reporting their state", () => {
    stubPopovers();
    const [isShown, setShown] = createSignal(false);
    const [sort, setSort] = createSignal("title");
    const { container } = render(() => (
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
    const { container } = render(dropdownMenuNoteMenu);
    const named = [...container.querySelectorAll("[role=menu]")].map(
        (menu) => document.getElementById(menu.getAttribute("aria-labelledby") ?? "")?.textContent,
    );
    expect(named).toEqual(["Note", "Move to"]);
});

test("render an item as a link that keeps the item's role and closes the menu when followed", () => {
    stubPopovers();
    const { container } = render(() => (
        <DropdownMenu defaultOpen>
            <DropdownMenuTrigger>Note</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuItem render={(item) => <a href="#settings" {...item} />}>
                    Settings
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    ));
    const link = find(container, "a[role=menuitem]");
    link.click();
    flush();
    expect([
        link.getAttribute("href"),
        link.dataset["slot"],
        link.textContent,
        find(container, "[data-slot=dropdown-menu-trigger]").getAttribute("aria-expanded"),
    ]).toEqual(["#settings", "dropdown-menu-item", "Settings", "false"]);
});
