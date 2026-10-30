import { testWorld } from "../test/browser.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { none, principal, relation } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { defineObject, field } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { renderView } from "../page/mount.ts";
import { Errored, For, Loading } from "../solid/flow.ts";
import { useQuery, useSetting, useSpace } from "./index.ts";
import { defineSetting } from "@destack/setting/declare";
import { deviceSetting, setting } from "@destack/setting/object";
import { changed } from "../test/dom.ts";
import type { Environment } from "@destack/environment";

/** A page origin that calls nothing, as a test page has none. */
function noOrigin(path: string): never {
    throw new TypeError(`a test page reaches no ${path}`);
}

/** The longest a test waits for a render, far above the few microtasks a local resource takes. */
const RENDER_MILLISECONDS = 2000;

/** A board with cards. */
const board = defineObject({
    name: "board",
    plural: "boards",
    scope: Scope.universe.id,
    isScope: true,
    fields: {},
    permissions: { read: none() },
});

/** A card on a board. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: board,
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        permissions: { read: relation("owner"), edit: relation("owner") },
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
);

/** The board the view opens. */
const BOARD_ID = schema.identifier("board").parse("board-01996ab0-0000-7000-8000-000000000001");

/** The view the cards open in, on the board's space. */
/** The view context the tests mount. */
const CONTEXT = {
    installation: "installation-cards",
    space: BOARD_ID,
    account: "account-01996ab0-0000-7000-8000-000000000002",
    view: "cards",
    user: principal.user.reference(Scope.universe.id, "user-01996ab0-0000-7000-8000-000000000003"),
};

/** Open a board's cards offline, as a view's page keeps them, predicting every change locally. */
async function openClient(
    environment: Environment,
): Promise<ObjectClient<{ readonly card: typeof card }>> {
    // keep the cards in a local database whose server never answers
    const storage = await TestDatabase.create(ObjectClient.tables({ card }), {
        environment,
    });
    const offline = { url: "https://cards.test", fetch: () => new Promise<Response>(() => {}) };
    const client = await ObjectClient.open({
        environment,
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
    const { simulation, tab } = testWorld();
    const client = await openClient(simulation.environment("view"));
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
            {
                context: CONTEXT,
                clients: new Map([[BOARD_ID, client]]),
                packages: new Map(),
                origin: noOrigin,
            },
            Localization.of(SOURCE_LOCALE, []),
            tab,
        ),
    );
    await changed(element, () => element.textContent === "ApplesBread", RENDER_MILLISECONDS);
    const [apples, bread] = element.querySelectorAll("li");

    // rename the first card and keep the second card's element
    if (apples === undefined) {
        throw new TypeError("missing first card");
    }
    apples.click();
    await changed(element, () => element.textContent === "Apples!Bread", RENDER_MILLISECONDS);
    expect(element.querySelectorAll("li")[1]).toBe(bread);
});

test("fail access to an object type the view's space does not hold into the error boundary", async () => {
    // render a query of boards, which live outside every scope of the view
    const { simulation, tab } = testWorld();
    const client = await openClient(simulation.environment("view"));
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
            {
                context: CONTEXT,
                clients: new Map([[BOARD_ID, client]]),
                packages: new Map(),
                origin: noOrigin,
            },
            Localization.of(SOURCE_LOCALE, []),
            tab,
        ),
    );
    expect(element.textContent).toBe("the client has no object board");
});

/** An editor mode the person sets, which a device may override. */
const editor = defineSetting({
    name: "editor.mode",
    title: "Editor mode",
    description: "Keyboard behavior in the editor.",
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["device"],
    apply: "immediate",
});

/** The device the settings page runs on. */
const DEVICE_ID = schema.identifier("device").parse("device-01996ab0-0000-7000-8000-000000000004");

/** Show the person's editor mode. */
function EditorMode() {
    const mode = useSetting(editor);

    return <p>{mode()}</p>;
}

test("show a setting's value for the page's person, and the device's value once it stores one", async () => {
    // keep the person's settings and the device's own offline
    const { simulation, tab } = testWorld();
    const storage = await TestDatabase.create(ObjectClient.tables({ setting, deviceSetting }), {
        environment: simulation.environment("view"),
    });
    const offline = { url: "https://settings.test", fetch: () => new Promise<Response>(() => {}) };
    const client = await ObjectClient.open({
        environment: simulation.environment("view"),
        database: storage.database,
        objects: { setting, deviceSetting },
        scope: CONTEXT.user.id,
        caller: CONTEXT.user,
        device: principal.device.reference(Scope.universe.id, DEVICE_ID),
        endpoint: offline,
        reconnect: () => offline,
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });

    // show the default, then the value the device stores
    const element = document.createElement("main");
    onTestFinished(
        renderView(
            () => <EditorMode />,
            element,
            {
                context: {
                    ...CONTEXT,
                    space: "space-01996ab0-0000-7000-8000-000000000005",
                    installation: "installation-01996ab0-0000-7000-8000-000000000006",
                    device: DEVICE_ID,
                },
                clients: new Map([[CONTEXT.user.id, client]]),
                packages: new Map(),
                origin: noOrigin,
            },
            Localization.of(SOURCE_LOCALE, []),
            tab,
        ),
    );
    await changed(element, () => element.textContent === "standard", RENDER_MILLISECONDS);
    await client.mutate(deviceSetting).create({
        packageId: editor.reference.packageId,
        name: editor.reference.name,
        value: "vim",
        release: editor.package.version,
    }).predicted;
    await changed(element, () => element.textContent === "vim", RENDER_MILLISECONDS);
});

/** A space's language, which the space's members share rather than set for themselves. */
const language = defineSetting({
    name: "space.language",
    title: "Language",
    description: "The space's language.",
    schema: schema.string(),
    default: "en",
    scope: "space",
    overrides: [],
    apply: "immediate",
});

test("refuse following a space's setting as the page's person into the error boundary", async () => {
    // render a space setting through the person's settings
    const { simulation, tab } = testWorld();
    const client = await openClient(simulation.environment("view"));
    const element = document.createElement("main");
    onTestFinished(
        renderView(
            () => (
                <Errored fallback={(error) => <p>{failureMessage(error())}</p>}>
                    {(() => {
                        useSetting(language);

                        return <p>Language</p>;
                    })()}
                </Errored>
            ),
            element,
            {
                context: CONTEXT,
                clients: new Map([[BOARD_ID, client]]),
                packages: new Map(),
                origin: noOrigin,
            },
            Localization.of(SOURCE_LOCALE, []),
            tab,
        ),
    );
    expect(element.textContent).toBe(
        "useSetting only reads the person's settings, not space settings: space.language",
    );
});
