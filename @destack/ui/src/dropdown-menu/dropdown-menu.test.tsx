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
import { dropdownMenuItemDisabled } from "./dropdown-menu.example.tsx";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches '${selector}'`);
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

test("label each menu by the trigger that opens it", () => {
    stubPopovers();
    const { container } = render(dropdownMenuItemDisabled);
    const named = [...container.querySelectorAll("[role=menu]")].map(
        (menu) => document.getElementById(menu.getAttribute("aria-labelledby") ?? "")?.textContent,
    );
    expect(named).toEqual(["Note", "Move to"]);
});

test("render an item as a link with the item's role, closing the menu when followed", () => {
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

/** Send a mouse event to an element at a point and a time in milliseconds. */
function point(type: string, element: Element, x: number, at: number): void {
    const event = new PointerEvent(type, {
        bubbles: true,
        cancelable: true,
        view: window,
        pointerType: "mouse",
        clientX: x,
        detail: type === "click" ? 1 : 0,
    });
    Object.defineProperty(event, "timeStamp", { value: at });
    element.dispatchEvent(event);
    flush();
}

test("open on a press, choose the item a drag is let go over and close over none", () => {
    // a menu of one action
    stubPopovers();
    const ran: string[] = [];
    const { container } = render(() => (
        <DropdownMenu>
            <DropdownMenuTrigger>Note</DropdownMenuTrigger>
            <DropdownMenuContent>
                <DropdownMenuItem onSelect={() => ran.push("rename")}>Rename</DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    ));
    const trigger = find(container, "[data-slot=dropdown-menu-trigger]");
    const item = find(container, "[data-menu-item]");
    const isOpen = () => trigger.getAttribute("aria-expanded");
    const run = (target: Element, x: number, at: number): (string | null)[] => {
        // press the trigger and let go over a target, with the click of a press in place
        point("pointerdown", trigger, 0, 0);
        const pressed = isOpen();
        point("pointerup", target, x, at);
        if (target === trigger) {
            point("click", trigger, x, at);
        }

        return [pressed, isOpen()];
    };

    expect({
        click: run(trigger, 0, 90),
        secondClick: run(trigger, 0, 90),
        drag: run(item, 40, 300),
        brief: run(item, 40, 100),
        close: run(trigger, 0, 90),
        outside: run(document.body, 400, 300),
        ran,
    }).toEqual({
        // a click leaves it open, the next one closes it
        click: ["true", "true"],
        secondClick: ["true", "false"],
        // a drag to the item chooses it, a press too brief for a drag leaves the menu open
        drag: ["true", "false"],
        brief: ["true", "true"],
        close: ["true", "false"],
        // a drag to nothing closes it
        outside: ["true", "false"],
        ran: ["rename"],
    });
});
