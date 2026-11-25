import { expect, test } from "@destack/test";
import { createMemo, createSignal, flush } from "@destack/view";
import { renderHook } from "@destack/view/test";
import { AutocompleteControl } from "../autocomplete/index.ts";
import { CommandControl } from "../command/index.ts";
import { type PaletteItem, PaletteControl, type PalettePage } from "./control.ts";

/** Make an entry that records its runs. */
function entryOf(id: string, title: string, ran: string[], keywords?: readonly string[]) {
    return {
        id,
        title,
        ...(keywords === undefined ? {} : { keywords }),
        actions: [{ id: "run", title: "Run", run: () => void ran.push(id) }],
    } satisfies PaletteItem;
}

/** Open a palette over a root of tasks and a page of people, recording runs and closes. */
function openPalette(key: string) {
    const ran: string[] = [];
    let closes = 0;
    const people: PalettePage = {
        id: "people",
        title: "People",
        groups: [{ id: "people", title: "People", items: () => [entryOf("ada", "Ada", ran)] }],
    };
    const assign: PaletteItem = {
        id: "assign",
        title: "Assign to",
        actions: [],
        page: () => people,
    };
    const root: PalettePage = {
        id: "root",
        title: "Launch",
        groups: [
            {
                id: "tasks",
                title: "Tasks",
                items: () => [
                    entryOf("demo", "Record the demo", ran),
                    entryOf("archive", "Archive launch", ran),
                    entryOf("pricing", "Pricing page", ran, ["archived plans"]),
                ],
            },
            { id: "commands", title: "Commands", items: () => [assign] },
        ],
    };
    const { result } = renderHook(() => {
        const command = new CommandControl(new AutocompleteControl({}));

        return new PaletteControl({ command, root: () => root, key, close: () => (closes += 1) });
    });

    return { palette: result, ran, assign, closes: () => closes };
}

/** List the ids the palette shows: recent first, then each group's. */
function shownIds(palette: PaletteControl): string[][] {
    const { recent, groups } = palette.results();

    return [
        recent.map((item) => item.id),
        ...groups.map((result) => result.items.map((item) => item.id)),
    ];
}

test("rank a search's matches by a title's start, a word's start, then any text", () => {
    // search for "arc", which starts one title, starts a keyword's word, and is inside neither else
    const { palette } = openPalette("rank");
    palette.command.search.type("arc");
    flush();

    expect(shownIds(palette)).toEqual([[], ["archive", "pricing"]]);
});

test("run an entry's first action, close, and list it first among the recent on the empty root", async () => {
    // run the demo entry, then read the root again with an empty search
    const { palette, ran, closes } = openPalette("recent");
    const demo = palette.results().groups[0]?.items[0];
    if (demo === undefined) {
        throw new TypeError("the root lists no tasks");
    }
    await palette.run(demo);
    flush();

    // a reopened palette under the same key restores the use
    const reopened = openPalette("recent");
    reopened.palette.restore();
    flush();

    expect({
        ran,
        closes: closes(),
        shown: shownIds(palette),
        reopened: shownIds(reopened.palette),
    }).toEqual({
        ran: ["demo"],
        closes: 1,
        shown: [["demo"], ["archive", "pricing"], ["assign"]],
        reopened: [["demo"], ["archive", "pricing"], ["assign"]],
    });
});

test("open an entry's page over the root and go back, refusing an entry without one", () => {
    // drill into the people page, then pop it
    const { palette, ran, assign } = openPalette("pages");
    palette.drill(assign);
    flush();
    const onPage = [palette.page().id, shownIds(palette)];
    palette.command.pop();
    flush();

    expect({ onPage, back: palette.page().id }).toEqual({
        onPage: ["people", [[], ["ada"]]],
        back: "root",
    });
    expect(() => palette.drill(entryOf("demo", "Record the demo", ran))).toThrow(
        new TypeError("palette entry 'demo' is invalid: it opens no page"),
    );
});

test("list fifty of a long group, and the best twenty a search finds", () => {
    // search two hundred tasks by a word the last starts with
    const ran: string[] = [];
    const tasks = Array.from({ length: 200 }, (_task, i) =>
        entryOf(`task ${String(i)}`, i === 199 ? "Pack the case" : `Case ${String(i)} pack`, ran),
    );
    const root: PalettePage = {
        id: "root",
        title: "Tasks",
        groups: [{ id: "tasks", title: "Tasks", items: () => tasks }],
    };
    const { result } = renderHook(() => {
        const command = new CommandControl(new AutocompleteControl({}));

        return new PaletteControl({
            command,
            root: () => root,
            key: "long",
            close: () => undefined,
        });
    });
    const all = shownIds(result);
    result.command.search.type("pack");
    flush();
    const found = shownIds(result);

    expect([all.map((ids) => ids.length), found.map((ids) => ids.length), found[1]?.[0]]).toEqual([
        [0, 50],
        [0, 20],
        "task 199",
    ]);
});

test("list a group's entries once per change to them, not per key", () => {
    // count the listings while a search is typed, and after the entries change
    let listings = 0;
    const [notes, setNotes] = createSignal(["Lisbon"]);
    const root: PalettePage = {
        id: "root",
        title: "Notes",
        groups: [
            {
                id: "notes",
                title: "Notes",
                items: () => {
                    listings += 1;

                    return notes().map((note) => entryOf(note, note, []));
                },
            },
        ],
    };
    const { result } = renderHook(() => {
        const palette = new PaletteControl({
            command: new CommandControl(new AutocompleteControl({})),
            root: () => root,
            key: "listings",
            close: () => undefined,
        });

        return { palette, results: createMemo(() => palette.results()) };
    });
    result.results();
    for (const query of ["l", "li", "lis"]) {
        result.palette.command.search.type(query);
        flush();
        result.results();
    }
    const typed = listings;
    setNotes(["Lisbon", "Order form"]);
    flush();
    result.results();

    expect({ typed, changed: listings }).toEqual({ typed: 1, changed: 2 });
});
