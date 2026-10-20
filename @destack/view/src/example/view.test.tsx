import { principal, relation } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { onTestFinished, test } from "@destack/test";
import { defineView } from "../declare/view.ts";
import { useQuery, useSpace } from "../scope/index.ts";
import { For } from "../solid/flow.ts";
import { renderExample } from "./frame.ts";
import { changed } from "../test/dom.ts";

/** The longest a test waits for a render, far above the few microtasks a local resource takes. */
const RENDER_MILLISECONDS = 2000;

/** A card in a space. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string().min(1)),
        },
        permissions: { read: relation("owner"), edit: relation("owner") },
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("edit"),
    }),
);

/** List the space's cards by title. */
function Cards() {
    const cards = useQuery(useSpace({ card }).query.card.findMany({ orderBy: { title: "asc" } }));

    return (
        <ul>
            <For each={cards()}>{(entry) => <li>{entry.title}</li>}</For>
        </ul>
    );
}

/** The cards of a space. */
const cards = defineView({
    name: "cards",
    objects: [card],
    component: async () => ({ default: Cards }),
});

/** The view holding two cards. */
export const cardsTwoCards = defineExample({
    of: cards,
    name: "two-cards",
    description: "the view holding two cards",
    objects: {
        space: [
            { object: card, method: "create", input: { title: "Bread" } },
            { object: card, method: "create", input: { title: "Apples" } },
        ],
    },
});

test("render a view's example over the objects its declared calls bring about", async () => {
    // render the example of the view holding two cards
    const element = document.createElement("main");
    onTestFinished(renderExample(element, { example: cardsTwoCards }));

    // list the cards in the view's order
    await changed(element, () => element.textContent === "ApplesBread", RENDER_MILLISECONDS);
});
