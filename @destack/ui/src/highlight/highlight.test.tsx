import { Search } from "@destack/db";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import { AutocompleteContext, AutocompleteControl } from "../autocomplete/index.ts";
import { Highlight } from "./index.ts";

test("mark what its search or the nearest list's matches", () => {
    // mark by each, then as the list's search changes
    const list = new AutocompleteControl({});
    const { container } = render(() => (
        <>
            <Highlight text="Change status" search={Search.of("sta cha")} />
            <Highlight text="Order form" search={Search.of("harbour order")} />
            <Highlight text="Change status" />
            <AutocompleteContext value={list}>
                <Highlight text="Change status" />
            </AutocompleteContext>
        </>
    ));
    flush();
    const before = markup(container);
    list.type("stat");
    flush();

    expect({ before, after: markup(container).slice(before.lastIndexOf("<span")) }).toEqual({
        before: [
            '<span data-slot="highlight"><mark>Cha</mark>nge <mark>sta</mark>tus</span>',
            '<span data-slot="highlight"><mark>Order</mark> form</span>',
            '<span data-slot="highlight">Change status</span>',
            '<span data-slot="highlight">Change status</span>',
        ].join(""),
        after: '<span data-slot="highlight">Change <mark>stat</mark>us</span>',
    });
});
