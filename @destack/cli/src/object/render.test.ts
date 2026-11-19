import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, presentable } from "@destack/object";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
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

test("list a type's title first, then its other fields, its identifier last", () => {
    expect(Render.columns(card, { kind: "table" }, undefined)).toEqual([
        "headline",
        "author",
        "body",
        "id",
    ]);
});

/** Two cards as the space returns them. */
const cards = [
    { id: "card-1", headline: "Launch", author: "ada", body: "Write the post", color: null },
    { id: "card-2", headline: "Intel start", author: "grace", body: "Fix it", color: "red" },
];

test("show a person a table with the identifiers last and the rows counted", () => {
    expect(
        Render.table(
            cards,
            Render.columns(card, { kind: "table" }, undefined),
            card,
            TerminalFixture.terminal(true),
        ),
    ).toBe(
        [
            "  HEADLINE     AUTHOR  BODY            ID",
            "  Launch       ada     Write the post  card-1",
            "  Intel start  grace   Fix it          card-2",
            "",
            "  2 cards",
        ].join("\n"),
    );
});

test("show a person one object under its title and identifier, and a program every field", () => {
    const [launch] = cards;
    expect({
        person: Render.object(launch ?? {}, card, TerminalFixture.terminal(true)),
        program: Render.object(launch ?? {}, card, TerminalFixture.terminal(false)),
    }).toEqual({
        person: [
            "  Launch  card-1",
            "",
            "  author  ada",
            "  body    Write the post",
            "  color   -",
        ].join("\n"),
        program: "id\tcard-1\nheadline\tLaunch\nauthor\tada\nbody\tWrite the post\ncolor\t-",
    });
});
