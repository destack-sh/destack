import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { Card, CardContent, CardHeader, CardTitle, type CardSize } from "./index.ts";

test("pad a card and its parts by its size, following a change of size", () => {
    // a card that turns small, beside one small from the start
    const [size, setSize] = createSignal<CardSize>("default");
    const { container } = render(() => (
        <>
            <Card size={size()}>
                <CardHeader>
                    <CardTitle>Storage</CardTitle>
                </CardHeader>
                <CardContent>2 of 5 GB</CardContent>
            </Card>
            <Card size="sm">
                <CardHeader>
                    <CardTitle>Storage</CardTitle>
                </CardHeader>
                <CardContent>2 of 5 GB</CardContent>
            </Card>
        </>
    ));
    const [changing, small] = [...container.querySelectorAll<HTMLElement>("[data-slot=card]")];
    const before = classesOf(changing);
    setSize("sm");
    flush();

    // the card marks its size, and once small its parts take exactly a small card's classes
    expect({
        sizes: [changing?.dataset["size"], small?.dataset["size"]],
        isChanged: classesOf(changing).join() !== before.join(),
        after: classesOf(changing),
    }).toEqual({ sizes: ["sm", "sm"], isChanged: true, after: classesOf(small) });
});

/** List the classes of a card and each of its parts. */
function classesOf(card: HTMLElement | undefined): string[] {
    return [card?.className ?? "", ...[...(card?.children ?? [])].map((part) => part.className)];
}
