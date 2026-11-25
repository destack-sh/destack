import type { Localization } from "@destack/locale";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    type JSX,
    onCleanup,
    untrack,
    useContext,
} from "@destack/view";
import type { DragManager } from "../drag/index.ts";
import { tabbablesIn } from "../focus/index.ts";
import type { SortableMove } from "../sortable/index.ts";
import type { TabsControl } from "../tabs/index.ts";
import { PANE_TYPE, PaneDrag } from "./drag.ts";
import { handlePanes } from "./handle.ts";
import {
    type PaneLeaf,
    type PaneEdge,
    type PaneItem,
    PaneLayout,
    type PaneNode,
    type PanePlace,
    type PanePlacement,
} from "./layout.ts";

/** The least length of a pane along either axis, in pixels, which fits a tab. */
export const LEAST_LENGTH = 120;

/** What a person moves: something open, or a whole pane. */
export interface PaneMove {
    /** The id of the item or of the pane, or of something from elsewhere. */
    readonly id: string;
    /** The type of what moves, such as `tab`, `pane`, or another part's type. */
    readonly type: string;
    /** Where it goes. */
    readonly place: PanePlace;
    /** Whether it stays where it was, too. */
    readonly isCopied: boolean;
}

/** The verbs of moves. */
export const PaneMove = {
    /** Make a move in a layout, opening what comes from elsewhere by its id. */
    apply(layout: PaneLayout, move: PaneMove): PaneLayout {
        // move a pane
        if (move.type === PANE_TYPE) {
            return PaneLayout.dock(layout, move.id, move.place);
        }
        const pane = PaneLayout.paneOf(layout, move.id);
        const item = pane?.items.find(({ id }) => id === move.id);

        // open a copy under an id of its own, else move it, or open it
        return move.isCopied && item !== undefined
            ? PaneLayout.open(
                  layout,
                  { id: PaneLayout.freeItem(layout, item.value), value: item.value },
                  move.place,
              )
            : PaneLayout.open(layout, move.id, move.place);
    },
};

/** The room around a pane's body within the pane, in pixels. */
export interface PaneInset {
    /** The room before it along the text. */
    readonly start: number;
    /** The room after it. */
    readonly end: number;
    /** The room above it. */
    readonly top: number;
    /** The room below it. */
    readonly bottom: number;
}

/** An item that was closed, and where it stood. */
export interface PaneClosed {
    /** The item. */
    readonly item: PaneItem;
    /** The id of its pane. */
    readonly pane: string;
    /** Its place among the items. */
    readonly index: number;
}

/** What a group of panes is given. */
export interface PaneGroupOptions {
    /** Where each pane is, and what it has open. */
    readonly layout: PaneLayout;
    /** Handle the layout changing. */
    readonly onLayoutChange?: (layout: PaneLayout) => void;
    /** What an item is called, its value by default. */
    readonly label?: (item: PaneItem) => JSX.Element;
    /** Handle a move in place of making it, such as to refuse it. */
    readonly onMove?: (move: PaneMove) => void;
}

/** A pane in its group. */
export interface PaneEntry {
    /** The id. */
    readonly id: string;
    /** Read its element, none before it mounts. */
    readonly element: () => HTMLElement | undefined;
    /** Read the element its contents cover, none for a pane without. */
    readonly body: () => HTMLElement | undefined;
    /** Read its least lengths in pixels along both axes. */
    readonly least: () => number;
    /** Read the types it takes. */
    readonly accepts: () => readonly string[] | undefined;
    /** Read its tabs, none for a pane without. */
    readonly tabs: () => TabsControl | undefined;
}

