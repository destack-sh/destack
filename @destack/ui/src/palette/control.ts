import type { SwatchName } from "@destack/color";
import type { IconBodies } from "@destack/icon";
import type { Action } from "../action/index.ts";
import { schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { type Accessor, createSignal, type JSX, type Setter, useEnvironment } from "@destack/view";
import { StoredItem } from "@destack/view/primitives/storage";
import type { CommandControl } from "../command/index.ts";

/** How strongly a query matching a title from its start, a word's start or inside it ranks. */
const MATCHES = { start: 3, word: 2, inside: 1 } as const;

/** The recent entries the root page lists while the search is empty, as Raycast's. */
const RECENT_SHOWN = 5;

/** The uses a palette remembers, the least frecent dropping out past it. */
const RECENT_KEPT = 50;

/** The time over which a use's weight halves, a week, after Firefox's frecency. */
const HALF_LIFE = 7 * 24 * 60 * 60 * 1000;

/** The most a remembered use lifts an entry above an equal match, under one match step. */
const FRECENCY_WEIGHT = 0.5;

/** The uses of a palette's entries as saved: each entry's use count and last use. */
const SavedUses = schema.array(
    schema.object({
        /** The entry's id. */
        id: schema.string().min(1),
        /** How often it ran. */
        count: schema.number().int().positive(),
        /** When it last ran, in milliseconds since the epoch. */
        last: schema.number(),
    }),
);

/** The remembered uses of a palette's entries. */
type Uses = schema.Infer<typeof SavedUses>;

/** Something an entry does, after Raycast's actions: the first opens it, the rest in a panel. */
export interface PaletteAction extends Action {
    /** Whether the palette stays open after the action runs, closing by default. */
    readonly isStayingOpen?: boolean;
}

/** A detail at an entry's end, after Raycast's accessories: text, or a tag in a swatch's tint. */
export interface PaletteAccessory {
    /** The text. */
    readonly text: string;
    /** The swatch a tag tints itself in, plain text when absent. */
    readonly color?: SwatchName;
}

/** An entry of a palette: what it shows, what it does, and the page it opens. */
export interface PaletteItem {
    /** The entry's id, unique in its palette, which its uses are remembered by. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** The line after the title, such as its kind. */
    readonly subtitle?: string;
    /** The icon before the title. */
    readonly icon?: IconBodies;
    /** Further words a search matches. */
    readonly keywords?: readonly string[];
    /** The key combination that runs the entry from anywhere, shown at its end. */
    readonly shortcut?: string;
    /** The details at the entry's end, such as its status or its space. */
    readonly accessories?: readonly PaletteAccessory[];
    /** What it does, the first on Enter and the second on Command or Control with Enter. */
    readonly actions: readonly PaletteAction[];
    /** The page Tab or the right arrow opens, such as the people an assignment picks from. */
    readonly page?: () => PalettePage;
    /** Render the entry's detail beside the list. */
    readonly detail?: () => JSX.Element;
}

/** A group of a page's entries under a heading. */
export interface PaletteGroup {
    /** The group's id. */
    readonly id: string;
    /** The heading. */
    readonly title: string;
    /** The entries, in their order before a search ranks them. */
    readonly items: Accessor<readonly PaletteItem[]>;
}

/** A page of a palette: its groups of entries and its search's placeholder. */
export interface PalettePage {
    /** The page's id. */
    readonly id: string;
    /** The title its chip shows in the search. */
    readonly title: string;
    /** The text the empty search shows. */
    readonly placeholder?: string;
    /** The groups of entries. */
    readonly groups: readonly PaletteGroup[];
}

/** A group's entries as a search ranks them. */
export interface PaletteResult {
    /** The group. */
    readonly group: PaletteGroup;
    /** Its entries that match, best first. */
    readonly items: readonly PaletteItem[];
}

/** The shown page's entries as a search ranks them: the recent ones, then each group's. */
export interface PaletteResults {
    /** The most frecent entries, listed first on an empty root search. */
    readonly recent: readonly PaletteItem[];
    /** Each group's other matching entries, groups without any left out. */
    readonly groups: readonly PaletteResult[];
}

/** A palette's pages, ranking, remembered uses and actions panel, over its command's search. */
export class PaletteControl {
    /** The command whose search and page stack the palette steers. */
    readonly command: CommandControl;
    /** The entry whose actions panel is open, none while it is closed. */
    readonly acting: Accessor<PaletteItem | undefined>;
    /** The remembered uses of the entries. */
    readonly uses: Accessor<Uses>;
    /** Replace the entry whose actions panel is open. */
    readonly #setActing: Setter<PaletteItem | undefined>;
    /** Replace the remembered uses. */
    readonly #setUses: Setter<Uses>;
    /** The root page. */
    readonly #root: () => PalettePage;
    /** The pages opened over the root, by name. */
    readonly #opened = new Map<string, PalettePage>();
    /** Close the palette. */
    readonly #close: () => void;
    /** The saved uses, under the palette's key. */
    readonly #stored: StoredItem;
    /** Read the time now, in milliseconds since the epoch. */
    readonly #now: () => number;

    /** Create a palette over a command and its root page, remembering uses under a key. */
    constructor(options: {
        readonly command: CommandControl;
        readonly root: () => PalettePage;
        readonly key: string;
        readonly close: () => void;
    }) {
        // start without an open panel or remembered uses
        const [acting, setActing] = createSignal<PaletteItem | undefined>(undefined, {
            ownedWrite: true,
        });
        const [uses, setUses] = createSignal<Uses>([], { ownedWrite: true });
        const environment = useEnvironment();
        const telemetry = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.command = options.command;
        this.acting = acting;
        this.uses = uses;
        this.#setActing = setActing;
        this.#setUses = setUses;
        this.#root = options.root;
        this.#close = options.close;
        this.#stored = new StoredItem(`destack-palette:${options.key}`, environment, telemetry);
        this.#now = () => environment.clock.now();
    }

    /** Read the page shown: the newest opened, else the root. */
    page(): PalettePage {
        // read the root, or the page the command's stack names, which the palette opened
        const name = this.command.page();
        const page = name === undefined ? this.#root() : this.#opened.get(name);
        if (page === undefined) {
            throw new RangeError(
                `palette page '${String(name)}' is invalid: the palette never opened it`,
            );
        }

        return page;
    }

    /** Take the uses the viewer left, which the server never reads. */
    restore(): void {
        const text = this.#stored.read();
        this.#setUses(text === undefined ? [] : SavedUses.parse(JSON.parse(text)));
    }

    /** Open an entry's page over the shown one. */
    drill(item: PaletteItem): void {
        // refuse an entry without a page
        const page = item.page?.();
        if (page === undefined) {
            throw new TypeError(`palette entry '${item.id}' is invalid: it opens no page`);
        }

        // forget the pages the command has left, then open the new one
        const stacked = new Set(this.command.pages().map((each) => each.name));
        for (const name of this.#opened.keys()) {
            if (!stacked.has(name)) {
                this.#opened.delete(name);
            }
        }
        this.#opened.set(page.id, page);
        this.command.push({ name: page.id, title: page.title });
    }

    /** Open or close an entry's actions panel. */
    act(item: PaletteItem | undefined): void {
        this.#setActing(item);
    }

    /** Run an entry's action, its first by default, remembering the use and closing the palette. */
    async run(
        item: PaletteItem,
        action: PaletteAction | undefined = item.actions[0],
    ): Promise<void> {
        // refuse an entry that does nothing
        if (action === undefined) {
            throw new TypeError(`palette entry '${item.id}' is invalid: it has no actions`);
        }

        // remember the use, then close and run
        this.#remember(item.id);
        this.#setActing(undefined);
        if (action.isStayingOpen !== true) {
            this.#close();
        }
        await action.run();
    }

    /** Rank the shown page's entries for the search: the recent first on an empty root search. */
    results(): PaletteResults {
        // rank each group's matching entries, lifting the frecent ones
        const query = this.command.search.value().trim().toLowerCase();
        const page = this.page();
        const ranked = page.groups.map((group) => ({
            group,
            items: group
                .items()
                .flatMap((item) => {
                    const match = matchOf(item, query);

                    return match === 0 ? [] : [{ item, score: match + this.#frecency(item.id) }];
                })
                .toSorted((left, right) => (query === "" ? 0 : right.score - left.score))
                .map(({ item }) => item),
        }));

        // put the recent entries first on the root while the search is empty
        const recent = query === "" && this.command.pages().length === 0 ? this.#recent(page) : [];
        const shown = new Set(recent.map((item) => item.id));
        const groups = ranked
            .map((result) => ({
                group: result.group,
                items: result.items.filter((item) => !shown.has(item.id)),
            }))
            .filter((result) => result.items.length > 0);

        return { recent, groups };
    }

    /** List the page's most frecent entries. */
    #recent(page: PalettePage): PaletteItem[] {
        const items = new Map(
            page.groups.flatMap((group) => group.items()).map((item) => [item.id, item]),
        );

        return this.uses()
            .toSorted((left, right) => this.#frecency(right.id) - this.#frecency(left.id))
            .flatMap((use) => {
                const item = items.get(use.id);

                return item === undefined ? [] : [item];
            })
            .slice(0, RECENT_SHOWN);
    }

    /** Weigh an entry's uses, each halving in weight every week, as a lift below one match step. */
    #frecency(id: string): number {
        // weigh nothing for an entry never used
        const use = this.uses().find((each) => each.id === id);
        if (use === undefined) {
            return 0;
        }

        // halve the count's weight each week since the last use, and bound the lift
        const weight = use.count * 0.5 ** ((this.#now() - use.last) / HALF_LIFE);

        return FRECENCY_WEIGHT * (1 - 1 / (1 + weight));
    }

    /** Count a use of an entry and save the uses, saving the most frecent. */
    #remember(id: string): void {
        // put the entry's use first with one more count, then the others by frecency up to the limit
        const now = this.#now();
        const known = this.uses().find((use) => use.id === id);
        const others = this.uses()
            .filter((use) => use.id !== id)
            .toSorted((left, right) => this.#frecency(right.id) - this.#frecency(left.id));
        const next = [{ id, count: (known?.count ?? 0) + 1, last: now }, ...others].slice(
            0,
            RECENT_KEPT,
        );
        this.#setUses(next);
        this.#stored.write(JSON.stringify(next));
    }
}

/** Score how an entry's title or keywords match a search, 0 for no match. */
function matchOf(item: PaletteItem, query: string): number {
    // match every entry for an empty search
    if (query === "") {
        return MATCHES.inside;
    }

    // prefer a title starting with the search, then a word of it, then any text
    const title = item.title.toLowerCase();
    const words = [title, ...(item.keywords ?? []).map((word) => word.toLowerCase())];
    if (title.startsWith(query)) {
        return MATCHES.start;
    } else if (words.some((text) => text.split(/\s+/u).some((word) => word.startsWith(query)))) {
        return MATCHES.word;
    }

    return words.some((text) => text.includes(query)) ||
        item.subtitle?.toLowerCase().includes(query) === true
        ? MATCHES.inside
        : 0;
}
