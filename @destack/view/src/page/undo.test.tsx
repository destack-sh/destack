import { testWorld } from "../test/browser.ts";
import { argument, plural, t } from "@destack/locale";
import { none, principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { defineObject, field } from "@destack/object";
import { ObjectRemote } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import type { Environment } from "@destack/environment";
import { PageUndo } from "./undo.ts";

/** A board with cards. */
const board = defineObject({
    name: "board",
    plural: "boards",
    title: t`${plural(argument("count"), { one: "Board", other: "Boards" })}`,
    scope: Scope.universe.id,
    fields: {},
    container: "scope",
    access: {
        permissions: { read: none() },
    },
});

/** A card on a board. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: board,
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        access: {
            permissions: { read: relation("owner"), edit: relation("owner") },
        },
    },
    (method) => ({
        create: method.create("edit"),
        update: method.update("edit"),
    }),
);

/** The board the cards are on. */
const BOARD_ID = schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000001");

/** The person editing the cards. */
const USER = principal.user.reference(
    Scope.universe.id,
    "user-01996ab0-0000-7000-8000-000000000003",
);

/** Open a board's cards offline, predicting every change locally. */
async function openClient(environment: Environment) {
    // keep the cards in a local database whose server never answers
    const storage = await TestDatabase.create(ObjectClient.tables({ card }), { environment });
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { card },
        scope: BOARD_ID,
        caller: USER,
        remote: ObjectRemote.unanswered(),
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });

    return client;
}

test("undo the newest group across a page's stacks, apart at gestures, redo the last undone, and follow what it can undo and redo", async () => {
    // open two scopes' clients and close their groups at the page's gestures
    const { simulation, tab } = testWorld();
    const [first, second] = [await openClient(tab), await openClient(tab)];
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    const undo = new PageUndo([first.undoStack, second.undoStack]);
    undo.listen(stopping.signal);
    const titles = async () =>
        Promise.all(
            [first, second].map(async (client) =>
                (await client.database.select({ title: card.table.title }).from(card.table)).map(
                    (row) => row.title,
                ),
            ),
        );

    // create a card in each scope, retitle the first's, then the second's twice a gesture apart
    const { id: apple } = await first.mutate(card).create({ title: "a" }).predicted;
    const { id: bread } = await second.mutate(card).create({ title: "b" }).predicted;
    await simulation.jump(tab.clock.now() + 1000);
    await first.mutate(card).update({ id: apple, title: "a1" }).predicted;
    await simulation.jump(tab.clock.now() + 1000);
    await second.mutate(card).update({ id: bread, title: "b1" }).predicted;
    document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    await second.mutate(card).update({ id: bread, title: "b2" }).predicted;

    // follow that the page can undo, and not yet redo
    const can = () => ({ undo: undo.canUndo(), redo: undo.canRedo() });
    await expect.poll(can).toEqual({ undo: true, redo: false });

    // undo the newest group of either scope each time, then redo the last undone
    const steps = [
        { step: () => undo.undo(), titles: [["a1"], ["b1"]] },
        { step: () => undo.undo(), titles: [["a1"], ["b"]] },
        { step: () => undo.undo(), titles: [["a"], ["b"]] },
        { step: () => undo.redo(), titles: [["a1"], ["b"]] },
    ];
    const done = [];
    for (const { step } of steps) {
        done.push(await step());
        done.push(await titles());
    }
    expect(done).toEqual(steps.flatMap(({ titles: expected }) => [true, expected]));
    await expect.poll(can).toEqual({ undo: true, redo: true });
});