/** The panes of a group and what a person does with them. */
export class PaneGroupControl {
    /** What the ids of its tabs and contents start with. */
    readonly prefix: string;
    /** The drags that move its tabs and panes. */
    readonly manager: DragManager;
    /** The drags of its tabs and panes, and where they land. */
    readonly drag: PaneDrag;
    /** Read the panes' places, in the order they are read. */
    readonly placements: Accessor<readonly PanePlacement[]>;
    /** Read the lines between panes. */
    readonly edges: Accessor<readonly PaneEdge[]>;
    /** Read the items whose contents are drawn, in an order that never changes. */
    readonly drawn: Accessor<readonly string[]>;
    /** The room around each pane's body. */
    readonly insets: Accessor<ReadonlyMap<string, PaneInset>>;
    /** What it is given. */
    readonly options: PaneGroupOptions;
    /** The items closed here in order, which open again last first. */
    readonly closed: Accessor<readonly PaneClosed[]>;
    /** Replace the closed items. */
    readonly #setClosed: (change: (closed: readonly PaneClosed[]) => readonly PaneClosed[]) => void;
    /** The panes as they mount. */
    readonly #entries = new Map<string, PaneEntry>();
    /** Replace the room around a pane's body. */
    readonly #setInsets: (
        change: (insets: ReadonlyMap<string, PaneInset>) => ReadonlyMap<string, PaneInset>,
    ) => void;
    /** The last change, and the layout it was made of. */
    #latest: { readonly given: PaneLayout; readonly next: PaneLayout } | undefined;
    /** The element of the group. */
    #element: HTMLElement | undefined;

    /** Create the panes of a group. */
    constructor(options: PaneGroupOptions, manager: DragManager) {
        // read the layout, each content drawn from when it is first selected
        this.options = options;
        this.manager = manager;
        this.prefix = createUniqueId();
        this.placements = createMemo(() => PaneLayout.placements(options.layout));
        this.edges = createMemo(() => PaneLayout.edges(options.layout));
        this.drawn = createMemo<readonly string[]>((before) => {
            const open = new Set(PaneLayout.panes(options.layout).flatMap(({ recent }) => recent));
            const standing = (before ?? []).filter((id) => open.has(id));

            return [...standing, ...[...open].filter((id) => !standing.includes(id))];
        });
        const [insets, setInsets] = createSignal<ReadonlyMap<string, PaneInset>>(new Map(), {
            ownedWrite: true,
        });
        const [closed, setClosed] = createSignal<readonly PaneClosed[]>([], { ownedWrite: true });
        this.closed = closed;
        this.#setClosed = (change) => void setClosed(change);
        this.insets = insets;
        this.#setInsets = (change) => void setInsets(change);
        this.drag = new PaneDrag(this);
    }

    /** List what handles the actions on its panes, for its element's `ref`. */
    handlers(locale: Localization): ((element: Element) => void)[] {
        return handlePanes(this, locale);
    }

    /** Read the layout. */
    layout(): PaneLayout {
        return this.options.layout;
    }

    /** Report whether a person changes nothing here, as nothing handles a change. */
    isLocked(): boolean {
        return this.options.onLayoutChange === undefined && this.options.onMove === undefined;
    }

    /** Change the layout. */
    change(change: (layout: PaneLayout) => PaneLayout): void {
        // follow a change of the same moment, which the owner has not given back yet
        const given = untrack(() => this.options.layout);
        const before = this.#latest?.given === given ? this.#latest.next : given;
        const next = change(before);
        if (next !== before) {
            this.#latest = { given, next };
            this.options.onLayoutChange?.(next);
        }
    }

    /** Make a move, or hand it on. */
    move(move: PaneMove): void {
        // make it, unless the owner does
        const before = this.#latest;
        if (this.options.onMove === undefined) {
            this.change((layout) => PaneMove.apply(layout, move));
        } else {
            this.options.onMove(move);
        }

        // go on where it landed, once a menu it came from gave the focus back
        const landed = this.#latest === before ? undefined : this.#latest?.next.active;
        if (landed !== undefined && move.type !== PANE_TYPE) {
            void this.manager.environment.display.frame().then(() => this.focus(landed));
        }
    }

    /** Make a move of a list of tabs. */
    moveTab(move: SortableMove): void {
        // find the place among the pane's other items
        const pane = untrack(() => PaneLayout.pane(this.options.layout, move.container));
        const others = pane?.items.filter(({ id }) => id !== move.id) ?? [];
        const at = others.findIndex(({ id }) => id === move.next);
        this.move({
            id: move.id,
            type: move.type,
            place: {
                kind: "item",
                pane: move.container,
                index: at === -1 ? others.length : at,
            },
            isCopied: this.manager.current()?.effect === "copy",
        });
    }

    /** Close an item, remembered to open again. */
    close(item: string): void {
        // remember where it stood
        const pane = untrack(() => PaneLayout.paneOf(this.options.layout, item));
        const at = pane?.items.findIndex(({ id }) => id === item) ?? -1;
        const closed = pane?.items[at];
        if (pane !== undefined && closed !== undefined) {
            this.#setClosed((all) => [...all, { item: closed, pane: pane.id, index: at }]);
            this.change((layout) => PaneLayout.close(layout, item));
        }
    }

    /** Close a pane with every item in it, each remembered to open again. */
    remove(pane: string): void {
        // remember them in order
        const items = untrack(() => PaneLayout.pane(this.options.layout, pane))?.items ?? [];
        this.#setClosed((all) => [...all, ...items.map((item, index) => ({ item, pane, index }))]);
        this.change((layout) => PaneLayout.remove(layout, pane));
    }

    /** Cover the others with a pane, or uncover them. */
    toggleMaximized(pane: string): void {
        this.change((layout) =>
            layout.maximized === pane
                ? PaneLayout.restore(layout)
                : PaneLayout.maximize(layout, pane),
        );
    }

    /** Open the item closed last again where it stood, or in the active pane. */
    reopen(): void {
        // take the last one
        const closed = untrack(this.closed);
        const last = closed.at(-1);
        if (last === undefined) {
            return;
        }
        this.#setClosed((all) => all.slice(0, -1));
        this.change((layout) =>
            PaneLayout.open(layout, last.item, {
                kind: "item",
                pane: last.pane,
                index: last.index,
            }),
        );
    }

    /** Move the focus to a pane: its selected tab, else what takes the focus in it first. */
    focus(pane: string): void {
        // wait until it shows
        void this.manager.environment.display.frame().then(() => {
            // find what takes the focus: the tab, else the content, else the pane
            const entry = this.#entries.get(pane);
            const selected = untrack(() => {
                const current = PaneLayout.pane(this.options.layout, pane);

                return current === undefined ? undefined : PaneLayout.selected(current);
            });
            const root = this.#element?.ownerDocument;
            const tab =
                selected === undefined ? null : root?.getElementById(this.tabId(selected.id));
            const content =
                selected === undefined ? null : root?.getElementById(this.contentId(selected.id));
            const element = entry?.element();
            const first = [content, element].flatMap((each) =>
                each === null || each === undefined ? [] : tabbablesIn(each),
            )[0];
            (tab ?? first)?.focus();
        });
    }

    /** Read the id of the pane an element is in, or shows in, the active one elsewhere. */
    paneAt(element: Element | null): string | undefined {
        // pass over the panes of a group inside one of these
        for (let each = element?.closest("[data-pane]"); each;) {
            const id = each.getAttribute("data-pane") ?? "";
            if (PaneLayout.pane(this.options.layout, id) !== undefined) {
                return id;
            }
            each = each.parentElement?.closest("[data-pane]");
        }

        return this.options.layout.active;
    }

    /** Take the element of the group. */
    attach(element: HTMLElement): void {
        this.#element = element;
    }

    /** Read the element of the group. */
    element(): HTMLElement | undefined {
        return this.#element;
    }

    /** Read a pane that is mounted. */
    entry(pane: string): PaneEntry | undefined {
        return this.#entries.get(pane);
    }

    /** Read the box of a pane, undefined while another covers it. */
    placement(pane: string): PanePlacement | undefined {
        return this.placements().find((each) => each.pane.id === pane);
    }

    /** Read an item by its id. */
    item(id: string): PaneItem | undefined {
        return PaneLayout.paneOf(this.options.layout, id)?.items.find((item) => item.id === id);
    }

    /** Read the id of an item's content. */
    contentId(item: string): string {
        return `${this.prefix}-panel-${encodeURIComponent(item)}`;
    }

    /** Read the id of an item's tab. */
    tabId(item: string): string {
        return `${this.prefix}-tab-${encodeURIComponent(item)}`;
    }

    /** Set the room around a pane's body. */
    inset(pane: string, inset: PaneInset | undefined): void {
        this.#setInsets((insets) => {
            // leave the same room as it is
            const before = insets.get(pane);
            const isSame =
                before?.start === inset?.start &&
                before?.end === inset?.end &&
                before?.top === inset?.top &&
                before?.bottom === inset?.bottom;
            if (isSame) {
                return insets;
            }
            const next = new Map(insets);
            if (inset === undefined) {
                next.delete(pane);
            } else {
                next.set(pane, inset);
            }

            return next;
        });
    }

    /** Add a pane until it unmounts, taking drops on its sides and its middle. */
    add(entry: PaneEntry): void {
        // keep it until it unmounts
        this.#entries.set(entry.id, entry);
        onCleanup(() => {
            if (this.#entries.get(entry.id) === entry) {
                this.#entries.delete(entry.id);
            }
        });
        this.drag.add(entry);
    }

    /** Move the line between two parts to a share of their box, each at its least or more. */
    resize(edge: PaneEdge, share: number): void {
        this.change((layout) => {
            // read the split and the room both parts have
            const split = nodeAt(layout.root, edge.path);
            const bounds = this.#element?.getBoundingClientRect();
            if (split?.kind !== "split" || bounds === undefined) {
                return layout;
            }
            const isInline = edge.direction === "horizontal";
            const room = isInline ? bounds.width * edge.box.width : bounds.height * edge.box.height;
            const [before, after] = split.children.slice(edge.index, edge.index + 2);
            if (before === undefined || after === undefined || room === 0) {
                return layout;
            }

            // keep both at their least lengths
            const least = this.#leastOf(before, edge.direction) / room;
            const most = 1 - this.#leastOf(after, edge.direction) / room;
            const kept = least > most ? edge.share : Math.min(Math.max(share, least), most);
            const both = (split.sizes[edge.index] ?? 0) + (split.sizes[edge.index + 1] ?? 0);

            return PaneLayout.resize(
                layout,
                edge.path,
                split.sizes.toSpliced(edge.index, 2, both * kept, both * (1 - kept)),
            );
        });
    }

    /** Read the least length of a node along an axis, in pixels. */
    #leastOf(node: PaneNode, direction: PaneEdge["direction"]): number {
        // read a pane's own, or its parts' together
        if (node.kind === "leaf") {
            return this.#entries.get(node.id)?.least() ?? LEAST_LENGTH;
        }
        const parts = node.children.map((child) => this.#leastOf(child, direction));

        return node.direction === direction
            ? parts.reduce((sum, part) => sum + part, 0)
            : Math.max(...parts);
    }
}

