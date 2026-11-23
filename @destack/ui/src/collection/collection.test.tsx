import { expect, onTestFinished, test } from "@destack/test";
import { createEffect, createMemo, createRoot, createSignal, flush, Show } from "@destack/view";
import { render, wait } from "@destack/view/test";
import { Collection, CollectionBuilder, type CollectionSection, Load } from "./collection.ts";

/** A note of a test collection. */
interface Note {
    /** The note's id. */
    readonly id: string;
    /** The note's title. */
    readonly title: string;
    /** Whether the note is archived and unavailable. */
    readonly isArchived?: boolean;
}

/** The sections of a test collection: two pinned notes and two recent ones, one archived. */
const SECTIONS: readonly CollectionSection<Note>[] = [
    {
        key: "pinned",
        label: "Pinned",
        items: [
            { id: "groceries", title: "Groceries" },
            { id: "lisbon", title: "Trip to Lisbon" },
        ],
    },
    {
        key: "recent",
        items: [
            { id: "plans", title: "Plans" },
            { id: "taxes", title: "Taxes", isArchived: true },
        ],
    },
];

/** Run a function inside a reactive root disposed after the test. */
function rooted<Value>(run: () => Value): Value {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return run();
    });
}

/** Create a collection of notes from sections and a load, disposed after the test. */
function createNotes(
    sections: () => readonly CollectionSection<Note>[],
    load?: Load<unknown>,
): Collection<Note> {
    return rooted(
        () =>
            new Collection<Note>({
                sections,
                key: (note) => note.id,
                text: (note) => note.title,
                isDisabled: (note) => note.isArchived === true,
                ...(load === undefined ? {} : { load }),
            }),
    );
}

test("number the items of every section by key and place, reading each one's text and availability", () => {
    const notes = createNotes(() => SECTIONS);

    expect({
        state: notes.state(),
        keys: notes.keys(),
        size: notes.size(),
        places: [notes.index("groceries"), notes.index("taxes"), notes.index("missing")],
        at: [notes.at(2), notes.at(4)],
        item: notes.item("lisbon"),
        texts: [notes.text("plans"), notes.text("missing")],
        has: [notes.has("plans"), notes.has("missing")],
        disabled: [notes.isDisabled("plans"), notes.isDisabled("taxes"), notes.isDisabled("x")],
    }).toEqual({
        state: "loaded",
        keys: ["groceries", "lisbon", "plans", "taxes"],
        size: 4,
        places: [0, 3, undefined],
        at: ["plans", undefined],
        item: { id: "lisbon", title: "Trip to Lisbon" },
        texts: ["Plans", ""],
        has: [true, false],
        disabled: [false, true, true],
    });
});

test("rerun only the readers of the keys that come or go as the sections change", () => {
    // follow how often two keys' readers run while the shown notes change
    const [search, setSearch] = createSignal("");
    const runs = { plans: 0, groceries: 0 };
    const notes = createNotes(() =>
        SECTIONS.map((section) => ({
            ...section,
            items: section.items.filter((note) => note.title.toLowerCase().includes(search())),
        })),
    );
    rooted(() => {
        createEffect(
            () => notes.has("plans"),
            () => {
                runs.plans += 1;
            },
        );
        createEffect(
            () => notes.has("groceries"),
            () => {
                runs.groceries += 1;
            },
        );
    });
    flush();
    setSearch("gro");
    flush();
    setSearch("groc");
    flush();

    // plans leaves once and groceries never does
    expect({ keys: notes.keys(), runs }).toEqual({
        keys: ["groceries"],
        runs: { plans: 2, groceries: 1 },
    });
});

test("list no items while their load runs, then the loaded ones, and throw a failed load's error", async () => {
    // load one collection's notes and fail another's
    const loaded = new Load(Promise.resolve(SECTIONS));
    const failed = new Load<readonly CollectionSection<Note>[]>(
        Promise.reject(new TypeError("offline")),
    );
    const notes = createNotes(() => loaded.value(), loaded);
    const broken = createNotes(() => failed.value(), failed);
    const before = { state: notes.state(), keys: notes.keys() };
    await Promise.resolve();
    flush();

    expect({
        before,
        after: { state: notes.state(), keys: notes.keys() },
        failed: broken.state(),
        error: (() => {
            try {
                broken.keys();

                return "none";
            } catch (error) {
                const cause: unknown = error instanceof Error ? error.cause : undefined;

                return cause instanceof Error ? `${cause.name}: ${cause.message}` : "unknown";
            }
        })(),
        early: (() => {
            try {
                new Load(Promise.withResolvers<never>().promise).value();

                return "none";
            } catch (error) {
                return String(error);
            }
        })(),
    }).toEqual({
        before: { state: "loading", keys: [] },
        after: { state: "loaded", keys: ["groceries", "lisbon", "plans", "taxes"] },
        failed: "failed",
        error: "TypeError: offline",
        early: "TypeError: the load has no value yet",
    });
});

