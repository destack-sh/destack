import caretDown from "@destack/icon/phosphor/caret-down";
import caretRight from "@destack/icon/phosphor/caret-right";
import eye from "@destack/icon/phosphor/eye";
import dotsThree from "@destack/icon/phosphor/dots-three";
import plus from "@destack/icon/phosphor/plus";
import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import {
    color,
    radius,
    shadow,
    size,
    space,
    stroke,
    weight,
    width,
    swatch,
} from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { present } from "@destack/schema";
import { type Direction, t } from "@destack/locale";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createMemo,
    createSignal,
    createUniqueId,
    flush,
    For,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Action } from "../action/index.ts";
import { Icon } from "../icon/index.ts";
import { Collection, CollectionBuilder } from "../collection/index.ts";
import { ContextMenuContent } from "../context-menu/index.ts";
import { Focus, type KeyboardDelegate } from "../focus/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { Press, type InteractionEvent } from "../interaction/index.ts";
import { Menu, MenuAction, MenuControl } from "../menu/index.ts";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "../dropdown-menu/index.ts";
import { mergeProperties } from "../part/index.ts";
import { type MultipleSelection, Selection } from "../selection/index.ts";
import { type VirtualHost, VirtualHostContext, type VirtualWindow } from "../virtual-list/index.ts";
import {
    Sortable,
    SortableContainer,
    type SortableElementProperties,
    SortableHandle,
    type SortableHandleProperties,
    SortableItem,
    type SortableActivator,
    SortableMove,
    useSortable,
} from "../sortable/index.ts";

/** The name of a board's row of columns, which its container id ends in. */
const COLUMNS = "columns";

/** The item type of a board's columns, scoped by the board's id. */
const COLUMN_TYPE = "column";

/** The item type of a board's cards. */
export const CARD_TYPE = "card";

/** A card's edge inside its box, which no scrolling column clips, and its cast shadow below. */
const CARD_SHADOW = `inset 0 0 0 ${stroke.border} ${shadow.edge}, 0 1px 2px ${shadow.cast}`;

/** The styles of a board and its parts. */
const styles = style.create({
    board: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        minHeight: 0,
        overflowX: "auto",
        overscrollBehaviorX: "contain",
        scrollSnapType: "x proximity",
        paddingBlockEnd: space[2],
    },
    columns: {
        display: "flex",
        flexGrow: 1,
        alignItems: "flex-start",
        gap: space[3],
    },
    column: {
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        width: width.column,
        maxHeight: "100%",
        borderRadius: radius[3],
        backgroundColor: color.muted,
        scrollSnapAlign: "start",
    },
    columnBare: {
        backgroundColor: "transparent",
    },
    over: {
        boxShadow: `inset 0 0 0 ${stroke.border} ${swatch.destructiveMark}`,
    },
    header: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: space[2],
        minHeight: size[3],
        paddingBlock: space[1],
        paddingInlineStart: `calc(${space[2]} + ${space[3]})`,
        paddingInlineEnd: space[2],
        fontWeight: weight.medium,
    },
    count: {
        color: color.mutedForeground,
        fontWeight: weight.regular,
        fontVariantNumeric: "tabular-nums",
    },
    countOver: {
        color: swatch.destructiveMark,
    },
    toggle: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size[1],
        height: size[1],
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[2],
        backgroundColor: "transparent",
        color: color.mutedForeground,
        font: "inherit",
        cursor: "pointer",
    },
    end: {
        marginInlineStart: "auto",
    },
    action: {
        opacity: {
            default: 1,
            [media.hover]: {
                default: 0,
                ":is([data-slot=board-column-header]:hover *)": 1,
                ":is([data-slot=board-column-header]:focus-within *)": 1,
                ":is([aria-expanded=true])": 1,
            },
        },
    },
    grip: {
        position: "absolute",
        insetInlineStart: 0,
        width: `calc(${space[2]} + ${space[3]})`,
        padding: 0,
        opacity: {
            default: 1,
            [media.hover]: { default: 0, ":is(:hover > *)": 1, ":focus-visible": 1 },
        },
    },
    content: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        gap: space[2],
        minHeight: space[8],
        overflowY: "auto",
        paddingInline: space[2],
        paddingBlockStart: `calc(${stroke.ring} + ${stroke.border})`,
        paddingBlockEnd: space[2],
    },
    cell: {
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        width: width.column,
        borderRadius: radius[3],
        backgroundColor: color.muted,
        paddingBlockStart: space[2],
    },
    cards: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        margin: {
            default: 0,
            ":has(> [data-slot=virtual-list])": `calc(-1 * (${stroke.ring} + ${stroke.border}))`,
        },
    },
    hidden: {
        display: "none",
    },
    hiddenList: {
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        gap: space[1],
        width: width.column,
        paddingBlock: space[1],
        paddingInline: space[2],
    },
    hiddenTitle: {
        display: "flex",
        alignItems: "center",
        minHeight: size[1],
        paddingInline: space[3],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    hiddenRow: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        minHeight: size[2],
        paddingInlineStart: space[3],
        paddingInlineEnd: space[1],
        borderRadius: radius[3],
        backgroundColor: color.muted,
    },
    hiddenRowAimed: {
        boxShadow: `inset 0 0 0 ${stroke.ring} ${swatch.accentMark}`,
    },
    hiddenLabel: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    lane: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
    },
    laneHeader: {
        position: "sticky",
        insetInlineStart: 0,
        display: "flex",
        alignItems: "center",
        gap: space[1],
        width: "max-content",
        minHeight: size[3],
        fontWeight: weight.medium,
    },
    laneCells: {
        display: "flex",
        alignItems: "stretch",
        gap: space[3],
    },
    item: {
        paddingBlock: space[2],
        paddingInline: space[3],
        borderRadius: radius[3],
        backgroundColor: color.card,
        boxShadow: CARD_SHADOW,
    },
    lifting: {
        cursor: { default: "grab", ":active": "grabbing" },
    },
    selected: {
        boxShadow: `0 0 0 ${stroke.ring} ${swatch.accentMark}`,
    },
    add: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        paddingBlock: space[1],
        marginInline: space[2],
        marginBlockEnd: space[2],
        paddingInline: space[3],
        borderWidth: 0,
        borderRadius: radius[3],
        backgroundColor: "transparent",
        color: color.mutedForeground,
        font: "inherit",
        textAlign: "start",
        cursor: "pointer",
    },
    composer: {
        display: "block",
        alignSelf: "stretch",
        marginInline: space[2],
        marginBlockEnd: space[2],
        fieldSizing: "content",
        paddingBlock: space[2],
        paddingInline: space[3],
        borderWidth: 0,
        borderRadius: radius[3],
        outlineStyle: "none",
        backgroundColor: color.card,
        boxShadow: CARD_SHADOW,
        color: color.foreground,
        font: "inherit",
        resize: "none",
        "::placeholder": { color: color.mutedForeground },
    },
});

/** How a board's cells order their cards: by hand, or sorted by the owner so moves only re-file. */
export type BoardOrder = "manual" | "sorted";

/** A cell of a board: a column, in a lane where the board has lanes. */
export interface BoardCell {
    /** The id of the column. */
    readonly column: string;
    /** The id of the lane, none on a board without lanes. */
    readonly lane: string | undefined;
}

/** A card or another view's item landing in a cell between two cards. */
export interface BoardMove extends BoardCell {
    /** The id of the card or item. */
    readonly id: string;
    /** Its type: a card's, or another view's for an item dropped in from there. */
    readonly type: string;
    /** The id of the card before it, undefined at the start. */
    readonly previous: string | undefined;
    /** The id of the card after it, undefined at the end. */
    readonly next: string | undefined;
}

/** The verbs of a board's moves. */
export const BoardMove = {
    /** Apply a move to the owner's lists of cards, keyed by cell as the owner keys them. */
    apply<Item>(
        lists: Readonly<Record<string, readonly Item[]>>,
        move: BoardMove,
        keyOf: (cell: BoardCell) => string,
        idOf: (item: Item) => string,
    ): Record<string, readonly Item[]> {
        return SortableMove.apply(lists, { ...move, container: keyOf(move) }, idOf);
    },
};

/** A column moving along the row between two columns. */
export interface BoardColumnMove {
    /** The id of the column. */
    readonly id: string;
    /** The id of the column before it, undefined at the start. */
    readonly previous: string | undefined;
    /** The id of the column after it, undefined at the end. */
    readonly next: string | undefined;
}

/** A card a person writes into a cell's composer, at its start or its end. */
export interface BoardDraft extends BoardCell {
    /** The card's title, as the person wrote it. */
    readonly title: string;
    /** Where in the cell it goes. */
    readonly position: BoardPosition;
}

/** Where in a cell a composer writes its card. */
export type BoardPosition = "start" | "end";