/** The group of panes around an element, null outside one. */
export const PaneGroupContext = createContext<PaneGroupControl | null>(null);

/** A pane as its parts read it. */
export interface PaneControl {
    /** Read the pane. */
    readonly pane: Accessor<PaneLeaf>;
    /** The group. */
    readonly group: PaneGroupControl;
    /** Take the element its contents cover. */
    readonly setBody: (element: HTMLElement | undefined) => void;
    /** Read that element. */
    readonly body: () => HTMLElement | undefined;
    /** Take its tabs, until they unmount. */
    readonly setTabs: (tabs: TabsControl) => void;
    /** Read its tabs, none for a pane without. */
    readonly tabs: () => TabsControl | undefined;
}

/** The pane around an element, null outside one. */
export const PaneContext = createContext<PaneControl | null>(null);

/** Read the nearest group of panes, refusing elements outside one. */
export function usePaneGroup(): PaneGroupControl {
    const control = useContext(PaneGroupContext);
    if (control === null) {
        throw new TypeError("pane elements need a pane group around them");
    }

    return control;
}

/** Read the nearest pane, refusing elements outside one. */
export function usePane(): PaneControl {
    const control = useContext(PaneContext);
    if (control === null) {
        throw new TypeError("a pane's elements need a pane around them");
    }

    return control;
}

/** Read the node at a path. */
function nodeAt(node: PaneNode | undefined, path: readonly number[]): PaneNode | undefined {
    return path.reduce<PaneNode | undefined>(
        (each, index) => (each?.kind === "split" ? each.children[index] : undefined),
        node,
    );
}
