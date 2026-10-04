import { none, principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { defineObject, field } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { renderView } from "../page/mount.ts";
import { Errored, For, Loading } from "../solid/flow.ts";
import { useQuery, useSpace } from "./index.ts";

/** A board holding cards. */
const board = defineObject({
    name: "board",
    plural: "boards",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
});

/** A card on a board. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: board,
    fields: { owner: field.reference(principal.user).caller(), title: field.string() },
    permissions: { read: relation("owner"), edit: relation("owner") },
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
});

/** The board the view opens. */
const BOARD_ID = schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000001");

/** The view the cards open in, on the board's space. */
const CONTEXT = {
    installation: "installation-cards",
    space: BOARD_ID,
    account: "account-01996ab0-0000-7000-8000-000000000002",
    view: "cards",
    user: principal.user.reference(Scope.universe.id, "alice"),
};

/** Open a board's cards offline, as a view's page holds them, predicting every change locally. */
async function openClient(): Promise<ObjectClient<{ readonly card: typeof card }>> {
    // keep the cards in a local database whose server never answers
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ card }));
    const offline = { url: "https://cards.test", fetch: () => new Promise<Response>(() => {}) };
    const client = await ObjectClient.open({
        database: storage.database,
        objects: { card },
        scope: BOARD_ID,
        caller: CONTEXT.user,
        endpoint: offline,
        reconnect: () => offline,
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });

    return client;
}

/** List the board's cards by title, renaming one through the view's mutations. */
function Cards() {
    const space = useSpace({ card });
    const cards = useQuery(space.query.card.findMany({ orderBy: { title: "asc" } }));
    const update = space.mutate.card.update;

    return (
        <ul>
            <For each={cards()}>
                {(entry) => (
                    <li onClick={() => update({ id: entry.id, title: `${entry.title}!` })}>
                        {entry.title}
                    </li>
                )}
            </For>
        </ul>
    );
}

/** Describe a failure by its message, refusing values that are not errors. */
function failureMessage(failure: unknown): string {
    if (!(failure instanceof Error)) {
        throw new TypeError("expected an error");
    }

    return failure.message;
}

test("show a query's rows after loading, and re-render only the row a predicted change renames", async () => {
    // render two predicted cards under a loading boundary
    const client = await openClient();
    const created = client.mutation(async (mutation) => {
        await mutation.call(card).create({ title: "Apples" });
        await mutation.call(card).create({ title: "Bread" });
    });
    await created.predicted;
    const element = document.createElement("main");
    onTestFinished(
        renderView(
            () => (
                <Loading fallback={<p>Loading</p>}>
                    <Cards />
                </Loading>
            ),
            element,
            CONTEXT,
            { [BOARD_ID]: client },
            [],
        ),
    );
    await expect.poll(() => element.textContent).toBe("ApplesBread");
    const [apples, bread] = element.querySelectorAll("li");

    // rename the first card and keep the second card's element
    if (apples === undefined) {
        throw new TypeError("missing first card");
    }
    apples.click();
    await expect.poll(() => element.textContent).toBe("Apples!Bread");
    expect(element.querySelectorAll("li")[1]).toBe(bread);
});

test("fail access to an object type the view's space does not hold into the error boundary", async () => {
    // render a query of boards, which live outside every scope of the view
    const client = await openClient();
    const element = document.createElement("main");
    onTestFinished(
        renderView(
            () => (
                <Errored fallback={(error) => <p>{failureMessage(error())}</p>}>
                    {(() => {
                        useQuery(useSpace({ board }).query.board.findMany());

                        return <p>Boards</p>;
                    })()}
                </Errored>
            ),
            element,
            CONTEXT,
            { [BOARD_ID]: client },
            [],
        ),
    );
    expect(element.textContent).toBe("the client has no object board");
});
