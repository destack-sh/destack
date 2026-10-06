import { expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "./index.ts";
import { draw, stubPopovers } from "@destack/view/test";
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
    const container = draw(dropdownMenuNoteMenu);
    const named = [...container.querySelectorAll("[role=menu]")].map(
        (menu) => document.getElementById(menu.getAttribute("aria-labelledby") ?? "")?.textContent,
    );
    expect(named).toEqual(["Note", "Move to"]);
});
