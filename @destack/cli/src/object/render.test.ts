import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, presentable } from "@destack/object";
import { expect, test } from "@destack/test";
import { Render } from "./render.ts";

/** A card titled by its headline, with fields that sort before it. */
const card = defineObject({
    name: "card",
    plural: "cards",
    title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
    scope: space,
    fields: {
        /** The card's headline. */
        headline: field.string(),
        /** The card's author. */
        author: field.string(),
        /** The card's body. */
        body: field.string(),
    },
    traits: [presentable({ title: "headline" })],
});

test("list a type's identifier and title first, then its other fields", () => {
    expect(Render.columns(card, undefined)).toEqual(["id", "headline", "author", "body", "color"]);
});
