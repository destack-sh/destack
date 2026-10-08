import { expect, onTestFinished, test } from "@destack/test";
import { createEffect, createRoot, createSignal, flush } from "@destack/view";
import { Collection, CollectionBuilder, type CollectionSection, Load } from "./collection.ts";

/** A note a test collection holds. */
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

test("hold no items while their load runs, then the loaded ones, and throw a failed load's error", async () => {
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
        early: "TypeError: a load has no value until it loads",
    });
});

test("keep the items child components add in mount order, dropping each as it unmounts", () => {
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

    expect({ mounted, kept: builder.items() }).toEqual({
        mounted: ["second", "first", "third"],
        kept: ["first", "third"],
    });
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
