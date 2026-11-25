import { none, principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import type { Environment } from "@destack/environment";
import { argument, plural, t } from "@destack/locale";
import { defineAction, defineObject, field, picked, Place, selected } from "@destack/object";
import { ObjectRemote } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
import { type JsonValue, schema } from "@destack/schema";
import { type ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { testWorld } from "../test/browser.ts";
import type {} from "@destack/package/import-meta";
import { flush } from "solid-js";
import { render } from "../test/index.ts";
import { ActionRegistry, type ActionQuestion } from "./action.ts";
import { useAction } from "./state.ts";

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

/** Cards of a board, each open or done. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: board,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string()).labeled(t`Title`),
            status: field.enum(["open", "done"]).default("open"),
        },
        access: { permissions: { read: relation("owner"), edit: relation("owner") } },
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
        delete: method.delete("edit"),
    }),
);

/** Finish the selected open cards. */
const finish = defineAction({
    name: "card.finish",
    title: t`Finish`,
    keybinding: "f",
    input: { cards: selected(card, { status: "open" }) },
    call: ({ cards }) => cards.prepare("update", { status: "done" }),
});

/** Put cards after another. */
const follow = defineAction({
    name: "card.follow",
    title: t`Put after…`,
    input: { cards: selected(card), after: picked(card) },
    call: ({ cards, after }) => cards.prepare("update", { title: after.title }),
});

/** Add a card to the board. */
const add = defineAction({
    name: "card.add",
    title: t`New card`,
    keybinding: "c",
    input: { title: field.string(schema.string()).labeled(t`Title`) },
    call: ({ title }) => card.prepare("create", { title }),
});

/** Set where cards stand. */
const mark = defineAction({
    name: "card.mark",
    title: t`Mark`,
    input: { cards: selected(card), state: field.enum(["open", "done"]) },
    call: ({ cards, state }) => cards.prepare("update", { status: state }),
});

/** Two boards. */
const BOARDS = [
    schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000001"),
    schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000002"),
] as const;

/** Open a board's cards offline, as a page keeps them, predicting every change locally. */
async function openBoard(environment: Environment, scope: string) {
    // keep the cards in a local database whose server never answers
    const storage = await TestDatabase.create(ObjectClient.tables({ card }), { environment });
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { card },
        scope,
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

    return client;
}

/** Open a page with two boards and some cards on them, answering questions from a list. */
async function openPage() {
    // open both boards, and attach them to the page's actions
    const { simulation } = testWorld();
    const environment = simulation.environment("view");
    const [first, second] = [
        await openBoard(environment, BOARDS[0]),
        await openBoard(environment, BOARDS[1]),
    ];
    const actions = new ActionRegistry();
    actions.attach({
        clients: new Map([
            [BOARDS[0], first],
            [BOARDS[1], second],
        ]),
        scope: BOARDS[0],
    });
    for (const action of [finish, follow, add]) {
        actions.offer(action);
    }

    // add cards, and read the cards of both boards by title
    const cards: ObjectReference[] = [];
    for (const [client, title] of [
        [first, "Apples"],
        [first, "Bread"],
        [second, "Cheese"],
    ] as const) {
        const { id } = await client.mutate(card).create({ title }).predicted;
        cards.push(card.policy.reference(client.scope, id));
    }
    const state = async () =>
        (
            await Promise.all(
                [first, second].map(async (client) => {
                    const live = client
                        .queryType(card)
                        .findMany({ orderBy: { title: "asc" } })
                        .subscribe();
                    const rows = await live.read();
                    await live.close();

                    return rows;
                }),
            )
        ).map((rows) => rows.map((row) => `${row.title} ${row.status}`));

    return { actions, cards, state };
}

test("call a method on the objects selected across spaces, and leave what it does not apply to", async () => {
    // select a card of each board in a part of the page
    const { actions, cards, state } = await openPage();
    const [apples, bread, cheese] = cards;
    const part = document.createElement("section");
    document.body.append(part);
    onTestFinished(() => part.remove());
    const several = [apples, cheese].filter((each) => each !== undefined);
    const stop = actions.provide(() => new Place().with(selected(card), several), part);

    // finish them by the key, then once more, and nowhere
    const closing = new AbortController();
    actions.listen(closing.signal);
    onTestFinished(() => closing.abort());
    const names = actions.actions(part).map(({ name }) => name);
    part.dispatchEvent(new KeyboardEvent("keydown", { key: "f", bubbles: true, cancelable: true }));
    await expect.poll(state).toEqual([["Apples done", "Bread open"], ["Cheese done"]]);
    const again = await actions.run(finish, part).catch((error: unknown) => String(error));
    const listed = (await actions.at(part)).map(({ name }) => name);
    stop();

    expect({
        names,
        listed,
        again,
        nowhere: actions.actions(part).map(({ name }) => name),
        bread: bread?.scope,
    }).toEqual({
        names: ["card.finish", "card.follow", "card.add"],
        // the finished cards offer no finish, their own methods beside the declared action
        listed: ["card.add", "card.follow", "card.update", "card.delete"],
        again: `FamilyError: cannot call 'card.update': it does not apply to card '${apples?.id}'`,
        nowhere: ["card.add"],
        bread: BOARDS[0],
    });
});

test("ask for what neither places nor what is given answer, and run nothing once cancelled", async () => {
    // answer each question in turn, cancelling the first
    const { actions, cards, state } = await openPage();
    const [apples, bread] = cards;
    actions.provide(() => Place.current(apples ?? card.policy.reference("", "")));
    const asked: string[][] = [];
    const answers = [undefined, { after: bread ?? null }, { title: "Dates" }];
    actions.asks(async ({ action, asked: inputs }: ActionQuestion) => {
        asked.push([action.name, ...inputs.map(({ name }) => name)]);

        return answers.shift();
    });

    // put the page's card after another twice, then add one, asked and given
    await actions.run(follow, null);
    const cancelled = await state();
    await actions.run(follow, null);
    await actions.run(add, null);
    await actions.run(add, null, { title: "Eggs" });

    expect({ asked, cancelled, state: await state() }).toEqual({
        asked: [
            ["card.follow", "after"],
            ["card.follow", "after"],
            ["card.add", "title"],
        ],
        cancelled: [["Apples open", "Bread open"], ["Cheese open"]],
        state: [["Bread open", "Bread open", "Dates open", "Eggs open"], ["Cheese open"]],
    });
});

test("confirm before removing for good, and refuse to where no part of the page asks", async () => {
    // remove the page's card by its own method, first with nobody to ask
    const { actions, cards, state } = await openPage();
    const [apples] = cards;
    actions.provide(() => Place.current(apples ?? card.policy.reference("", "")));
    const remove = (await actions.at(null)).find(({ name }) => name === "card.delete");
    if (remove === undefined) {
        throw new TypeError("the page offers no delete");
    }
    const refused = await actions.run(remove, null).catch((error: unknown) => String(error));
    actions.asks(async () => ({}));
    await actions.run(remove, null);

    expect({ refused, state: await state() }).toEqual({
        refused: `TypeError: action '${card.package.id}/card.delete' removes for good, and no part of the page asks to confirm it`,
        state: [["Bread open"], ["Cheese open"]],
    });
});

test("follow the value every target has, as its rows change", async () => {
    // select a card of each board, and follow where they stand
    const { actions, cards } = await openPage();
    const [apples, , cheese] = cards;
    const several = [apples, cheese].filter((each) => each !== undefined);
    actions.provide(() => Place.selected(several));
    const [input] = actions.asked(mark, null);
    const told: (readonly JsonValue[])[] = [];
    actions.offer(mark);
    const { container } = render(
        () => {
            const values = useAction(() => ({ action: mark, element: null })).chosen;

            return <output>{JSON.stringify(values())}</output>;
        },
        { actions },
    );
    const settle = async (expected: readonly JsonValue[]) => {
        await expect
            .poll(() => {
                flush();

                return container.textContent;
            })
            .toBe(JSON.stringify(expected));
        told.push(expected);
    };
    await settle(["open"]);

    // finish one, then both
    actions.provide(() => Place.selected(several.slice(0, 1)));
    await actions.run(finish, null);
    await settle([]);
    actions.provide(() => Place.selected(several));
    await actions.run(mark, null, { state: "done" });
    await settle(["done"]);

    expect({ names: input?.name, told }).toEqual({
        names: "state",
        told: [["open"], [], ["done"]],
    });
});