/** A card of a board as its control collects it. */
interface BoardCard {
    /** The card's id. */
    readonly key: string;
    /** The cell it stands in. */
    readonly cell: BoardCell;
    /** The id of its cell's sortable container. */
    readonly cellId: string;
    /** The card's text, which typeahead and announcements read. */
    readonly text: Accessor<string>;
    /** The card's element, which orders it among the others. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** A column or a lane of a board, which its headers and cells read. */
export interface BoardTrack {
    /** The track's id. */
    readonly key: string;
    /** The track's name. */
    readonly label: Accessor<string>;
    /** The track's element, which orders it among the others. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** A column of a board: a track with a limit of cards. */
export interface BoardColumnTrack extends BoardTrack {
    /** The most cards the column should take, none without a limit. */
    readonly limit: Accessor<number | undefined>;
}

/** A cell's virtual list: its window, and its cards in order, mounted or not. */
interface BoardWindow {
    /** The cell's id. */
    readonly cellId: string;
    /** The virtual list's window of cards. */
    readonly window: VirtualWindow;
    /** The cell's cards in order, those out of view by their text alone. */
    readonly cards: Accessor<readonly BoardCard[]>;
}

/** A hidden column's row in the hidden list. */
interface BoardHiddenRow {
    /** The column's key. */
    readonly column: string;
    /** The row's element, which drags aim at. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** What opened a composer, which takes the focus back as it closes: a card by c, or a button. */
export type BoardOpener =
    | { readonly kind: "card"; readonly key: string; readonly nearest: string | undefined }
    | { readonly kind: "button"; readonly button: () => HTMLElement };

/** The open composer, and what opened it. */
export interface BoardComposition {
    /** The composer's id, by its cell and position. */
    readonly composer: string;
    /** What opened it. */
    readonly opener: BoardOpener;
}

/** The properties of a board, the native element's attributes included. */
export interface BoardProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Handle a person moving a card into a cell between two cards, once per card moved. */
    readonly onItemMove?: (move: BoardMove) => void;
    /** Handle a person moving a column between two columns, absent to fix the columns in place. */
    readonly onColumnMove?: (move: BoardColumnMove) => void;
    /** Report whether a card may move into a column, every column by default. */
    readonly isAllowed?: (item: string, column: string) => boolean;
    /** Handle a person opening a card with a plain press or Enter. */
    readonly onAction?: (key: string) => void;
    /** Handle a person removing cards with Delete or the card menu. */
    readonly onItemRemove?: (keys: readonly string[]) => void;
    /** Handle a person writing a new card into a cell's composer. */
    readonly onCreate?: (draft: BoardDraft) => void;
    /** The types of other views' items the board takes, which it hands to its receiver. */
    readonly dropTypes?: readonly string[];
    /** Handle an item of another view dropped into a cell. */
    readonly onReceive?: (move: BoardMove) => void;
    /** How the cells order their cards, by hand by default. */
    readonly order?: BoardOrder;
    /** The selected cards, controlled or the board's, replacing on a plain press by default. */
    readonly selection?: MultipleSelection;
    /** The hidden columns, listed at the end of the columns' row, controlled. */
    readonly hiddenColumns?: readonly string[];
    /** The columns hidden at first, uncontrolled. */
    readonly defaultHiddenColumns?: readonly string[];
    /** Report the hidden columns as a person hides or shows one. */
    readonly onHiddenColumnsChange?: (columns: readonly string[]) => void;
    /** The collapsed lanes, controlled. */
    readonly collapsedLanes?: readonly string[];
    /** The lanes collapsed at first, uncontrolled. */
    readonly defaultCollapsedLanes?: readonly string[];
    /** Report the collapsed lanes as a person collapses or expands one. */
    readonly onCollapsedLanesChange?: (lanes: readonly string[]) => void;
    /** The name announcements call the row of columns by. */
    readonly label?: string;
}

/** The properties of a board's column, the native element's attributes included. */
export interface BoardColumnProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The column's id, which a card's move reports. */
    readonly id: string;
    /** The column's name, which announcements call it by. */
    readonly label: string;
    /** The most cards the column should take, after kanban's work-in-progress limits. */
    readonly limit?: number;
}

/** The properties of a board's lane, the native element's attributes included. */
export interface BoardLaneProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The lane's id, which a card's move reports. */
    readonly id: string;
    /** The lane's name, which announcements call its cells by. */
    readonly label: string;
}

/** The properties of a cell's cards, the native element's attributes included. */
export interface BoardColumnContentProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Read a card's text by its id, which a virtual list's cards out of view need. */
    readonly textOf?: (id: string) => string;
}

/** The properties of a lane's cell, the native element's attributes included. */
export interface BoardLaneCellProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The id of the column the cell stands under. */
    readonly column: string;
}

/** The properties of a cell's composer, the native button's attributes included. */
export interface BoardComposerProperties extends SortableElementProperties<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>
> {
    /** Where the new card goes, the cell's end by default. */
    readonly position?: BoardPosition;
}

/** The properties of a board's card, the native element's attributes included. */
export interface BoardItemProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The card's id, which a move reports. */
    readonly id: string;
    /** The name announcements call it by, its text by default. */
    readonly label?: string;
    /** Whether the card stays where it is. */
    readonly disabled?: boolean;
    /** The element that lifts the card: the whole card by default, or its handle. */
    readonly activator?: SortableActivator;
}

/** The cards of a board by cell, the one focused card, the selection, composers and the menu. */
export class BoardControl {
    /** The cards of cells without a virtual list, in document order. */
    readonly cards: CollectionBuilder<BoardCard>;
    /** The mounted cards of cells with a virtual list, which their elements come from. */
    readonly #mounted = new Map<string, BoardCard>();
    /** The columns in document order. */
    readonly columns: CollectionBuilder<BoardColumnTrack>;
    /** The lanes in document order, none on a board without lanes. */
    readonly lanes: CollectionBuilder<BoardTrack>;
    /** The cards by cell, one section per cell, lane by lane. */
    readonly collection: Collection<BoardCard>;
    /** The focused card, the whole board's one tab stop. */
    readonly focus: Focus<string>;
    /** The selected cards, which bulk actions apply to. */
    readonly selection: Selection;
    /** The hidden columns. */
    readonly hiddenColumns: Accessor<readonly string[]>;
    /** Hide or show columns. */
    readonly setHiddenColumns: (columns: readonly string[]) => void;
    /** The collapsed lanes. */
    readonly collapsedLanes: Accessor<readonly string[]>;
    /** Collapse or expand lanes. */
    readonly setCollapsedLanes: (lanes: readonly string[]) => void;
    /** The open composer, by its cell and position, none while every composer is closed. */
    readonly composing: Accessor<BoardComposition | undefined>;
    /** Open a composer by its cell and position, or close it. */
    readonly setComposing: (composing: BoardComposition | undefined) => void;
    /** The card menu a right click opens on a card. */
    readonly menu: MenuControl;
    /** The card the open card menu opened on. */
    readonly menuKey: Accessor<string | undefined>;
    /** The board's properties, read for its handlers. */
    readonly properties: BoardProperties;
    /** The board's id, which tells its containers from another board's. */
    readonly #id: string;
    /** The drag type of the board's cards, which no other board's lists take. */
    readonly cardType: string;
    /** The drag type of the board's columns, which no other board's row takes. */
    readonly columnType: string;
    /** The ids of the shown cells. */
    readonly #shownCellIds: Accessor<ReadonlySet<string>>;
    /** The keys of each cell's cards in document order, by cell id. */
    readonly #byCell: Accessor<ReadonlyMap<string, readonly string[]>>;
    /** The cells' virtual lists by cell id. */
    readonly #windows: CollectionBuilder<BoardWindow>;
    /** The mounted composers. */
    readonly #composers: CollectionBuilder<string>;
    /** The mounted list of hidden columns, none or one. */
    readonly #hiddenLists: CollectionBuilder<object>;
    /** The hidden list's row of each hidden column, which drags aim at, none without a list. */
    readonly #hiddenRows: CollectionBuilder<BoardHiddenRow>;
    /** The column menus mounted, whose Hide needs a hidden list. */
    readonly #columnMenus: CollectionBuilder<object>;
    /** The number of mounted card menus, the board opening one only while one is mounted. */
    #menus = 0;
    /** Set the card the card menu opened on. */
    readonly #setMenuKey: (key: string | undefined) => void;
    /** The locale the actions' titles are written in. */
    readonly #locale: ReturnType<typeof useLocale>;

