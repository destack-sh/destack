import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight, width, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type Direction, Shortcut } from "@destack/locale";
import {
    type Accessor,
    createContext,
    type JSX,
    omit,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Collection, CollectionBuilder } from "../collection/index.ts";
import { ColumnsDelegate, Focus } from "../focus/index.ts";
import { Press, type InteractionEvent } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { type MultipleSelection, Selection } from "../selection/index.ts";
import {
    Sortable,
    SortableContainer,
    type SortableElementProperties,
    SortableHandle,
    type SortableHandleProperties,
    SortableItem,
    type SortableMove,
} from "../sortable/index.ts";

/** The container id of a board's row of columns, which a column's move reports. */
const COLUMNS = "board-columns";

/** The item type of a board's columns. */
const COLUMN_TYPE = "column";

/** The item type of a board's cards. */
const CARD_TYPE = "card";

/** The styles of a board and its parts. */
const styles = style.create({
    board: {
        display: "flex",
        minHeight: 0,
        overflowX: "auto",
        overscrollBehaviorX: "contain",
        scrollSnapType: "x proximity",
        paddingBlockEnd: space[2],
    },
    columns: {
        display: "flex",
        flexGrow: 1,
        alignItems: "stretch",
        gap: space[3],
    },
    column: {
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
        width: width.column,
        maxHeight: "100%",
        borderRadius: radius[4],
        backgroundColor: color.muted,
        scrollSnapAlign: "start",
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[3],
        fontWeight: weight.medium,
    },
    content: {
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        gap: space[2],
        minHeight: space[8],
        overflowY: "auto",
        paddingInline: space[2],
        paddingBlockEnd: space[2],
    },
    item: {
        paddingBlock: space[2],
        paddingInline: space[3],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.card,
    },
    lifting: {
        cursor: { default: "grab", ":active": "grabbing" },
    },
    selected: {
        borderColor: swatch.accentMark,
    },
});

/** A card of a board as its control collects it. */
interface BoardCard {
    /** The card's id. */
    readonly key: string;
    /** The id of the column it stands in. */
    readonly column: string;
    /** The card's text, which typeahead and announcements read. */
    readonly text: Accessor<string>;
    /** Whether the card stays where it is. */
    readonly isDisabled: Accessor<boolean>;
    /** The card's element, which orders it among the others. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** A column of a board as its control collects it. */
interface BoardColumnEntry {
    /** The column's id. */
    readonly key: string;
    /** The column's name. */
    readonly label: Accessor<string>;
    /** The column's element, which orders it among the others. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** The id and name of a column, which its content and cards read. */
interface BoardColumnName {
    /** The column's id. */
    readonly id: string;
    /** The column's name. */
    readonly label: string;
}

/** The properties of a board, the native element's attributes included. */
export interface BoardProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Handle a person moving a card into a column between two cards. */
    readonly onItemMove?: (move: SortableMove) => void;
    /** Handle a person moving a column between two columns, absent to fix the columns in place. */
    readonly onColumnMove?: (move: SortableMove) => void;
    /** Report whether a card may move into a column, every column by default. */
    readonly isAllowed?: (item: string, column: string) => boolean;
    /** Handle a person opening a card with a plain press or Enter. */
    readonly onAction?: (key: string) => void;
    /** The selected cards, controlled or the board's, replacing on a plain press by default. */
    readonly selection?: MultipleSelection;
    /** The name announcements call the row of columns by. */
    readonly label?: string;
}

/** The properties of a board's column, the native element's attributes included. */
export interface BoardColumnProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The column's id, which a card's move reports as its container. */
    readonly id: string;
    /** The column's name, which announcements call it by. */
    readonly label: string;
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
    /** Whether the whole card lifts, true by default; false lifts only its handle. */
    readonly asHandle?: boolean;
}

/** The cards of a board by column, the one focused card, and the selected cards. */
export class BoardControl {
    /** The cards in document order. */
    readonly cards: CollectionBuilder<BoardCard>;
    /** The columns in document order. */
    readonly columns: CollectionBuilder<BoardColumnEntry>;
    /** The cards by column, one section per column. */
    readonly collection: Collection<BoardCard>;
    /** The focused card, the whole board's one tab stop. */
    readonly focus: Focus<string>;
    /** The selected cards, which bulk actions apply to. */
    readonly selection: Selection;
    /** The board's properties, read for its action. */
    readonly properties: BoardProperties;

