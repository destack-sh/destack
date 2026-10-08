import { expect, test } from "@destack/test";
import { createSignal, flush, type JSX } from "@destack/view";
import { draw, focused, press, stubPopovers } from "@destack/view/test";
import {
    Menu,
    MenuCheckboxItem,
    MenuContent,
    MenuControl,
    MenuItem,
    MenuRadioGroup,
    MenuRadioItem,
    MenuSubTrigger,
} from "./index.ts";

/** The items of a test menu: two actions, a submenu and a disabled action. */
function Items(properties: { readonly onSelect: (name: string) => void }): JSX.Element {
    return (
        <>
            <MenuItem onSelect={() => properties.onSelect("Copy")}>Copy</MenuItem>
            <MenuItem onSelect={() => properties.onSelect("Paste")}>Paste</MenuItem>
            <Menu>
                <MenuSubTrigger>Move to</MenuSubTrigger>
                <MenuContent>
                    <MenuItem onSelect={() => properties.onSelect("Inbox")}>Inbox</MenuItem>
                    <MenuItem onSelect={() => properties.onSelect("Archive")}>Archive</MenuItem>
                </MenuContent>
            </Menu>
            <MenuItem disabled onSelect={() => properties.onSelect("Delete")}>
                Delete
            </MenuItem>
        </>
    );
}

/** Render a top menu around a trigger and its content, returning the container and the menu. */
function drawMenu(
    content: () => JSX.Element,
): readonly [HTMLElement, () => MenuControl | undefined] {
    stubPopovers();
    let menu: MenuControl | undefined;
    const container = draw(() => {
        const control = new MenuControl(null);
        menu = control;

        return (
            <Menu control={control}>
                <button
                    ref={(element) => control.setTrigger(element)}
                    onClick={() => control.open("first")}
                >
                    Note
                </button>
                <MenuContent>{content()}</MenuContent>
            </Menu>
        );
    });

    return [container, () => menu];
}

/** Open a menu from its trigger. */
function open(container: HTMLElement): void {
    container.querySelector("button")?.click();
    flush();
}

/** Press keys in turn, reading the focused element's text after each. */
function walk(keys: readonly string[]): string[] {
    return keys.map((key) => {
        press(key);
        flush();

        return focused();
    });
}

test("focus the first item on opening and move through the items with the arrows, Home, End and typed letters", () => {
    const [container] = drawMenu(() => <Items onSelect={() => undefined} />);
    open(container);
    const first = focused();

    // disabled items are skipped and the arrows wrap
    expect([
        first,
        ...walk(["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp", "Home", "End", "p", "m"]),
    ]).toEqual([
        "Copy",
        "Paste",
        "Move to",
        "Copy",
        "Move to",
        "Copy",
        "Move to",
        "Paste",
        "Move to",
    ]);
});

test("open a submenu toward its arrow, close it back out and list each menu's own items", () => {
    const [container, menu] = drawMenu(() => <Items onSelect={() => undefined} />);
    open(container);
    const opened = walk(["End", "ArrowRight"]);
    const list = menu()?.list;
    const items = list?.collection
        .keys()
        .filter((key) => list.delegate.has(key))
        .map((key) => list.collection.text(key));
    const closed = walk(["ArrowLeft"]);
    const expanded = container
        .querySelector("[data-slot=menu-sub-trigger]")
        ?.getAttribute("aria-expanded");

    expect({ opened, items, closed, expanded }).toEqual({
        opened: ["Move to", "Inbox"],
        items: ["Copy", "Paste", "Move to"],
        closed: ["Move to"],
        expanded: "false",
    });
});

test("run a chosen item's action and close every menu, returning the focus to the trigger", () => {
    // choose an item of the submenu
    const chosen: string[] = [];
    const [container, menu] = drawMenu(() => <Items onSelect={(name) => chosen.push(name)} />);
    open(container);
    walk(["End", "ArrowRight", "ArrowDown", "Enter"]);

    expect({ chosen, isOpen: menu()?.isOpen(), focused: focused() }).toEqual({
        chosen: ["Archive"],
        isOpen: false,
        focused: "Note",
    });
});

test("ignore a disabled item and keep the menu open for an action that prevents the default", () => {
    const chosen: string[] = [];
    const [container, menu] = drawMenu(() => (
        <>
            <MenuItem onSelect={(event) => event.preventDefault()}>Pin</MenuItem>
            <MenuItem disabled onSelect={() => chosen.push("Delete")}>
                Delete
            </MenuItem>
        </>
    ));
    open(container);
    for (const item of container.querySelectorAll<HTMLElement>("[role=menuitem]")) {
        item.click();
    }
    flush();

    expect({ chosen, isOpen: menu()?.isOpen() }).toEqual({ chosen: [], isOpen: true });
});

test("close on Escape returning the focus to the trigger, and on Tab without it", () => {
    const [container, menu] = drawMenu(() => <Items onSelect={() => undefined} />);
    open(container);
    const escaped = [...walk(["Escape"]), menu()?.isOpen()];
    open(container);
    press("Tab");
    flush();

    expect({ escaped, tabbed: menu()?.isOpen() }).toEqual({
        escaped: ["Note", false],
        tabbed: false,
    });
});

test("check items and select radio items, reporting their state", () => {
    const [isShown, setShown] = createSignal(false);
    const [sort, setSort] = createSignal<string | undefined>("title");
    const [container] = drawMenu(() => (
        <>
            <MenuCheckboxItem
                checked={isShown()}
                onCheckedChange={setShown}
                onSelect={(event) => event.preventDefault()}
            >
                Show archived
            </MenuCheckboxItem>
            <MenuRadioGroup value={sort()} onValueChange={setSort}>
                <MenuRadioItem value="title" onSelect={(event) => event.preventDefault()}>
                    Title
                </MenuRadioItem>
                <MenuRadioItem value="date" onSelect={(event) => event.preventDefault()}>
                    Date
                </MenuRadioItem>
            </MenuRadioGroup>
        </>
    ));
    open(container);
    for (const name of ["Show archived", "Date"]) {
        [...container.querySelectorAll<HTMLElement>("[data-menu-item]")]
            .find((item) => item.textContent === name)
            ?.click();
    }
    flush();
    const states = [...container.querySelectorAll("[aria-checked]")].map((item) =>
        item.getAttribute("aria-checked"),
    );

    expect({ states, isShown: isShown(), sort: sort() }).toEqual({
        states: ["true", "false", "true"],
        isShown: true,
        sort: "date",
    });
});