    /** Collect a board's cards, columns and lanes as they mount. */
    constructor(properties: BoardProperties, locale: ReturnType<typeof useLocale>) {
        // collect cards, columns and lanes in document order, a remounted one replacing its earlier
        this.properties = properties;
        this.#locale = locale;
        this.#id = createUniqueId();
        this.cardType = `${this.#id} ${CARD_TYPE}`;
        this.columnType = `${this.#id} ${COLUMN_TYPE}`;
        this.cards = new CollectionBuilder(
            (card) => card.element(),
            (card) => card.key,
        );
        this.columns = new CollectionBuilder(
            (column) => column.element(),
            (column) => column.key,
        );
        this.lanes = new CollectionBuilder(
            (lane) => lane.element(),
            (lane) => lane.key,
        );

        // index the cards by cell
        this.#byCell = createMemo(() => {
            const cells = new Map<string, string[]>();
            for (const card of this.cards.items()) {
                const keys = cells.get(card.cellId) ?? [];
                keys.push(card.key);
                cells.set(card.cellId, keys);
            }

            return cells;
        });
        this.#windows = new CollectionBuilder(undefined, (entry) => entry.cellId);
        this.#shownCellIds = createMemo(
            () => new Set(this.shownCells().map((cell) => this.cellId(cell))),
        );
        this.collection = new Collection({
            sections: () => {
                // list each cell's cards from the index, the cards found by key once
                const cards = new Map(this.cards.items().map((card) => [card.key, card]));

                return this.shownCells().map((cell) => {
                    // take a virtual list's cards, mounted or not, else the mounted cards
                    const cellId = this.cellId(cell);
                    const items =
                        this.#windows.item(cellId)?.cards() ??
                        this.keysIn(cell).map((key) =>
                            present(cards.get(key), `the board card '${key}'`),
                        );

                    return { key: cellId, label: this.labelOf(cell), items };
                });
            },
            key: (card) => card.key,
            text: (card) => card.text(),
        });

        // follow the collapsed columns and lanes, controlled or the board's
        [this.hiddenColumns, this.setHiddenColumns] = createControllableSignal<readonly string[]>({
            isControlled: () => properties.hiddenColumns !== undefined,
            value: () => properties.hiddenColumns ?? [],
            defaultValue: untrack(() => properties.defaultHiddenColumns ?? []),
            onChange: (columns) => properties.onHiddenColumnsChange?.(columns),
        });
        [this.collapsedLanes, this.setCollapsedLanes] = createControllableSignal<readonly string[]>(
            {
                isControlled: () => properties.collapsedLanes !== undefined,
                value: () => properties.collapsedLanes ?? [],
                defaultValue: untrack(() => properties.defaultCollapsedLanes ?? []),
                onChange: (lanes) => properties.onCollapsedLanesChange?.(lanes),
            },
        );

        // move one focused card across the shown cells, and select several
        this.focus = new Focus({
            delegate: new BoardDelegate(this),
            mode: "roving",
            reveal: (key) => this.#reveal(key),
        });
        this.selection = new Selection(
            untrack(() => properties.selection) ?? { multiple: true, behavior: "replace" },
        );

        // open one composer at a time, and one card menu for every card
        [this.composing, this.setComposing] = createSignal<BoardComposition | undefined>(undefined);
        this.#composers = new CollectionBuilder(undefined, (id) => id);
        [this.menuKey, this.#setMenuKey] = createSignal<string | undefined>(undefined);

        // follow the hidden list's rows and count the column menus as they mount
        this.#hiddenLists = new CollectionBuilder();
        this.#hiddenRows = new CollectionBuilder(undefined, (row) => row.column);
        this.#columnMenus = new CollectionBuilder();
        this.menu = new MenuControl(null, {
            onOpenChange: (isOpen) => (isOpen ? undefined : this.#setMenuKey(undefined)),
        });
    }

    /** Read the id of the board's row of columns, unique among boards. */
    columnsId(): string {
        return `${this.#id} ${COLUMNS}`;
    }

    /** Read the cell of a sortable container id, none for another board's container. */
    cellOfId(id: string): BoardCell | undefined {
        // read the board, lane and column the id was written from
        const parsed: unknown = id.startsWith("[") ? JSON.parse(id) : undefined;
        if (!Array.isArray(parsed)) {
            return undefined;
        }
        const board: unknown = parsed[0];
        const lane: unknown = parsed[1];
        const column: unknown = parsed[2];
        if (board !== this.#id) {
            return undefined;
        }

        return typeof column === "string" && (lane === null || typeof lane === "string")
            ? { column, lane: lane ?? undefined }
            : undefined;
    }

    /** Read the group a lane's cells share, which a lifted card crosses within by keys. */
    laneGroup(lane: string | undefined): string {
        return JSON.stringify([this.#id, lane ?? null]);
    }

    /** Read the sortable container id of a cell, unique among boards. */
    cellId(cell: BoardCell): string {
        return JSON.stringify([this.#id, cell.lane ?? null, cell.column]);
    }

    /** List the board's cells, lane by lane, each lane's columns in order. */
    cells(): BoardCell[] {
        const lanes = this.lanes.items();
        const columns = this.columns.items();

        return lanes.length === 0
            ? columns.map((column) => ({ column: column.key, lane: undefined }))
            : lanes.flatMap((lane) =>
                  columns.map((column) => ({ column: column.key, lane: lane.key })),
              );
    }

    /** List the cells that show: those outside collapsed columns and lanes. */
    shownCells(): BoardCell[] {
        return this.cells().filter(
            (cell) =>
                !this.isHidden(cell.column) &&
                (cell.lane === undefined || !this.isLaneCollapsed(cell.lane)),
        );
    }

    /** List the keys of a cell's cards in order, a virtual list's out of view included. */
    keysIn(cell: BoardCell): readonly string[] {
        const id = this.cellId(cell);

        return this.#windows.item(id)?.window.keys() ?? this.#byCell().get(id) ?? [];
    }

    /** Report whether a card shows and takes the focus: one outside collapsed tracks. */
    isShown(key: string): boolean {
        const cellId = this.cellIdOf(key);

        return cellId !== undefined && this.#shownCellIds().has(cellId);
    }

    /** Read the id of a card's cell, a virtual list's card out of view included. */
    cellIdOf(key: string): string | undefined {
        // find the card among the virtual lists' cards, else among the mounted cards
        for (const each of this.#windows.items()) {
            if (each.window.positions().has(key)) {
                return each.cellId;
            }
        }

        return this.cards.item(key)?.cellId;
    }

    /** Read a card's place among its virtual list's cards and their number, none elsewhere. */
    placeOf(key: string): { readonly index: number; readonly size: number } | undefined {
        for (const each of this.#windows.items()) {
            const index = each.window.positions().get(key);
            if (index !== undefined) {
                return { index, size: each.window.positions().size };
            }
        }

        return undefined;
    }

    /** List the cards a cell's virtual list mounts out of view: the focused card. */
    persistedIn(cell: BoardCell): readonly string[] {
        const key = this.focus.current();
        const each = this.#windows.item(this.cellId(cell));

        return key !== undefined && each?.window.positions().has(key) === true ? [key] : [];
    }

    /** Add a column until it unmounts. */
    addColumn(column: BoardColumnTrack): void {
        this.columns.add(column);
    }

    /** Add a lane until it unmounts. */
    addLane(lane: BoardTrack): void {
        this.lanes.add(lane);
    }

    /** Add a card until it unmounts, a virtual list's card among its mounted ones. */
    addCard(card: BoardCard): void {
        // add a virtual list's card apart, refusing a card its list lacks
        const windowed = untrack(() => this.#windows.item(card.cellId));
        if (windowed !== undefined && untrack(() => !windowed.window.positions().has(card.key))) {
            throw new TypeError(`board card '${card.key}' is not among its virtual list's keys`);
        }
        if (windowed === undefined) {
            this.cards.add(card);
        } else {
            this.#mounted.set(card.key, card);
            onCleanup(() => {
                if (this.#mounted.get(card.key) === card) {
                    this.#mounted.delete(card.key);
                }
            });
        }

        // focus the card's element as the focus reaches it
        this.focus.bind(() => card.key, card.element);
    }

    /** Add a cell's virtual list until it unmounts, its cards out of view read by their text. */
    addWindow(cell: BoardCell, window: VirtualWindow, textOf: (id: string) => string): void {
        // stand each card in by its key and text, with its element once it mounts
        const cellId = this.cellId(cell);
        const cards = createMemo(() =>
            window.keys().map((key) => ({
                key,
                cell,
                cellId,
                text: () => textOf(key),
                element: () => this.#mounted.get(key)?.element(),
            })),
        );

        // add the list under its cell until it unmounts
        this.#windows.add({ cellId, window, cards });
    }

    /** Add a cell's composer at a position until it unmounts. */
    addComposer(cell: BoardCell, position: BoardPosition): void {
        this.#composers.add(this.composerId(cell, position));
    }

    /** Count a card menu until it unmounts, which lets a right click open it. */
    addMenu(): void {
        this.#menus += 1;
        onCleanup(() => {
            this.#menus -= 1;
        });
    }

    /** Read the id of a cell's composer at a position. */
    composerId(cell: BoardCell, position: BoardPosition): string {
        return `${this.cellId(cell)} ${position}`;
    }

    /** Read the composer a column's new button opens: its first shown cell's top, else its end. */
    newComposerIn(column: string): string | undefined {
        return this.shownCells()
            .flatMap((cell) =>
                cell.column === column
                    ? [this.composerId(cell, "start"), this.composerId(cell, "end")]
                    : [],
            )
            .find((id) => this.#hasComposer(id));
    }

    /** Add the board's list of hidden columns until it unmounts. */
    addHiddenList(): void {
        this.#hiddenLists.add({});
    }

    /** Add a column menu until it unmounts, whose Hide needs a list of hidden columns. */
    addColumnMenu(): void {
        this.#columnMenus.add({});
    }

    /** Read the number of lists of hidden columns. */
    hiddenListCount(): number {
        return this.#hiddenLists.size();
    }

    /** Report whether columns hide, by a menu or by the owner, with no hidden list to go into. */
    isMissingHiddenList(): boolean {
        const isHiding = this.#columnMenus.size() > 0 || this.hiddenColumns().length > 0;

        return isHiding && this.#hiddenLists.size() === 0;
    }

    /** Read a column by its key, refusing a key of none. */
    columnOf(key: string): BoardColumnTrack {
        const column = this.columns.item(key);
        if (column === undefined) {
            throw new TypeError(`board column '${key}' is not on the board`);
        }

        return column;
    }

    /** Read a lane by its key, refusing a key of none. */
    laneOf(key: string): BoardTrack {
        const lane = this.lanes.item(key);
        if (lane === undefined) {
            throw new TypeError(`board lane '${key}' is not on the board`);
        }

        return lane;
    }

    /** Read the name announcements call a cell by: its column's, with its lane's. */
    labelOf(cell: BoardCell): string {
        const column = this.columnOf(cell.column).label();

        return cell.lane === undefined
            ? column
            : this.#locale.render(t`${column} for ${this.laneOf(cell.lane).label()}`);
    }

    /** Count a column's cards across its lanes, a long cell's out of view included. */
    count(column: string): number {
        return this.cells()
            .filter((cell) => cell.column === column)
            .reduce((total, cell) => total + this.keysIn(cell).length, 0);
    }

    /** Report whether a column has more cards than its limit. */
    isOver(column: string): boolean {
        const limit = this.columnOf(column).limit();

        return limit !== undefined && this.count(column) > limit;
    }

    /** Report whether a column is hidden. */
    isHidden(column: string): boolean {
        return this.hiddenColumns().includes(column);
    }

    /** Report whether a lane is collapsed. */
    isLaneCollapsed(lane: string): boolean {
        return this.collapsedLanes().includes(lane);
    }

    /** Hide a column into the hidden list, or show it. */
    toggleHidden(column: string): void {
        const hidden = this.hiddenColumns();
        this.setHiddenColumns(
            hidden.includes(column)
                ? hidden.filter((each) => each !== column)
                : [...hidden, column],
        );
    }

    /** Add a hidden column's row in the hidden list until it unmounts. */
    addHiddenRow(column: string, element: Accessor<HTMLElement | undefined>): void {
        this.#hiddenRows.add({ column, element });
    }

    /** Read the element drags aim at to land in a cell: its hidden column's row while hidden. */
    targetOf(cell: BoardCell, element: HTMLElement): HTMLElement {
        // aim at the cell itself while its column shows
        if (!this.isHidden(cell.column)) {
            return element;
        }

        return present(
            untrack(() => this.#hiddenRows.item(cell.column))?.element(),
            `the hidden list's row of column '${cell.column}'`,
        );
    }

    /** Report whether a cell takes an item: a hidden cell in a lane only that lane's cards. */
    takes(cell: BoardCell, key: string): boolean {
        // refuse a card the owner bars from the column
        const isCard = this.collection.has(key);
        if (isCard && !this.isAllowed(key, cell.column)) {
            return false;
        }
        // let a hidden cell in a lane take only that lane's cards, one row standing for all
        else if (this.isHidden(cell.column) && cell.lane !== undefined) {
            const from = isCard ? this.cellIdOf(key) : undefined;

            return from !== undefined && this.cellOfId(from)?.lane === cell.lane;
        }

        return true;
    }

    /** Collapse a lane, or expand it. */
    toggleLane(lane: string): void {
        const collapsed = this.collapsedLanes();
        this.setCollapsedLanes(
            collapsed.includes(lane)
                ? collapsed.filter((each) => each !== lane)
                : [...collapsed, lane],
        );
    }

    /** Select or open a pressed card: open on a plain press, select with a modifier. */
    press(key: string, gesture: InteractionEvent): void {
        // select with command, control or shift, or by any press on a board that opens nothing
        if (this.selection.isSelecting(gesture, this.properties.onAction !== undefined)) {
            this.selection.select(key, gesture, this.collection);
        }
        // open the card
        else {
            this.properties.onAction?.(key);
        }
    }

    /** Take a key on the board: select every shown card, clear, or move the focus. */
    keyDown(event: KeyboardEvent, direction: Direction, isCommand: boolean): void {
        // leave keys from outside the cards, such as a menu's, and keys a card, drag or field took
        const focused = this.focus.current();
        const isFromCard =
            event.target instanceof Element &&
            event.target.closest("[data-slot=board-item]") !== null;
        if (
            !isFromCard ||
            event.defaultPrevented ||
            Action.isTyping(event) ||
            Action.handle(this.actions(), event, isCommand)
        ) {
            return;
        }

        // move the focus across the shown cells, extending the selection with shift
        const target = this.focus.move(event, direction);
        if (target !== undefined && focused !== undefined && event.shiftKey) {
            this.selection.extendFrom(focused, target, this.collection);
        }
    }

    /** List what keys do anywhere on the board: select every shown card, or clear the selection. */
    actions(): Action[] {
        const locale = this.#locale;

        return [
            {
                id: "select-all",
                title: locale.render(t`Select all`),
                shortcut: "mod+a",
                run: () =>
                    this.selection.replace(this.shownCells().flatMap((cell) => this.keysIn(cell))),
            },
            ...(this.selection.values().length === 0
                ? []
                : [
                      {
                          id: "clear",
                          title: locale.render(t`Clear selection`),
                          shortcut: "escape",
                          run: () => this.selection.clear(),
                      },
                  ]),
        ];
    }

    /** List the cards an action on a card takes: the selection it is in, else the card. */
    targetsOf(key: string): string[] {
        return this.selection.isSelected(key)
            ? this.selection.values().filter((each) => this.isShown(each))
            : [key];
    }

    /** List what keys do on a card. */
    cardActions(key: string, direction: Direction): Action[] {
        // toggle the card, write a card into its cell, and remove the cards
        const locale = this.#locale;
        const options = this.properties;
        const targets = this.targetsOf(key);
        const cell = this.#cardOf(key).cell;
        const composer = this.#composerIn(cell);
        const editing: Action[] = [
            {
                id: "toggle",
                title: locale.render(t`Select`),
                shortcut: "x",
                run: () => this.selection.toggle(key),
            },
            ...(options.onCreate === undefined || composer === undefined
                ? []
                : [
                      {
                          id: "create",
                          title: locale.render(t`Add a card`),
                          shortcut: "c",
                          run: () => {
                              this.setComposing({
                                  composer,
                                  opener: { kind: "card", key, nearest: this.#survivor([key]) },
                              });
                          },
                      },
                  ]),
            ...this.#removing(targets),
        ];

        // move the cards a column over, and a lane over on a board with lanes
        const isRtl = direction === "rtl";
        const steps = [
            { id: "move-left", title: t`Move left`, key: "left", column: isRtl ? 1 : -1, lane: 0 },
            {
                id: "move-right",
                title: t`Move right`,
                key: "right",
                column: isRtl ? -1 : 1,
                lane: 0,
            },
            ...(cell.lane === undefined
                ? []
                : [
                      { id: "move-up", title: t`Move up`, key: "up", column: 0, lane: -1 },
                      { id: "move-down", title: t`Move down`, key: "down", column: 0, lane: 1 },
                  ]),
        ];
        const moving: Action[] =
            options.onItemMove === undefined
                ? []
                : steps.map((step) => ({
                      id: step.id,
                      title: locale.render(step.title),
                      shortcut: `shift+alt+${step.key}`,
                      run: () => this.step(key, targets, step.column, step.lane),
                  }));

        return [...editing, ...moving];
    }

    /** List what the card menu offers: open the card, move the cards, or delete them. */
    menuActions(key: string): Action[] {
        // open a lone card, and move the cards to each other column that takes them all
        const locale = this.#locale;
        const options = this.properties;
        const targets = this.targetsOf(key);
        const from = this.#cardOf(key).cell;
        const opening: Action[] =
            options.onAction === undefined || targets.length > 1
                ? []
                : [
                      {
                          id: "open",
                          title: locale.render(t`Open`),
                          shortcut: "enter",
                          run: () => options.onAction?.(key),
                      },
                  ];
        const moving: Action[] =
            options.onItemMove === undefined
                ? []
                : this.columns
                      .items()
                      .filter(
                          (column) =>
                              column.key !== from.column &&
                              targets.every((each) => this.isAllowed(each, column.key)),
                      )
                      .map((column) => ({
                          id: `move-to-${column.key}`,
                          title: locale.render(t`Move to ${column.label()}`),
                          run: () => {
                              this.place(targets, { column: column.key, lane: from.lane });
                              this.refocus(() => key);
                          },
                      }));

        return [...opening, ...moving, ...this.#removing(targets)];
    }

    /** Open the card menu at a point for a card, selecting it alone unless it is selected. */
    openMenu(key: string, element: HTMLElement, x: number, y: number): boolean {
        // open nothing without a mounted menu or anything to offer, leaving the browser's menu
        if (this.#menus === 0 || this.menuActions(key).length === 0) {
            return false;
        }

        // act on the selection a selected card is in, else on the card alone
        if (!this.selection.isSelected(key)) {
            this.selection.replace([key]);
        }
        this.focus.focus(key);
        this.#setMenuKey(key);

        // open the menu at the pointer, its focus returning to the card
        this.menu.setTrigger(element);
        this.menu.open("first", { x, y });

        return true;
    }

    /** Report whether the board lets a card into a column. */
    isAllowed(key: string, column: string): boolean {
        return this.properties.isAllowed?.(key, column) !== false;
    }

    /** Move cards a column or a lane over from a card, past hidden and refusing columns. */
    step(key: string, keys: readonly string[], columns: number, lanes: number): void {
        // read where the card stands, and the next shown column that takes a card, or lane
        const from = this.#cardOf(key).cell;
        const column =
            columns === 0
                ? from.column
                : nextShown(
                      this.columns.items().map((each) => each.key),
                      from.column,
                      columns,
                      (each) =>
                          this.isHidden(each) ||
                          !keys.some((target) => this.isAllowed(target, each)),
                  );
        const lane =
            lanes === 0 || from.lane === undefined
                ? from.lane
                : nextShown(
                      this.lanes.items().map((each) => each.key),
                      from.lane,
                      lanes,
                      (each) => this.isLaneCollapsed(each),
                  );

        // place the cards at the card's row, staying put at the board's edges
        if (column === undefined || (from.lane !== undefined && lane === undefined)) {
            return;
        }
        this.place(keys, { column, lane }, this.keysIn(from).indexOf(key));
    }

    /** Move cards into a cell before a row, at its end without one, those it lets in. */
    place(keys: readonly string[], cell: BoardCell, row?: number): void {
        // place the cards among the cell's other cards
        const moving = new Set(keys);
        const others = this.keysIn(cell).filter((key) => !moving.has(key));
        const at = row === undefined ? others.length : Math.min(row, others.length);
        this.#emit(keys, cell, others[at - 1], others[at]);
    }

    /** Hand on a drop: a column's move, cards' moves with the selection, or another view's item. */
    drop(move: SortableMove): void {
        // hand a column's move on as it is
        const cell = this.cellOfId(move.container);
        if (move.container === this.columnsId()) {
            this.properties.onColumnMove?.({
                id: move.id,
                previous: move.previous,
                next: move.next,
            });
        }
        // refuse a drop into a container of no cell, which the board's containers rule out
        else if (cell === undefined) {
            throw new TypeError(`board drop into an unknown cell '${move.container}'`);
        }
        // hand another view's item on with the cell and its neighbours
        else if (move.type !== this.cardType) {
            this.properties.onReceive?.({
                id: move.id,
                type: move.type,
                ...cell,
                previous: move.previous,
                next: move.next,
            });
        }
        // re-file sorted cards only when one changes cell, the owner placing them by its order
        else if (
            this.properties.order === "sorted" &&
            this.targetsOf(move.id).every((key) => this.#cardOf(key).cellId === move.container)
        ) {
            return;
        }
        // move the card with the selection it is in
        else {
            const keys = this.targetsOf(move.id);
            const moving = new Set(keys);
            const order = this.keysIn(cell);
            const others = order.filter((key) => !moving.has(key));
            const next = nextOutside(move.next, moving, order);
            const previous = next === undefined ? others.at(-1) : others[others.indexOf(next) - 1];
            this.#emit(keys, cell, previous, next);
        }
    }

    /** Write a new card into a cell. */
    create(cell: BoardCell, title: string, position: BoardPosition): void {
        this.properties.onCreate?.({ ...cell, title, position });
    }

    /** List the action that removes cards, on a board that hands removals on. */
    #removing(targets: readonly string[]): Action[] {
        const options = this.properties;

        return options.onItemRemove === undefined || targets.length === 0
            ? []
            : [
                  {
                      id: "delete",
                      title: this.#locale.render(t`Delete`),
                      shortcut: "delete",
                      isDestructive: true,
                      run: () => {
                          // focus the card that stays nearest the removed ones, then remove them
                          const next = this.#survivor(targets);
                          options.onItemRemove?.(targets);
                          this.refocus(() => next);
                      },
                  },
              ];
    }

    /** Read the shown card nearest removed cards: after them, before them, else the first. */
    #survivor(targets: readonly string[]): string | undefined {
        // look in the first card's cell, after it and then before it
        const [first] = targets;
        const removing = new Set(targets);
        const order = first === undefined ? [] : this.keysIn(this.#cardOf(first).cell);
        const index = first === undefined ? 0 : order.indexOf(first);
        const after = order.slice(index + 1).find((key) => !removing.has(key));
        const before = order.slice(0, index).findLast((key) => !removing.has(key));

        // fall back to the first shown card the removal leaves
        return after ?? before ?? this.collection.keys().find((key) => !removing.has(key));
    }

    /** Report whether a composer has an id. */
    #hasComposer(id: string): boolean {
        return this.#composers.item(id) !== undefined;
    }

    /** Read the composer the c key opens in a cell: its end's, else its start's. */
    #composerIn(cell: BoardCell): string | undefined {
        return [this.composerId(cell, "end"), this.composerId(cell, "start")].find((id) =>
            this.#hasComposer(id),
        );
    }

    /** Report each card's move into a cell between two cards, those the cell lets in, in order. */
    #emit(
        keys: readonly string[],
        cell: BoardCell,
        previous: string | undefined,
        next: string | undefined,
    ): void {
        // refuse keys the board does not show, and leave out cards the column refuses
        const moving = new Set(keys);
        for (const key of keys) {
            if (!this.isShown(key)) {
                throw new TypeError(`board card '${key}' is not on the board`);
            }
        }
        const ordered = this.shownCells()
            .flatMap((each) => this.keysIn(each))
            .filter((key) => moving.has(key) && this.isAllowed(key, cell.column));

        // move the cards one after another, each after the one before it
        let before = previous;
        for (const key of ordered) {
            this.properties.onItemMove?.({
                id: key,
                type: CARD_TYPE,
                ...cell,
                previous: before,
                next,
            });
            before = key;
        }
    }

    /** Move the focus to a card, scrolling a virtual list's card into view to mount it. */
    focusCard(key: string): void {
        this.focus.enter(key);
        this.focus.reveal(key);
        this.#cardOf(key).element()?.focus();
    }

    /** Focus the card a target reads once the owner's change has rendered, where it stands. */
    refocus(target: () => string | undefined): void {
        queueMicrotask(() => {
            // render the owner's change, then focus the card the target reads now
            flush();
            const key = target();
            if (key !== undefined && this.collection.has(key)) {
                this.focusCard(key);
            }
        });
    }

    /** Focus a composer's opening card once it closes, else the card nearest it as it opened. */
    refocusOpener(key: string, nearest: string | undefined): void {
        this.refocus(() => {
            // focus the opener where it still stands
            if (this.collection.has(key)) {
                return key;
            }
            // focus the nearest card where it still stands
            else if (nearest !== undefined && this.collection.has(nearest)) {
                return nearest;
            }
            // else focus the first card on the board
            else {
                return this.collection.keys()[0];
            }
        });
    }

    /** Bring a virtual list's card into view, which mounts it to take the focus. */
    #reveal(key: string): void {
        for (const each of this.#windows.items()) {
            if (each.window.positions().has(key)) {
                each.window.reveal(key);
            }
        }
    }

    /** Read a card by its key, refusing a key of none. */
    #cardOf(key: string): BoardCard {
        const card = this.collection.item(key);
        if (card === undefined) {
            throw new TypeError(`board card '${key}' is not on the board`);
        }

        return card;
    }
}

/** The keys that move the focus on a board: along a cell, across columns, and across lanes. */
class BoardDelegate implements KeyboardDelegate<string> {
    /** The board, whose shown cells the keys follow. */
    readonly #control: BoardControl;
    /** The shown cards' keys by lane and column, read once per change to the cards. */
    readonly #grid: Accessor<readonly (readonly (readonly string[])[])[]>;

    /** Follow a board's shown cells. */
    constructor(control: BoardControl) {
        // lay the shown cells out lane by lane, column by column
        this.#control = control;
        this.#grid = createMemo(() => {
            // read the shown lanes, one row without lanes, and the shown columns
            const lanes = control.lanes.items().map((lane) => lane.key);
            const rows: (string | undefined)[] =
                lanes.length === 0
                    ? [undefined]
                    : lanes.filter((lane) => !control.isLaneCollapsed(lane));
            const columns = control.columns
                .items()
                .map((column) => column.key)
                .filter((column) => !control.isHidden(column));

            return rows.map((lane) => columns.map((column) => control.keysIn({ column, lane })));
        });
    }

    /** Read the key a key press moves the focus to. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // find the cell and row the move starts from
        const grid = this.#grid();
        const place = placeIn(grid, from);
        if (place === undefined) {
            return undefined;
        }
        const { lane, column, row } = place;
        const keys = present(grid[lane]?.[column], "the focused card's cell");
        const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

        // step along the cell, past its ends into the same column of the lanes around it
        if (event.key === "ArrowDown") {
            return keys[row + 1] ?? nearestLane(grid, lane, column, 1)?.[0];
        }
        // step up the cell, past its start into the lane above
        else if (event.key === "ArrowUp") {
            return keys[row - 1] ?? nearestLane(grid, lane, column, -1)?.at(-1);
        }
        // jump to the cell's start
        else if (event.key === "Home") {
            return keys[0];
        }
        // jump to the cell's end
        else if (event.key === "End") {
            return keys.at(-1);
        }
        // step across to the nearest column with cards in the lane, at the same row or its last
        else if (event.key === forward || event.key === backward) {
            const cells = grid[lane] ?? [];
            const step = event.key === forward ? 1 : -1;
            let index = column + step;
            while (index >= 0 && index < cells.length && (cells[index]?.length ?? 0) === 0) {
                index += step;
            }
            const neighbour = cells[index];

            return neighbour?.[Math.min(row, neighbour.length - 1)];
        }
        // leave other keys alone
        else {
            return undefined;
        }
    }

    /** Read the first shown card. */
    first(): string | undefined {
        return this.#grid().flat(2)[0];
    }

    /** Report whether a card shows and takes the focus. */
    has(key: string): boolean {
        return this.#control.isShown(key);
    }
}

/** The nearest board, null outside one. */
const BoardContext = createContext<BoardControl | null>(null);

/** The nearest column, which its header and cards read. */
const BoardColumnContext = createContext<BoardColumnTrack | null>(null);

/** The nearest lane, which its header and cells read. */
const BoardLaneContext = createContext<BoardTrack | null>(null);

/** The nearest cell, which its cards read. */
const BoardCellContext = createContext<BoardCell | null>(null);

/** Render a board of columns of cards, in lanes where it has them. */
export function Board(properties: BoardProperties): JSX.Element {
    // move one focus across the cards and run the board's keys, after the keys a drag takes
    const locale = useLocale();
    const control = new BoardControl(properties, locale);
    const isCommand = useCommandPlatform();

    // refuse a second hidden list once one replacing another has settled
    createEffect(
        () => control.hiddenListCount(),
        (count) => {
            if (count > 1) {
                throw new TypeError("a board has one BoardHiddenList, and this board has two");
            }
        },
    );

    // refuse columns that hide with no list to hide into, as menus mount and columns hide
    createEffect(
        () => control.isMissingHiddenList(),
        (isMissing) => {
            if (isMissing) {
                throw new TypeError(
                    "board columns hide into a BoardHiddenList, and the board has none",
                );
            }
        },
    );
    const keys = {
        onKeyDown: (event: KeyboardEvent) => control.keyDown(event, locale.direction, isCommand()),
        onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
            control.focus.focusOut(event),
        onFocusIn: (event: FocusEvent) => {
            // follow the focus off the cards onto another control inside the board
            const target = event.target instanceof Element ? event.target : null;
            if (target?.closest("[data-slot=board-item]") === null) {
                control.focus.focusAway();
            }
        },
    };
    const rest = omit(
        properties,
        "onItemMove",
        "onColumnMove",
        "onAction",
        "onItemRemove",
        "onCreate",
        "dropTypes",
        "onReceive",
        "order",
        "isAllowed",
        "selection",
        "hiddenColumns",
        "defaultHiddenColumns",
        "onHiddenColumnsChange",
        "collapsedLanes",
        "defaultCollapsedLanes",
        "onCollapsedLanesChange",
        "label",
        "children",
        "xstyle",
    );

    return (
        <BoardContext value={control}>
            <Menu control={control.menu}>
                <Sortable
                    data-slot="board"
                    onMove={(move) => control.drop(move)}
                    {...mergeProperties(keys, rest)}
                    xstyle={[styles.board, properties.xstyle]}
                >
                    {properties.children}
                </Sortable>
            </Menu>
        </BoardContext>
    );
}

/** Render the row of a board's columns, which move along it by their handles. */
export function BoardColumnList(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useBoard();
    const rest = omit(properties, "id", "xstyle");

    return (
        <SortableContainer
            id={control.columnsId()}
            data-slot="board-column-list"
            orientation="horizontal"
            accepts={[control.columnType]}
            {...(control.properties.label === undefined ? {} : { label: control.properties.label })}
            {...rest}
            xstyle={[styles.columns, properties.xstyle]}
        />
    );
}

/** Render the list of hidden columns, each a place cards drop. */
export function BoardHiddenList(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // list the hidden columns in the board's order, the list a place columns hide into
    const control = useBoard();
    control.addHiddenList();
    const rest = omit(properties, "xstyle", "style");
    const locale = useLocale();
    const { manager } = useSortable();
    const hidden = () => control.columns.items().filter((column) => control.isHidden(column.key));

    // mark the row a drag aims at
    const isAimed = (column: string) => {
        const target = manager.operation()?.target;
        const cell = target === undefined ? undefined : control.cellOfId(target.droppable);

        return cell?.column === column;
    };

    return (
        <Show when={hidden().length > 0}>
            <div
                role="group"
                aria-label={locale.render(t`Hidden columns`)}
                data-slot="board-hidden-list"
                {...rest}
                {...style.attributes(
                    [text.sm, styles.hiddenList, properties.xstyle],
                    properties.style,
                )}
            >
                <span {...style.attrs(styles.hiddenTitle)}>{locale.render(t`Hidden`)}</span>
                <For each={hidden()}>
                    {(column) => {
                        // add the row as the place drags aim at for its column, once it mounts
                        const [row, setRow] = createSignal<HTMLElement | undefined>(undefined, {
                            ownedWrite: true,
                        });
                        control.addHiddenRow(column.key, row);

                        return (
                            <div
                                data-slot="board-hidden-column"
                                ref={setRow}
                                data-column={column.key}
                                data-over={isAimed(column.key) ? "" : undefined}
                                {...style.attrs(
                                    styles.hiddenRow,
                                    isAimed(column.key) && styles.hiddenRowAimed,
                                )}
                            >
                                <span {...style.attrs(styles.hiddenLabel)}>{column.label()}</span>
                                <span {...style.attrs(styles.count)}>
                                    {locale.number(control.count(column.key))}
                                </span>
                                <button
                                    type="button"
                                    data-slot="board-column-show"
                                    aria-label={locale.render(t`Show ${column.label()}`)}
                                    onClick={() => control.toggleHidden(column.key)}
                                    {...style.attrs(state.hover, styles.toggle, styles.end)}
                                >
                                    <Icon icon={eye} />
                                </button>
                            </div>
                        );
                    }}
                </For>
            </div>
        </Show>
    );
}

/** Render a column of the nearest board: its header, and its cards on a board without lanes. */
export function BoardColumn(properties: BoardColumnProperties): JSX.Element {
    // add the column to the board's order of columns
    const control = useBoard();
    const rest = omit(properties, "id", "label", "limit", "xstyle", "children");
    let element: HTMLElement | undefined;
    const column: BoardColumnTrack = {
        key: untrack(() => properties.id),
        label: () => properties.label,
        limit: () => properties.limit,
        element: () => element,
    };
    control.addColumn(column);
    const id = column.key;

    return (
        <BoardColumnContext value={column}>
            <BoardCellContext value={{ column: id, lane: undefined }}>
                <SortableItem
                    id={id}
                    type={control.columnType}
                    disabled={control.properties.onColumnMove === undefined}
                    label={properties.label}
                    data-slot="board-column"
                    data-hidden={control.isHidden(id) ? "" : undefined}
                    data-over={control.isOver(id) ? "" : undefined}
                    {...mergeProperties(
                        { ref: (created: HTMLDivElement) => (element = created) },
                        rest,
                    )}
                    xstyle={[
                        styles.column,
                        control.lanes.size() > 0 && styles.columnBare,
                        control.isHidden(id) && styles.hidden,
                        control.isOver(id) && styles.over,
                        properties.xstyle,
                    ]}
                >
                    {properties.children}
                </SortableItem>
            </BoardCellContext>
        </BoardColumnContext>
    );
}

/** Render the header of the nearest column: its title and count, then its actions at the end. */
export function BoardColumnHeader(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="board-column-header"
            {...rest}
            {...style.attributes([text.sm, styles.header, properties.xstyle], properties.style)}
        />
    );
}

/** Render the nearest column's count of cards, against its limit where it has one. */
export function BoardColumnCount(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    // write the count, and the limit it should stay within
    const control = useBoard();
    const column = useBoardColumn();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");
    const count = () => {
        const cards = locale.number(control.count(column.key));
        const limit = column.limit();

        return limit === undefined ? cards : locale.render(t`${cards} of ${locale.number(limit)}`);
    };

    return (
        <span
            data-slot="board-column-count"
            data-over={control.isOver(column.key) ? "" : undefined}
            aria-label={
                control.isOver(column.key)
                    ? locale.render(t`${count()}, over the limit`)
                    : undefined
            }
            {...rest}
            {...style.attributes(
                [styles.count, control.isOver(column.key) && styles.countOver, properties.xstyle],
                properties.style,
            )}
        >
            {count()}
        </span>
    );
}

/** The properties of a column's menu. */
export interface BoardColumnMenuProperties {
    /** What else can be done with the column, listed after Hide. */
    readonly actions?: readonly Action[];
}

/** Render the nearest column's menu behind a button that shows on hover: Hide, and the owner's. */
export function BoardColumnMenu(properties: BoardColumnMenuProperties): JSX.Element {
    // offer to hide the column, then the owner's actions
    const control = useBoard();
    const column = useBoardColumn();
    const locale = useLocale();
    control.addColumnMenu();
    const actions = (): Action[] => [
        {
            id: "hide",
            title: locale.render(t`Hide`),
            run: () => control.toggleHidden(column.key),
        },
        ...(properties.actions ?? []),
    ];

    return (
        <DropdownMenu>
            <DropdownMenuTrigger
                render={(attributes) => (
                    <button
                        type="button"
                        {...attributes}
                        {...style.attrs(state.hover, styles.toggle, styles.action, styles.end)}
                    />
                )}
                aria-label={locale.render(t`${column.label()} options`)}
                data-slot="board-column-menu"
            >
                <Icon icon={dotsThree} />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
                <For each={actions()} keyed={(action) => action.id}>
                    {(action) => <MenuAction action={action()} />}
                </For>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/** Render the button that opens a new card at the top of the nearest column, shown on hover. */
export function BoardColumnAdd(
    properties: SortableElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    // open the column's top composer, else its end one, where the column has one
    const control = useBoard();
    const column = useBoardColumn();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");
    const composer = () => control.newComposerIn(column.key);
    let element: HTMLButtonElement | undefined;

    return (
        <Show when={composer()}>
            {(id) => (
                <button
                    type="button"
                    data-slot="board-column-add"
                    aria-label={locale.render(t`New card in ${column.label()}`)}
                    {...mergeProperties(
                        {
                            ref: (created: HTMLButtonElement) => (element = created),
                            onClick: () =>
                                control.setComposing({
                                    composer: id(),
                                    opener: {
                                        kind: "button",
                                        button: () => present(element, "the new card button"),
                                    },
                                }),
                        },
                        rest,
                    )}
                    {...style.attributes(
                        [state.hover, styles.toggle, styles.action, properties.xstyle],
                        properties.style,
                    )}
                >
                    {properties.children ?? <Icon icon={plus} />}
                </button>
            )}
        </Show>
    );
}

/** Render the grip that moves the nearest column along the row, in its header's start padding. */
export function BoardColumnHandle(properties: SortableHandleProperties): JSX.Element {
    return (
        <SortableHandle
            data-slot="board-column-handle"
            {...properties}
            xstyle={[styles.grip, properties.xstyle]}
        />
    );
}

/** Render a lane of the nearest board: its header, then a cell per column. */
export function BoardLane(properties: BoardLaneProperties): JSX.Element {
    // add the lane to the board's order of lanes
    const control = useBoard();
    const rest = omit(properties, "id", "label", "xstyle", "style", "children");
    let element: HTMLElement | undefined;
    const lane: BoardTrack = {
        key: untrack(() => properties.id),
        label: () => properties.label,
        element: () => element,
    };
    control.addLane(lane);

    return (
        <BoardLaneContext value={lane}>
            <div
                role="group"
                aria-label={properties.label}
                data-slot="board-lane"
                data-collapsed={control.isLaneCollapsed(lane.key) ? "" : undefined}
                {...mergeProperties(
                    { ref: (created: HTMLDivElement) => (element = created) },
                    rest,
                )}
                {...style.attributes([styles.lane, properties.xstyle], properties.style)}
            >
                {properties.children}
            </div>
        </BoardLaneContext>
    );
}

/** Render the header of the nearest lane, which stays at the board's start as it scrolls. */
export function BoardLaneHeader(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="board-lane-header"
            {...rest}
            {...style.attributes([text.sm, styles.laneHeader, properties.xstyle], properties.style)}
        />
    );
}

/** Render the button that collapses the nearest lane to its header, or expands it. */
export function BoardLaneCollapse(
    properties: SortableElementProperties<JSX.ButtonHTMLAttributes<HTMLButtonElement>>,
): JSX.Element {
    // toggle the nearest lane, labelled by its name
    const control = useBoard();
    const lane = useBoardLane();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");
    const isCollapsed = () => control.isLaneCollapsed(lane.key);
    const label = () => (isCollapsed() ? t`Expand ${lane.label()}` : t`Collapse ${lane.label()}`);

    return (
        <button
            type="button"
            data-slot="board-lane-collapse"
            aria-expanded={isCollapsed() ? "false" : "true"}
            aria-label={locale.render(label())}
            {...mergeProperties({ onClick: () => control.toggleLane(lane.key) }, rest)}
            {...style.attributes([state.hover, styles.toggle, properties.xstyle], properties.style)}
        >
            {properties.children ?? <Icon icon={isCollapsed() ? caretRight : caretDown} />}
        </button>
    );
}

/** Render the row of the nearest lane's cells, hidden while the lane is collapsed. */
export function BoardLaneRow(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // hide the cells while the nearest lane is collapsed
    const control = useBoard();
    const lane = useBoardLane();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="board-lane-row"
            {...rest}
            {...style.attributes(
                [
                    styles.laneCells,
                    control.isLaneCollapsed(lane.key) && styles.hidden,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render a lane's cell under a column, around its cards and composer. */
export function BoardLaneCell(properties: BoardLaneCellProperties): JSX.Element {
    // read the cell under the column in the nearest lane, gone while the column is hidden
    const control = useBoard();
    const lane = useBoardLane();
    const cell: BoardCell = { column: untrack(() => properties.column), lane: lane.key };
    const rest = omit(properties, "column", "xstyle", "style", "children");

    return (
        <BoardCellContext value={cell}>
            <div
                data-slot="board-lane-cell"
                data-hidden={control.isHidden(cell.column) ? "" : undefined}
                {...rest}
                {...style.attributes(
                    [
                        styles.cell,
                        control.isHidden(cell.column) && styles.hidden,
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                {properties.children}
            </div>
        </BoardCellContext>
    );
}

/** Render the cards of a cell, which take cards from every cell of the board. */
export function BoardColumnContent(properties: BoardColumnContentProperties): JSX.Element {
    // read the nearest cell, its column's or its lane's
    const control = useBoard();
    const cell = useBoardCell();

    // take a virtual list's window inside, with its cards' text from the owner
    const host: VirtualHost = {
        add: (window) => {
            const textOf = untrack(() => properties.textOf);
            if (textOf === undefined) {
                const column = cell.column;
                throw new TypeError(
                    cell.lane === undefined
                        ? `board column '${column}' is invalid: its virtual list needs textOf`
                        : `board column '${column}' in lane '${cell.lane}' is invalid: its virtual list needs textOf`,
                );
            }
            control.addWindow(cell, window, textOf);
        },
        persisted: () => control.persistedIn(cell),
    };

    // take cards the board allows into the column, and the types of other views it receives
    const isLaneCollapsed = () => cell.lane !== undefined && control.isLaneCollapsed(cell.lane);
    const accepts = () =>
        isLaneCollapsed()
            ? []
            : [
                  control.cardType,
                  ...(control.properties.onReceive === undefined
                      ? []
                      : (control.properties.dropTypes ?? [])),
              ];
    const isHidden = () => control.isHidden(cell.column);
    const rest = omit(properties, "id", "textOf", "xstyle", "children");

    return (
        <VirtualHostContext value={host}>
            <SortableContainer
                id={control.cellId(cell)}
                label={control.labelOf(cell)}
                accepts={accepts()}
                group={control.laneGroup(cell.lane)}
                isEndOnly={isHidden()}
                allows={(item) => control.takes(cell, item)}
                target={(element) => control.targetOf(cell, element)}
                data-slot="board-column-content"
                data-hidden={isHidden() ? "" : undefined}
                {...rest}
                xstyle={[styles.content, properties.xstyle]}
            >
                {/* The cards, gone in a hidden column, whose row in the hidden list takes drops */}
                <div
                    role="listbox"
                    aria-label={control.labelOf(cell)}
                    aria-multiselectable="true"
                    data-slot="board-cards"
                    {...style.attrs(isHidden() ? styles.hidden : styles.cards)}
                >
                    {properties.children}
                </div>
            </SortableContainer>
        </VirtualHostContext>
    );
}

/** Render a cell's composer: a button that opens a field for a new card, beside the cards. */
export function BoardComposer(properties: BoardComposerProperties): JSX.Element {
    // open the composer by the button or c, one composer at a time
    const control = useBoard();
    const locale = useLocale();
    const cell = useBoardCell();
    const position = untrack(() => properties.position ?? "end");
    const id = control.composerId(cell, position);
    if (control.properties.onCreate === undefined) {
        throw new TypeError("a board composer needs the board's onCreate");
    }
    control.addComposer(cell, position);
    const isOpen = () => control.composing()?.composer === id;
    const rest = omit(properties, "position", "xstyle", "style", "children");
    let button: HTMLButtonElement | undefined;

    // write a card on Enter, a blank field writing none, and stay open for the next
    const submit = (field: HTMLTextAreaElement) => {
        const title = field.value.trim();
        if (title !== "") {
            control.create(cell, title, position);
        }
        field.value = "";
    };

    // write a card on Enter, and close on Escape
    const onKeyDown = (event: KeyboardEvent & { readonly currentTarget: HTMLTextAreaElement }) => {
        // write the card, Enter during an input method's composition left to the method
        if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
            event.preventDefault();
            submit(event.currentTarget);
        }
        // close the composer, and give the focus back to what opened it
        else if (event.key === "Escape") {
            event.preventDefault();
            const { opener } = present(control.composing(), `the open composer '${id}'`);
            control.setComposing(undefined);

            // focus the card that opened it, else the nearest card left
            if (opener.kind === "card") {
                control.refocusOpener(opener.key, opener.nearest);
            }
            // else focus the button once the close renders
            else {
                queueMicrotask(() => {
                    flush();
                    opener.button().focus();
                });
            }
        }
    };

    return (
        <Show
            when={!control.isHidden(cell.column) && isOpen()}
            fallback={
                <Show when={!control.isHidden(cell.column) && position === "end"}>
                    <button
                        type="button"
                        data-slot="board-composer-open"
                        {...mergeProperties(
                            {
                                ref: (created: HTMLButtonElement) => (button = created),
                                onClick: () => {
                                    control.setComposing({
                                        composer: id,
                                        opener: {
                                            kind: "button",
                                            button: () => present(button, "the new card button"),
                                        },
                                    });
                                },
                            },
                            rest,
                        )}
                        {...style.attributes(
                            [text.sm, state.hover, styles.add, properties.xstyle],
                            properties.style,
                        )}
                    >
                        {properties.children ?? (
                            <>
                                <Icon icon={plus} />
                                {locale.render(t`New`)}
                            </>
                        )}
                    </button>
                </Show>
            }
        >
            <textarea
                data-slot="board-composer"
                aria-label={locale.render(t`New card in ${control.labelOf(cell)}`)}
                placeholder={locale.render(t`Type a title…`)}
                rows={1}
                ref={(created: HTMLTextAreaElement) => queueMicrotask(() => created.focus())}
                onKeyDown={onKeyDown}
                onBlur={() => (isOpen() ? control.setComposing(undefined) : undefined)}
                {...style.attrs(text.sm, styles.composer)}
            />
        </Show>
    );
}

/** Render a card of the nearest cell: opened, selected, focused, lifted and given a menu. */
export function BoardItem(properties: BoardItemProperties): JSX.Element {
    // add the card to the board's focus and selection
    const control = useBoard();
    const cell = useBoardCell();
    const locale = useLocale();
    const isCommand = useCommandPlatform();
    const key = untrack(() => properties.id);
    const rest = omit(properties, "activator", "disabled", "xstyle");
    let element: HTMLElement | undefined;
    control.addCard({
        key,
        cell,
        cellId: control.cellId(cell),
        text: () => {
            // read the label
            if (properties.label !== undefined) {
                return properties.label;
            }
            // refuse a card without a label before it renders its text
            else if (element === undefined) {
                throw new TypeError(`board card '${key}' is invalid: it has not rendered`);
            }
            // else read the rendered card's text
            else {
                return element.textContent.trim();
            }
        },
        element: () => element,
    });

    // open or select the card on a press, and run its keys while it has the focus
    const press = new Press({
        onPress: (event) => control.press(key, event),
        keys: ["Enter"],
        shouldCancelOnMove: true,
    });
    const isSelected = (): boolean => control.selection.isSelected(key);
    const card = {
        role: "option" as const,
        get tabindex() {
            return control.focus.isActive(key) ? 0 : -1;
        },
        get "aria-selected"() {
            return isSelected() ? "true" : "false";
        },
        get "aria-posinset"() {
            const place = control.placeOf(key);

            return place === undefined ? undefined : place.index + 1;
        },
        get "aria-setsize"() {
            return control.placeOf(key)?.size;
        },
        get "data-selected"() {
            return isSelected() ? "" : undefined;
        },
        onFocus: () => control.focus.focusIn(key),
        onKeyDown: (event: KeyboardEvent) => {
            // run the card's keys, leaving those a drag or a field inside the card took
            if (!event.defaultPrevented && !Action.isTyping(event)) {
                Action.handle(control.cardActions(key, locale.direction), event, isCommand());
            }
        },
        onContextMenu: (event: MouseEvent) => {
            // open the board's card menu where the board has one, else leave the browser's
            if (
                element !== undefined &&
                control.openMenu(key, element, event.clientX, event.clientY)
            ) {
                event.preventDefault();
            }
        },
        ref: (created: HTMLDivElement) => (element = created),
    };

    return (
        <SortableItem
            type={control.cardType}
            disabled={properties.disabled === true || control.properties.onItemMove === undefined}
            activator={properties.activator ?? "item"}
            data-slot="board-item"
            {...mergeProperties(mergeProperties(card, press.attributes()), rest)}
            xstyle={[
                text.sm,
                state.focusInset,
                styles.item,
                (properties.activator ?? "item") === "item" && styles.lifting,
                isSelected() && state.selected,
                isSelected() && styles.selected,
                properties.xstyle,
            ]}
        />
    );
}

/** Render the button that lifts the nearest card, for cards with controls inside them. */
export function BoardItemHandle(properties: SortableHandleProperties): JSX.Element {
    return <SortableHandle data-slot="board-item-handle" {...properties} />;
}

/** Render the card menu a right click opens: open, move to a column, or delete the cards. */
export function BoardItemMenu(): JSX.Element {
    // let right clicks open the menu, listing the actions for the card it opened on
    const control = useBoard();
    control.addMenu();
    const actions = () => {
        const key = control.menuKey();

        return key === undefined || !control.collection.has(key) ? [] : control.menuActions(key);
    };

    return (
        <ContextMenuContent data-slot="board-item-menu">
            <For each={actions()} keyed={(action) => action.id}>
                {(action) => <MenuAction action={action()} />}
            </For>
        </ContextMenuContent>
    );
}

/** Read the nearest board, refusing a part outside one. */
export function useBoard(): BoardControl {
    const control = useContext(BoardContext);
    if (control === null) {
        throw new TypeError("a board part needs a board around it");
    }

    return control;
}

/** Read the nearest column, refusing a part outside one. */
export function useBoardColumn(): BoardColumnTrack {
    const column = useContext(BoardColumnContext);
    if (column === null) {
        throw new TypeError("a board part needs a board column around it");
    }

    return column;
}

/** Read the nearest lane, refusing a part outside one. */
export function useBoardLane(): BoardTrack {
    const lane = useContext(BoardLaneContext);
    if (lane === null) {
        throw new TypeError("a board part needs a board lane around it");
    }

    return lane;
}

/** Read the nearest cell, refusing a part outside one. */
export function useBoardCell(): BoardCell {
    const cell = useContext(BoardCellContext);
    if (cell === null) {
        throw new TypeError("a board part needs a column or a lane's cell around it");
    }

    return cell;
}

/** Read the next key along a track past skipped ones, none past the track's end. */
function nextShown(
    keys: readonly string[],
    from: string,
    step: number,
    isSkipped: (key: string) => boolean,
): string | undefined {
    // walk past skipped keys in the direction of the step
    let index = keys.indexOf(from) + step;
    let key = keys[index];
    while (key !== undefined && isSkipped(key)) {
        index += step;
        key = keys[index];
    }

    return key;
}

/** Read the first card a move lands before that stays, none at the cell's end. */
function nextOutside(
    next: string | undefined,
    moving: ReadonlySet<string>,
    order: readonly string[],
): string | undefined {
    // walk past moving cards to the next card that stays
    if (next === undefined || !moving.has(next)) {
        return next;
    }

    return order.slice(order.indexOf(next) + 1).find((key) => !moving.has(key));
}

/** Find a key's lane, column and row in a grid of cells. */
function placeIn(
    grid: readonly (readonly (readonly string[])[])[],
    key: string,
): { readonly lane: number; readonly column: number; readonly row: number } | undefined {
    for (const [lane, columns] of grid.entries()) {
        for (const [column, keys] of columns.entries()) {
            const row = keys.indexOf(key);
            if (row >= 0) {
                return { lane, column, row };
            }
        }
    }

    return undefined;
}

/** Read the nearest lane's keys in a column with cards, a step up or down. */
function nearestLane(
    grid: readonly (readonly (readonly string[])[])[],
    lane: number,
    column: number,
    step: number,
): readonly string[] | undefined {
    // walk the lanes until one has cards in the column
    for (let index = lane + step; index >= 0 && index < grid.length; index += step) {
        const keys = grid[index]?.[column] ?? [];
        if (keys.length > 0) {
            return keys;
        }
    }

    return undefined;
}
