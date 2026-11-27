import type { Search } from "@destack/db";
import type { Hue } from "@destack/style";
import type { IconBodies } from "@destack/icon";
import { schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import {
    type Accessor,
    createMemo,
    createSignal,
    type JSX,
    type Setter,
    useEnvironment,
} from "@destack/view";
import { StoredItem } from "@destack/view/primitives/storage";
import type { CommandControl } from "../command/index.ts";

/** The recent entries the root page lists while the search is empty. */
const RECENT_SHOWN = 5;

/** The entries of a group a palette lists, as many as a person reads before they search. */
const GROUP_SHOWN = 50;

/** The best entries of a group a search lists, about two screens, as typing on narrows it. */
const SEARCH_SHOWN = 20;

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

/** An icon, or a function that draws any other. */
export type PaletteIcon = IconBodies | (() => JSX.Element);

/** Something an entry does: the first opens it, the rest in a panel. */
export interface PaletteAction {
    /** The action's id, unique among its entry's. */
    readonly id: string;
    /** The title. */
    readonly title: string;
    /** The icon beside the title, drawn by a function for any other. */
    readonly icon?: PaletteIcon;
    /** The key combination that runs it. */
    readonly shortcut?: string;
    /** Whether it removes something, which the palette marks. */
    readonly isDestructive?: boolean;
    /** Whether it cannot run now. */
    readonly isDisabled?: boolean;
    /** Whether the palette stays open after it runs, closing by default. */
    readonly isStayingOpen?: boolean;
    /** Do it. */
    readonly run: () => Promise<void> | void;
}

/** A detail at an entry's end: text, or a tag in a swatch's tint. */
export interface PaletteAccessory {
    /** The text. */
    readonly text: string;
    /** The swatch a tag tints itself in, plain text when absent. */
    readonly color?: Hue;
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
    readonly icon?: PaletteIcon;
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
    /** Who ranks the entries a search finds: the palette by default, or the owner that searched. */
    readonly ranking?: "palette" | "owner";
    /** The text the entries match, which they mark, the typed one when absent. */
    readonly text?: Accessor<string | undefined>;
    /** Whether the owner still searches, which an empty palette says. */
    readonly isPending?: Accessor<boolean>;
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
    /** The shown page's groups with their entries, listed once per change to them. */
    readonly #listed: Accessor<readonly PaletteResult[]>;

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

        // list the entries apart from the search, which ranks them on every key
        this.#listed = createMemo(
            () => this.page().groups.map((group) => ({ group, items: group.items() })),
            { lazy: true },
        );
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
        const search = this.command.search.search();
        const ranked = this.#listed().map(({ group, items }) => ({
            group,
            items: group.ranking === "owner" ? items : this.#rank(items, search),
        }));

        // put the recent entries first on the root while the search is empty
        const recent = search.isEmpty && this.command.pages().length === 0 ? this.#recent() : [];
        const shown = new Set(recent.map((item) => item.id));
        const limit = search.isEmpty ? GROUP_SHOWN : SEARCH_SHOWN;
        const groups = ranked
            .map((result) => ({
                group: result.group,
                items: result.items.filter((item) => !shown.has(item.id)).slice(0, limit),
            }))
            .filter((result) => result.items.length > 0);

        return { recent, groups };
    }

    /** Report whether an owner of a shown group still searches. */
    isPending(): boolean {
        return this.page().groups.some((group) => group.isPending?.() === true);
    }

    /** List the entries a search matches, the best first, lifting the frecent ones. */
    #rank(items: readonly PaletteItem[], search: Search): PaletteItem[] {
        return items
            .flatMap((item) => {
                const score = search.score(textsOf(item));

                return score === undefined
                    ? []
                    : [{ item, score: score + this.#frecency(item.id) }];
            })
            .toSorted((left, right) => (search.isEmpty ? 0 : right.score - left.score))
            .map(({ item }) => item);
    }

    /** List the page's most frecent entries. */
    #recent(): PaletteItem[] {
        const items = new Map(
            this.#listed()
                .flatMap((listed) => listed.items)
                .map((item) => [item.id, item]),
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

/** List what a search finds an entry by: its title, its other words, and its subtitle. */
function textsOf(item: PaletteItem): string[] {
    return [
        item.title,
        ...(item.keywords ?? []),
        ...(item.subtitle === undefined ? [] : [item.subtitle]),
    ];
}