test("list the items child components add in mount order, dropping each as it unmounts", () => {
    // add three items, the second inside a root that disposes
    const builder = rooted(() => new CollectionBuilder<string>());
    const disposeSecond = rooted(() =>
        createRoot((dispose) => {
            builder.add("second");

            return dispose;
        }),
    );
    rooted(() => {
        builder.add("first");
        builder.add("third");
    });
    flush();
    const mounted = builder.items();
    disposeSecond();
    flush();

    expect({ mounted, listed: builder.items() }).toEqual({
        mounted: ["second", "first", "third"],
        listed: ["first", "third"],
    });
});

test("list each mount of equal items, one remounting before the items are read again", () => {
    // mount the same name twice, unmount one, then unmount the other and mount it again unread
    const builder = rooted(() => new CollectionBuilder<string>());
    const mount = () =>
        rooted(() =>
            createRoot((dispose) => {
                builder.add("Ada");

                return dispose;
            }),
        );
    const [first, second] = [mount(), mount()];
    flush();
    const both = builder.items();
    first();
    flush();
    const one = builder.items();
    second();
    mount();
    flush();

    expect([both, one, builder.items()]).toEqual([["Ada", "Ada"], ["Ada"], ["Ada"]]);
});

test("read no item of a child whose frame waits for a load, then read it with the frame", async () => {
    // show a child and start a load in one change, and rerun a reader while the load runs
    const load = Promise.withResolvers<string>();
    const [isShown, setShown] = createSignal(false);
    const [ticks, setTicks] = createSignal(0);
    const { container } = render(() => {
        const builder = new CollectionBuilder<string>();
        const data = createMemo(() => (isShown() ? load.promise : "none"));
        const Child = () => {
            builder.add("child");

            return <i />;
        };

        return (
            <p>
                {ticks()}:{builder.size()}:{data()}
                <Show when={isShown()}>
                    <Child />
                </Show>
            </p>
        );
    });
    setShown(true);
    flush();
    setTicks(1);
    flush();
    const held = container.textContent;
    load.resolve("loaded");
    await wait(0);
    flush();

    expect([held, container.textContent]).toEqual(["1:0:none", "1:1:loaded"]);
});

test("replace an earlier item of a key with the one a move remounts before the earlier unmounts", () => {
    // add a card in one column, remount it in another before the first unmounts, then unmount the first
    const builder = rooted(
        () =>
            new CollectionBuilder<{ readonly key: string; readonly column: string }>(
                undefined,
                (card) => card.key,
            ),
    );
    const disposeTodo = rooted(() =>
        createRoot((dispose) => {
            builder.add({ key: "post", column: "todo" });

            return dispose;
        }),
    );
    rooted(() => builder.add({ key: "post", column: "doing" }));
    flush();
    const during = builder.items();
    disposeTodo();
    flush();

    expect({ during, after: builder.items() }).toEqual({
        during: [{ key: "post", column: "doing" }],
        after: [{ key: "post", column: "doing" }],
    });
});

test("read the earlier item of a key again once the newer one unmounts first", () => {
    // remount a card, then unmount the newer mount while the earlier one stays
    const builder = rooted(
        () => new CollectionBuilder<{ readonly column: string }>(undefined, () => "post"),
    );
    rooted(() => builder.add({ column: "todo" }));
    const disposeDoing = rooted(() =>
        createRoot((dispose) => {
            builder.add({ column: "doing" });

            return dispose;
        }),
    );
    flush();
    const during = [builder.items(), builder.item("post")];
    disposeDoing();
    flush();

    expect([during, [builder.items(), builder.item("post")]]).toEqual([
        [[{ column: "doing" }], { column: "doing" }],
        [[{ column: "todo" }], { column: "todo" }],
    ]);
});

test("order the items by their elements' places in the document once they settle", async () => {
    // add three items whose elements stand in the document in another order than they mounted
    const list = document.createElement("ul");
    document.body.append(list);
    onTestFinished(() => list.remove());
    const elements = new Map(
        ["first", "second", "third"].map((name) => [name, document.createElement("li")] as const),
    );
    list.append(...["third", "first", "second"].map((name) => elements.get(name) ?? list));
    const builder = rooted(() => {
        const ordered = new CollectionBuilder<string>((name) => elements.get(name));
        for (const name of ["first", "second", "third"]) {
            ordered.add(name);
        }

        return ordered;
    });
    await Promise.resolve();
    flush();

    expect(builder.items()).toEqual(["third", "first", "second"]);
});