    /** Collect a board's cards and columns as they mount. */
    constructor(properties: BoardProperties) {
        // collect cards and columns in document order and group the cards by column
        this.properties = properties;
        this.cards = new CollectionBuilder((card) => card.element());
        this.columns = new CollectionBuilder((column) => column.element());
        this.collection = new Collection({
            sections: () =>
                this.columns.items().map((column) => ({
                    key: column.key,
                    label: column.label(),
                    items: this.cards.items().filter((card) => card.column === column.key),
                })),
            key: (card) => card.key,
            text: (card) => card.text(),
            isDisabled: (card) => card.isDisabled(),
        });

        // move one focused card across the columns, and select several as Finder does
        this.focus = new Focus({ delegate: new ColumnsDelegate(this.collection), mode: "roving" });
        this.selection = new Selection(
            untrack(() => properties.selection) ?? { multiple: true, behavior: "replace" },
        );
    }

    /** Add a card until it unmounts. */
    addCard(card: BoardCard): void {
        this.cards.add(card);
        this.focus.bind(() => card.key, card.element);
    }

    /** Select or open a pressed card: open on a plain press, select with a modifier. */
    press(key: string, gesture: InteractionEvent): void {
        // select with command, control or shift, or by any press on a board that opens nothing
        if (this.selection.selects(gesture, this.properties.onAction !== undefined)) {
            this.selection.select(key, gesture, this.collection);
        }
        // open the card
        else {
            this.properties.onAction?.(key);
        }
    }

    /** Take a key on the board: move the focus, toggle the selection, select all or clear it. */
    keyDown(event: KeyboardEvent, direction: Direction): void {
        // leave keys a card or a drag already took, and keys typed into a card's fields
        const focused = this.focus.current();
        const isTyping =
            event.target instanceof Element &&
            event.target.closest("input, textarea, select, [contenteditable]") !== null;
        if (event.defaultPrevented || isTyping) {
            return;
        }
        // toggle the focused card's selection with x, as Linear does
        else if (event.key === "x" && focused !== undefined) {
            event.preventDefault();
            this.selection.toggle(focused);
        }
        // select every card with the platform's mod and A, or clear the selection with Escape
        else if (Shortcut.matches("mod+a", event, Shortcut.isCommand(navigator.userAgent))) {
            event.preventDefault();
            this.selection.selectAll(this.collection);
        } else if (event.key === "Escape" && this.selection.values().length > 0) {
            event.preventDefault();
            this.selection.clear();
        }
        // move the focus across the columns, extending the selection with shift
        else {
            const target = this.focus.move(event, direction);
            if (target !== undefined && event.shiftKey) {
                this.selection.extend(target, this.collection);
            }
        }
    }
}

/** The nearest board, null outside one. */
const BoardContext = createContext<BoardControl | null>(null);

/** The id and name of the nearest column, which its content and cards read. */
const BoardColumnContext = createContext<BoardColumnName | null>(null);

/** Render a board of columns of cards, after Dice UI's Kanban, announcing each move. */
export function Board(properties: BoardProperties): JSX.Element {
    // report a move in the row of columns as a column's move, any other as a card's
    const onMove = (move: SortableMove) =>
        move.container === COLUMNS
            ? properties.onColumnMove?.(move)
            : properties.onItemMove?.(move);
    const rest = omit(
        properties,
        "onItemMove",
        "onColumnMove",
        "onAction",
        "isAllowed",
        "selection",
        "label",
        "children",
        "xstyle",
    );

    // move one focus across the cards and select them, after the keys a drag takes
    const control = new BoardControl(properties);
    const locale = useLocale();
    const keys = {
        onKeyDown: (event: KeyboardEvent) => control.keyDown(event, locale.direction),
        onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
            control.focus.focusOut(event),
    };

    return (
        <BoardContext value={control}>
            <Sortable
                data-slot="board"
                onMove={onMove}
                {...mergeProperties(keys, rest)}
                xstyle={[styles.board, properties.xstyle]}
            >
                <SortableContainer
                    id={COLUMNS}
                    data-slot="board-columns"
                    orientation="horizontal"
                    accepts={[COLUMN_TYPE]}
                    {...(properties.label === undefined ? {} : { label: properties.label })}
                    xstyle={styles.columns}
                >
                    {properties.children}
                </SortableContainer>
            </Sortable>
        </BoardContext>
    );
}

