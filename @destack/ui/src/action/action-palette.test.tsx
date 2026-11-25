import { none, principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { argument, plural, t } from "@destack/locale";
import { defineAction, defineObject, field, searchable } from "@destack/object";
import { ObjectClient, ObjectRemote } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { ActionRegistry, flush, handle, handleOnPage, palette } from "@destack/view";
import { render, stubPopovers, testWorld, wait } from "@destack/view/test";
import { ActionPalette, selectAll } from "./index.ts";

/** Bold the text of an editor. */
const bold = defineAction({
    name: "bold",
    title: t`Bold`,
    category: t`Format`,
    keybinding: "mod+b",
});

/** Write a note. */
const write = defineAction({ name: "write", title: t`Write` });

/** A board with cards. */
const board = defineObject({
    name: "board",
    plural: "boards",
    title: t`${plural(argument("count"), { one: "Board", other: "Boards" })}`,
    scope: Scope.universe.id,
    fields: {},
    container: "scope",
    access: { permissions: { read: none() } },
});

/** Cards found by their title. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: board,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string()),
        },
        access: { permissions: { read: relation("owner"), edit: relation("owner") } },
        traits: [searchable({ text: ["title"] })],
    },
    (method) => ({ list: method.list("read"), create: method.create("edit") }),
);

/** A board. */
const BOARD = schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000001");

test("list what runs where a person is, before what they open, and run it there", async () => {
    // open the palette in the editor, and run its first entry
    stubPopovers();
    const ran: unknown[] = [];
    const actions = new ActionRegistry();
    const open = { id: "open", title: "Open", run: () => void ran.push("open") };
    const { container } = render(
        () => {
            handleOnPage(selectAll, { run: () => void ran.push("select") });
            handleOnPage(write, { run: () => void ran.push("write") });

            return (
                <>
                    <textarea
                        ref={handle(bold, {
                            run: ({ element }) => void ran.push(element?.tagName),
                        })}
                    />
                    <ActionPalette
                        id="test"
                        title="Notes"
                        groups={[
                            {
                                id: "notes",
                                title: "Notes",
                                items: () => [{ id: "lisbon", title: "Lisbon", actions: [open] }],
                            },
                        ]}
                    />
                </>
            );
        },
        { actions },
    );
    flush();
    await actions.run(palette, container.querySelector("textarea"));
    flush();
    await wait(0);
    flush();
    const items = [...document.querySelectorAll<HTMLElement>("[data-slot=command-item]")];
    items[0]?.click();
    await wait(0);

    expect({ items: items.map((item) => item.textContent), ran }).toEqual({
        items: ["BoldFormatCtrlB", "Write", "Select allEditCtrlA", "Lisbon"],
        ran: ["TEXTAREA"],
    });
});

/** List the entries the palette shows. */
function shownItems(): HTMLElement[] {
    return [...document.querySelectorAll<HTMLElement>("[data-slot=command-item]")];
}

/** List the titles of the groups the palette shows. */
function shownHeadings(): (string | null)[] {
    return [...document.querySelectorAll("[data-slot=command-group-heading]")].map(
        (heading) => heading.textContent,
    );
}

test("find objects as a person types, the best first, and open the one picked", async () => {
    // keep some cards on a device whose server never answers
    stubPopovers();
    const environment = testWorld().simulation.environment("view");
    const storage = await TestDatabase.create(ObjectClient.tables({ card }), { environment });
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { card },
        scope: BOARD,
        caller: principal.user.reference(
            Scope.universe.id,
            "user-01996ab0-0000-7000-8000-000000000003",
        ),
        remote: ObjectRemote.unanswered(),
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });
    for (const title of ["The order book", "Margins", "Order form"]) {
        await client.mutate(card).create({ title }).predicted;
    }

    // open the palette and type
    const opened: string[] = [];
    const actions = new ActionRegistry();
    actions.attach({ clients: new Map([[BOARD, client]]), scope: BOARD });
    render(
        () => (
            <ActionPalette
                id="found"
                title="Cards"
                objects={{ types: [card], onOpen: ({ type, scope }) => opened.push(type, scope) }}
            />
        ),
        { actions },
    );
    flush();
    await actions.run(palette, null);
    flush();
    const input = document.querySelector<HTMLInputElement>(
        "[data-slot=command-input] input, input",
    );
    if (input === null) {
        throw new TypeError("the palette has no search");
    }
    input.value = "ord";
    input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    await expect
        .poll(() => {
            flush();

            return shownItems().map((item) => item.textContent);
        })
        .toEqual(["Order form", "The order book"]);
    const marks = shownItems().map((item) => item.querySelector("mark")?.textContent);

    // type a word nothing has, found as it was meant
    input.value = "odrer";
    input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    await expect
        .poll(() => {
            flush();

            return shownItems().length;
        })
        .toBe(0);
    await testWorld().simulation.sleep(200);
    await expect
        .poll(() => {
            flush();

            return shownHeadings();
        })
        .toEqual(["Results for “\u2068order\u2069”"]);
    const meant = shownItems().map((item) => item.querySelector("mark")?.textContent);
    shownItems()[0]?.click();
    await wait(0);

    expect({ marks, meant, opened }).toEqual({
        marks: ["Ord", "ord"],
        meant: ["Order", "order"],
        opened: ["card", BOARD],
    });
});
