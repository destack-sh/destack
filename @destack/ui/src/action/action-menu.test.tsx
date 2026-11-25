import { t } from "@destack/locale";
import { defineAction, field } from "@destack/object";
import { expect, test } from "@destack/test";
import { ActionRegistry, flush, handleOnPage } from "@destack/view";
import { render, stubPopovers, wait } from "@destack/view/test";
import {
    duplicateItems,
    items,
    moveItemsDown,
    moveItemsUp,
    openItem,
    openMenu,
    provideItems,
    removeItems,
    ActionMenu,
} from "./index.ts";

/** Set the color of notes. */
const paint = defineAction({
    name: "paint",
    title: t`Paint`,
    input: { items, color: field.enum(["red", "blue"]).labeled(t`Color`) },
});

/** Add a note, on nothing. */
const add = defineAction({ name: "add", title: t`Add note` });

/** Draw two notes with a page menu, recording what runs. */
function drawNotes() {
    stubPopovers();
    const ran: unknown[] = [];
    const actions = new ActionRegistry();
    const record = (name: string) => ({
        run: ({ values }: { readonly values: unknown }) => void ran.push([name, values]),
    });
    const { container } = render(
        () => {
            handleOnPage(removeItems, record("remove"));
            handleOnPage(moveItemsUp, record("up"));
            handleOnPage(moveItemsDown, record("down"));
            handleOnPage(paint, record("paint"));
            handleOnPage(duplicateItems, record("duplicate"));
            handleOnPage(openItem, record("open"));
            handleOnPage(add, record("add"));

            return (
                <main>
                    <ul
                        ref={provideItems({
                            targets: (element) => {
                                const key = element?.closest("li")?.dataset["key"];

                                return key === undefined ? [] : [key];
                            },
                        })}
                    >
                        <li data-key="a" tabindex="0">
                            Apples <input />
                        </li>
                        <li data-key="b" tabindex="0">
                            Bread
                        </li>
                    </ul>
                    <ActionMenu />
                </main>
            );
        },
        { actions },
    );
    flush();

    return { container, ran, actions };
}

/** Ask for the menu at an element, reporting whether the page took it from the browser. */
function askAt(element: Element | null | undefined): boolean {
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    element?.dispatchEvent(event);
    flush();

    return event.defaultPrevented;
}

/** List the titles of the open menu's items, the submenu's after it. */
function listed(): (string | undefined)[] {
    return [...document.querySelectorAll("[data-menu-item]")].map(
        (item) => item.firstChild?.textContent ?? undefined,
    );
}

test("list the actions on the item a person asks at, grouped, and run one on it", () => {
    // ask at the second note
    const { container, ran } = drawNotes();
    const isTaken = askAt(container.querySelector("[data-key=b]"));
    const titles = listed();
    const duplicate = [...document.querySelectorAll<HTMLElement>("[data-menu-item]")].find((item) =>
        item.textContent.startsWith("Duplicate"),
    );
    duplicate?.click();
    flush();

    expect({ isTaken, titles, ran }).toEqual({
        isTaken: true,
        titles: ["Open", "Paint", "Duplicate", "Move", "Move up", "Move down", "Delete"],
        ran: [["duplicate", { items: ["b"] }]],
    });
});

test("leave the browser its menu in fields and where no action acts on anything", () => {
    const { container } = drawNotes();

    expect({
        field: askAt(container.querySelector("input")),
        page: askAt(container.querySelector("main")),
    }).toEqual({ field: false, page: false });
});

test("open under what the person is on by the action", async () => {
    // run the action at the first note
    const { container, actions, ran } = drawNotes();
    const first = container.querySelector("[data-key=a]");
    await actions.run(openMenu, first);
    flush();
    const titles = listed();
    document.querySelector<HTMLElement>("[data-menu-item]")?.click();
    await wait(0);

    expect({ titles: titles[0], ran }).toEqual({
        titles: "Open",
        ran: [["open", { items: ["a"] }]],
    });
});