/** Render a column of the nearest board, which itself moves along the row. */
export function BoardColumn(properties: BoardColumnProperties): JSX.Element {
    // add the column to the board's order of columns
    const control = useBoard();
    const rest = omit(properties, "id", "label", "xstyle", "children");
    let element: HTMLElement | undefined;
    control.columns.add({
        key: untrack(() => properties.id),
        label: () => properties.label,
        element: () => element,
    });
    const column = {
        get id() {
            return properties.id;
        },
        get label() {
            return properties.label;
        },
    };

    return (
        <BoardColumnContext value={column}>
            <SortableItem
                id={properties.id}
                type={COLUMN_TYPE}
                label={properties.label}
                data-slot="board-column"
                {...mergeProperties(
                    { ref: (created: HTMLDivElement) => (element = created) },
                    rest,
                )}
                xstyle={[styles.column, properties.xstyle]}
            >
                {properties.children}
            </SortableItem>
        </BoardColumnContext>
    );
}

/** Render the header of the nearest column: its title, count and handle. */
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

/** Render the button that moves the nearest column along the row. */
export function BoardColumnHandle(properties: SortableHandleProperties): JSX.Element {
    return <SortableHandle data-slot="board-column-handle" {...properties} />;
}

/** Render the cards of the nearest column, which take cards from every column of the board. */
export function BoardColumnContent(
    properties: SortableElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // take cards of the column the board allows into it
    const control = useBoard();
    const column = useBoardColumn();
    const rest = omit(properties, "id", "xstyle");

    return (
        <SortableContainer
            id={column.id}
            label={column.label}
            role="listbox"
            aria-label={column.label}
            aria-multiselectable="true"
            accepts={[CARD_TYPE]}
            allows={(item) => control.properties.isAllowed?.(item, column.id) !== false}
            data-slot="board-column-content"
            {...rest}
            xstyle={[styles.content, properties.xstyle]}
        />
    );
}

/** Render a card of the nearest column: opened, selected, focused and lifted. */
export function BoardItem(properties: BoardItemProperties): JSX.Element {
    // add the card to the board's focus and selection
    const control = useBoard();
    const column = useBoardColumn();
    const rest = omit(properties, "asHandle", "xstyle");
    let element: HTMLElement | undefined;
    control.addCard({
        key: properties.id,
        column: column.id,
        text: () => {
            // read the label, else the rendered card's text
            if (properties.label !== undefined) {
                return properties.label;
            } else if (element === undefined) {
                throw new TypeError(
                    `board card '${properties.id}' is invalid: it has not rendered`,
                );
            }

            return element.textContent.trim();
        },
        isDisabled: () => properties.disabled === true,
        element: () => element,
    });

    // open or select the card on a press, and mark it while selected
    const press = new Press({
        onPress: (event) => control.press(properties.id, event),
        keys: ["Enter"],
    });
    const isSelected = (): boolean => control.selection.isSelected(properties.id);
    const card = {
        role: "option" as const,
        get tabindex() {
            return control.focus.isActive(properties.id) ? 0 : -1;
        },
        get "aria-selected"() {
            return isSelected() ? "true" : "false";
        },
        get "data-selected"() {
            return isSelected() ? "" : undefined;
        },
        onFocus: () => control.focus.focusIn(properties.id),
        ref: (created: HTMLDivElement) => (element = created),
    };

    return (
        <SortableItem
            type={CARD_TYPE}
            asHandle={properties.asHandle ?? true}
            data-slot="board-item"
            {...mergeProperties(mergeProperties(card, press.attributes()), rest)}
            xstyle={[
                text.sm,
                state.focus,
                styles.item,
                (properties.asHandle ?? true) && styles.lifting,
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

/** Read the nearest board, refusing a part outside one. */
export function useBoard(): BoardControl {
    const control = useContext(BoardContext);
    if (control === null) {
        throw new TypeError("a board part needs a board around it");
    }

    return control;
}

/** Read the nearest column, refusing a part outside one. */
function useBoardColumn(): BoardColumnName {
    const column = useContext(BoardColumnContext);
    if (column === null) {
        throw new TypeError("a board part needs a board column around it");
    }

    return column;
}
