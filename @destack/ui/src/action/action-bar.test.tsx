import { expect, test } from "@destack/test";
import { ActionRegistry, createSignal, flush, handleOnPage } from "@destack/view";
import { markup, render, stubPopovers, wait } from "@destack/view/test";
import { ActionButton, removeItems } from "./index.ts";
import { ActionBar } from "./index.ts";

test("show the count while items are selected, and clear them", () => {
    // select two, then clear them
    stubPopovers();
    const [items, setItems] = createSignal<readonly string[]>([]);
    const { container } = render(() => <ActionBar items={items()} onClear={() => setItems([])} />);
    flush();
    const before = markup(container);
    setItems(["a", "b"]);
    flush();
    const shown = container.querySelector("[data-slot=action-bar]");
    const told = {
        role: shown?.getAttribute("role"),
        name: shown?.getAttribute("aria-label"),
        placement: shown?.getAttribute("data-placement"),
        count: shown?.querySelector("[data-slot=action-bar-count]")?.textContent,
    };
    const clear = shown?.querySelector<HTMLElement>("[data-slot=action-bar-clear]");
    const label = clear?.getAttribute("aria-label");
    clear?.click();
    flush();

    expect({ before, told, label, after: markup(container) }).toEqual({
        before: "",
        told: {
            role: "group",
            name: "Actions",
            placement: "floating",
            count: "⁨2⁩ selected",
        },
        label: "Clear selection",
        after: "",
    });
});

test("act on the selected items", async () => {
    // remove them by a button of the bar
    stubPopovers();
    const ran: unknown[] = [];
    const actions = new ActionRegistry();
    const { container } = render(
        () => {
            handleOnPage(removeItems, { run: ({ values }) => void ran.push(values) });

            return (
                <ActionBar items={["a", "b"]}>
                    <ActionButton action={removeItems} />
                </ActionBar>
            );
        },
        { actions },
    );
    flush();
    container.querySelector("button")?.click();
    await wait(0);

    expect(ran).toEqual([{ items: ["a", "b"] }]);
});
