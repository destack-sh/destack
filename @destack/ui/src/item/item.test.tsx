import * as style from "@destack/style";
import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Item, ItemContent, ItemGroup, ItemTitle, itemStyle } from "./index.ts";

test("list a group's items and leave a lone item without a role", () => {
    const { container } = render(() => (
        <>
            <ItemGroup aria-label="Recent notes">
                <Item>
                    <ItemContent>
                        <ItemTitle>Groceries</ItemTitle>
                    </ItemContent>
                </Item>
            </ItemGroup>
            <Item size="sm">Trip</Item>
        </>
    ));
    expect(markup(container)).toBe(
        '<div data-slot="item-group" role="list" aria-label="Recent notes">' +
            '<div data-slot="item" data-variant="default" data-size="default" role="listitem">' +
            '<div data-slot="item-content"><div data-slot="item-title">Groceries</div></div></div></div>' +
            '<div data-slot="item" data-variant="default" data-size="sm">Trip</div>',
    );
});

test("style a link like an item, adding a hover background", () => {
    const { container } = render(() => (
        <>
            <Item variant="muted" />
            <a href="/notes/trip" {...style.attrs(itemStyle({ variant: "muted" }))} />
        </>
    ));
    const [item, link] = [...container.children];
    expect(link?.className).not.toBe(item?.className);
});
