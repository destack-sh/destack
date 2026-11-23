import { state } from "@destack/theme/state";
import type { Direction } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, size, space, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createComponent,
    createContext,
    createSignal,
    createUniqueId,
    For,
    type JSX,
    merge,
    omit,
    onSettled,
    untrack,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { Hover, type InteractionEvent, Press } from "../interaction/index.ts";
import type { AutocompleteControl } from "../autocomplete/index.ts";
import { Collection, type CollectionOptions, type CollectionState } from "../collection/index.ts";
import { Focus, GridDelegate, type GridRow } from "../focus/index.ts";
import { Selection, type SelectionProperties } from "../selection/index.ts";
import { Virtualizer } from "../virtualizer/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";

/** The height a row is estimated at before it is measured, in pixels. */
const ROW_HEIGHT = 36;

/** The rows a grid list renders before its viewport is measured. */
const UNMEASURED_ROWS = 24;

/** The grid list of the nearest grid list as its parts read it, null outside one. */
const GridListContext = createContext<Pick<
    GridListControl<never>,
    "observe" | "state" | "isEmpty" | "isFitted"
> | null>(null);

/** The styles of a grid list's parts. */
const styles = style.create({
    viewport: {
        position: "relative",
        maxHeight: `calc(8 * ${size[2]})`,
        overflowY: "auto",
        overscrollBehavior: "contain",
    },
    list: {
        outlineStyle: "none",
    },
    row: {
        display: "flex",
        height: size[2],
    },
    header: {
        display: "flex",
        alignItems: "flex-end",
        height: size[2],
        paddingInline: space[1],
        color: color.mutedForeground,
    },
    cell: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        borderRadius: radius[1],
        cursor: "pointer",
    },
    square: {
        width: size[2],
        height: size[2],
    },
    fitted: {
        minWidth: 0,
    },
    selected: {
        borderColor: swatch.accentMark,
    },
    note: {
        color: color.mutedForeground,
    },
    spacer: (height: string) => ({ height }),
    fit: (columns: number, gap: number) => ({
        display: "grid",
        gridTemplateColumns: `repeat(${String(columns)}, minmax(0, 1fr))`,
        columnGap: `${String(gap)}px`,
        paddingBlockEnd: `${String(gap)}px`,
        height: "auto",
    }),
});

/** The items, layout, focus and window of a grid list. */
export class GridListControl<Item> {
    /** The id of the grid element. */
    readonly id: string;
    /** The items by section. */
    readonly collection: Collection<Item>;
    /** The rows of the items and the keys that move through them. */
    readonly delegate: GridDelegate<Item>;
    /** The focused item. */
    readonly focus: Focus<string>;
    /** The selected items, undefined on a grid list that selects nothing. */
    readonly selection: Selection | undefined;
    /** The cells in each row. */
    readonly columns: Accessor<number>;
    /** The room between cells and between rows, in pixels. */
    readonly gap: number;
    /** Whether the cells share a row's width, as on a grid list without a set column count. */
    readonly isFitted: boolean;
    /** The options the grid list was created with. */
    readonly #options: GridListOptions<Item>;
    /** The element that scrolls the rows, undefined without a viewport. */
    readonly #viewport: Accessor<HTMLElement | undefined>;
    /** Replace the element that scrolls the rows. */
    readonly #setViewport: (viewport: HTMLElement | undefined) => void;
    /** Whether a viewport will scroll the rows, so the first render shows a few rows alone. */
    #isObserved = false;
    /** Replace the grid element, which a grid list without a viewport measures. */
    readonly #setElement: (element: HTMLElement | undefined) => void;
    /** The rows in or near the viewport, each measured as it renders. */
    readonly #virtualizer: Virtualizer;

    /** Lay the sections of items out in rows of a column count. */
    constructor(options: GridListOptions<Item>) {
        // lay the items out and follow the viewport that scrolls them
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.id = createUniqueId();
        this.collection = new Collection(options);
        this.#options = options;
        this.#viewport = viewport;
        this.#setViewport = setViewport;
        this.#setElement = setElement;

        // take the given columns, else fit as many cells of the least width as fit the width
        const measured = createElementSize(() => viewport() ?? element());
        const gap = options.gap ?? 0;
        const least = options.minItemWidth;
        this.gap = gap;
        this.isFitted = options.columns === undefined;
        this.columns =
            options.columns ??
            (() =>
                least === undefined
                    ? 1
                    : Math.min(
                          options.maxColumns ?? Number.POSITIVE_INFINITY,
                          Math.max(1, Math.floor(((measured.width ?? 0) + gap) / (least + gap))),
                      ));
        this.delegate = new GridDelegate(this.collection, this.columns);
        const rowHeight = options.rowHeight ?? ROW_HEIGHT;
        const estimate = (): number => rowHeight;
        this.#virtualizer = new Virtualizer({
            count: () => this.delegate.rows().length,
            estimateSize: () => estimate,
            scrollElement: viewport,
            initialRect: { width: 0, height: UNMEASURED_ROWS * rowHeight },
        });

        // select nothing, one item or several
        this.selection =
            options.selection === undefined ? undefined : new Selection(options.selection);

        // focus the items themselves, or virtually under a search field, bringing each row into view
        this.focus = new Focus<string>({
            delegate: this.delegate,
            mode: options.autocomplete === undefined ? "roving" : "virtual",
            reveal: (key) => this.#reveal(key),
        });
        options.autocomplete?.connect({
            id: this.id,
            focus: this.focus,
            isVertical: false,
            act: (key) => this.act(key),
        });
    }

    /** Whether the items are still loading, loaded, or failed to load. */
    state(): CollectionState {
        return this.collection.state();
    }

    /** Report whether the items loaded and none are there. */
    isEmpty(): boolean {
        return this.collection.state() === "loaded" && this.collection.size() === 0;
    }

    /** The focused item, undefined while the grid list has none. */
    active(): Item | undefined {
        const key = this.focus.active();

        return key === undefined ? undefined : this.collection.item(key);
    }

    /** Run the action of a key's item, as a press or Enter does. */
    act(key: string): void {
        const item = this.collection.item(key);
        if (item !== undefined && !this.collection.isDisabled(key)) {
            this.#options.onAction?.(item);
        }
    }

    /** Select or open a pressed item: open on a plain press, select with a modifier. */
    press(key: string, gesture: InteractionEvent): void {
        // select with command, control or shift, or by any press on a grid list that opens nothing
        const hasAction = this.#options.onAction !== undefined;
        if (this.selection?.isSelecting(gesture, hasAction) === true) {
            this.selection.select(key, gesture, this.collection);
        }
        // open the item
        else {
            this.act(key);
        }
    }

    /** Take a key: move the focus, open with Enter, toggle with Space, select all, or clear. */
    keyDown(event: KeyboardEvent, direction: Direction): void {
        // leave keys an item already took
        const focused = this.focus.current();
        const selection = this.focus.mode === "roving" ? this.selection : undefined;
        if (event.defaultPrevented) {
            return;
        }
        // toggle the focused item's selection with Space
        else if (selection !== undefined && event.key === " " && focused !== undefined) {
            event.preventDefault();
            selection.toggle(focused);
        }
        // select every item with command or control and A, or clear the selection with Escape
        else if (
            selection?.isMultiple() === true &&
            event.key === "a" &&
            (event.metaKey || event.ctrlKey)
        ) {
            event.preventDefault();
            selection.selectAll(this.collection);
        } else if (
            selection !== undefined &&
            event.key === "Escape" &&
            selection.values().length > 0
        ) {
            event.preventDefault();
            selection.clear();
        }
        // move the focus through the rows, extending the selection with shift, or open with Enter
        else {
            const target = this.focus.move(event, direction);
            if (
                target !== undefined &&
                focused !== undefined &&
                event.shiftKey &&
                selection?.isMultiple() === true
            ) {
                selection.extendFrom(focused, target, this.collection);
            } else if (target === undefined && event.key === "Enter" && focused !== undefined) {
                event.preventDefault();
                this.act(focused);
            }
        }
    }

    /** Follow the grid element, which a grid list without a viewport measures. */
    attach(element: HTMLElement | undefined): void {
        this.#setElement(element);
    }

    /** The rows the viewport shows with some to spare, every row without a viewport. */
    window(): GridListWindow {
        // read the first and last rows the virtualizer has in view
        const items = this.#virtualizer.items();
        const [first, last] = [items[0], items.at(-1)];

        // show the rows in view, a first few until a viewport that will come is measured
        const count = this.delegate.rows().length;
        if (this.#viewport() !== undefined && first !== undefined && last !== undefined) {
            return { first: first.index, end: last.index + 1 };
        }

        return { first: 0, end: this.#isObserved ? Math.min(count, UNMEASURED_ROWS) : count };
    }

    /** The height in pixels of the rows before and after the window, which spacers stand in for. */
    spacers(): { readonly before: number; readonly after: number } {
        return this.#viewport() === undefined
            ? { before: 0, after: 0 }
            : this.#virtualizer.spacers();
    }

    /** Measure a rendered row's element at its place among every row. */
    measure(element: Element, index: number): void {
        element.setAttribute("data-index", String(index));
        this.#virtualizer.measure(element);
    }

    /** Follow the element that scrolls the rows once it mounts, until it unmounts. */
    observe(element: () => HTMLElement | undefined): void {
        this.#isObserved = true;
        onSettled(() => {
            // take the mounted viewport, letting it go as it unmounts
            const viewport = element();
            if (viewport === undefined) {
                return undefined;
            }
            this.#setViewport(viewport);

            return () => this.#setViewport(undefined);
        });
    }

    /** Scroll a key's row into the viewport. */
    #reveal(key: string): void {
        const place = this.delegate.place(key);
        if (this.#viewport() !== undefined && place !== undefined) {
            this.#virtualizer.scrollToIndex(place.row);
        }
    }
}

/** The items, layout and action of a grid list. */
export interface GridListOptions<Item> extends CollectionOptions<Item> {
    /** The cells in each row, as many as fit by their least width when absent, else one. */
    readonly columns?: Accessor<number>;
    /** The least width of a cell in pixels, which sets how many fit a row. */
    readonly minItemWidth?: number;
    /** The most cells in a row that fits its cells. */
    readonly maxColumns?: number;
    /** The room between cells and between rows, in pixels. */
    readonly gap?: number;
    /** The height a row is estimated at before it is measured, in pixels, 36 by default. */
    readonly rowHeight?: number;
    /** The search field that drives the focus, which the items take themselves without one. */
    readonly autocomplete?: AutocompleteControl | undefined;
    /** The selection, absent on a grid list that selects nothing. */
    readonly selection?: SelectionProperties | undefined;
    /** Handle an item being opened with a press or Enter. */
    readonly onAction?: ((item: Item) => void) | undefined;
}

/** The rows of a grid list a viewport shows, the first and the one after the last. */
export interface GridListWindow {
    /** The first row rendered. */
    readonly first: number;
    /** The row after the last one rendered. */
    readonly end: number;
}

/** The properties of an element of a grid list, the native element's attributes included. */
export type GridListElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** The properties of a section heading of a grid list, the native element's attributes included. */
export interface GridListSectionHeaderProperties extends GridListElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The section's name. */
    readonly label: string;
}

/** The properties of a row of a grid list, the native element's attributes included. */
export type GridListRowProperties = GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>;

/** The properties of a cell's element of a grid list, the native element's attributes included. */
export interface GridListCellFrameProperties extends GridListElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Whether the cell is the focused one, which Enter opens. */
    readonly isActive: boolean;
    /** Whether the cell's item is selected. */
    readonly isSelected: boolean;
}

/** The properties a grid list renders a cell with, the native element's attributes included. */
export interface GridListCellProperties<Item> extends GridListCellFrameProperties {
    /** The item the cell offers. */
    readonly item: Item;
}

/** The properties of a grid list, the native element's attributes included. */
export interface GridListProperties<Item> extends GridListElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "children">
> {
    /** The grid list state its owner created. */
    readonly control: GridListControl<Item>;
    /** Label an item as assistive technology reads it. */
    readonly label: (item: Item) => string;
    /** Render a section's heading, the grid list's default when absent. */
    readonly sectionHeader?: (properties: GridListSectionHeaderProperties) => JSX.Element;
    /** Render a cell, the item's label in the grid list's default cell when absent. */
    readonly children?: (properties: GridListCellProperties<Item>) => JSX.Element;
}

/** The attributes a grid list gives a heading row. */
interface GridListHeadingAttributes extends Pick<JSX.HTMLAttributes<HTMLDivElement>, "ref"> {
    /** The heading's data attributes, such as its index. */
    readonly [data: `data-${string}`]: string | number | undefined;
    /** The section's name. */
    readonly label: string;
    /** The row role. */
    readonly role: "row";
    /** The row's place among every row, from 1. */
    readonly "aria-rowindex": string;
    /** The row's place among every row, from 0, which its measuring reads. */
    readonly "data-index": string;
}

/** Read the grid list of the nearest grid list, refusing parts outside one. */
export function useGridList(): Pick<
    GridListControl<never>,
    "observe" | "state" | "isEmpty" | "isFitted"
> {
    const control = useContext(GridListContext);
    if (control === null) {
        throw new TypeError("a grid list part needs a grid list around it");
    }

    return control;
}

/** Provide a grid list's state to its parts, such as its viewport and notes. */
export function GridListProvider<Item>(properties: {
    readonly control: GridListControl<Item>;
    readonly children?: JSX.Element;
}): JSX.Element {
    return <GridListContext value={properties.control}>{properties.children}</GridListContext>;
}

/** Render a grid of items by section, and render only the rows in view. */
export function GridList<Item>(properties: GridListProperties<Item>): JSX.Element {
    // read the grid list and the parts to render with
    const control = properties.control;
    const locale = useLocale();
    const isVirtual = control.focus.mode === "virtual";
    const rest = omit(
        properties,
        "control",
        "label",
        "sectionHeader",
        "children",
        "xstyle",
        "style",
    );

    return (
        <div
            id={control.id}
            role="grid"
            tabindex={isVirtual ? 0 : undefined}
            aria-rowcount={control.delegate.rows().length}
            aria-colcount={control.columns()}
            aria-activedescendant={isVirtual ? control.focus.descendant() : undefined}
            aria-multiselectable={control.selection?.isMultiple() === true ? "true" : undefined}
            data-slot="grid-list"
            {...rest}
            ref={(element) => control.attach(element)}
            onKeyDown={(event) => control.keyDown(event, locale.direction)}
            onFocusOut={(event) => control.focus.focusOut(event)}
            {...style.attributes([styles.list, properties.xstyle], properties.style)}
        >
            <GridListContext value={control}>
                <GridListRows {...properties} />
            </GridListContext>
        </div>
    );
}

/** Render the element that scrolls the nearest grid list. */
export function GridListViewport(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // measure the viewport once it mounts so the list renders the rows in view
    const control = useGridList();
    let element: HTMLDivElement | undefined;
    control.observe(() => element);

    return renderPart("div", "grid-list-viewport", properties, styles.viewport, {
        ref: (viewport) => (element = viewport),
    });
}

/** Render a section's heading in a grid list. */
export function GridListSectionHeader(properties: GridListSectionHeaderProperties): JSX.Element {
    const rest = omit(properties, "label", "xstyle", "style");

    return (
        <div
            data-slot="grid-list-section-header"
            {...rest}
            {...style.attributes([text.xs, styles.header, properties.xstyle], properties.style)}
        >
            <span role="columnheader">{properties.label}</span>
        </div>
    );
}

/** Render a row of a grid list. */
export function GridListRow(properties: GridListRowProperties): JSX.Element {
    return renderPart("div", "grid-list-row", properties, styles.row);
}

/** Render a cell's element of a grid list, marked while focused. */
export function GridListCell(properties: GridListCellFrameProperties): JSX.Element {
    const isFitted = useGridList().isFitted;
    const rest = omit(properties, "isActive", "isSelected", "xstyle", "style");

    return (
        <div
            data-slot="grid-list-cell"
            data-highlighted={properties.isActive ? "" : undefined}
            data-selected={properties.isSelected ? "" : undefined}
            data-disabled={properties["aria-disabled"] === "true" ? "" : undefined}
            {...rest}
            {...style.attributes(
                [
                    styles.cell,
                    isFitted ? [state.focus, styles.fitted] : styles.square,
                    properties.isActive && !isFitted && state.selected,
                    properties.isSelected && state.selected,
                    properties.isSelected && styles.selected,
                    properties["aria-disabled"] === "true" && state.inert,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render what the nearest grid list shows while its items load. */
export function GridListLoading(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useGridList();

    return (
        <Show when={control.state() === "loading"}>
            {renderPart("div", "grid-list-loading", properties, styles.note, { role: "status" })}
        </Show>
    );
}

/** Render what the nearest grid list shows once its items load and none are there. */
export function GridListEmpty(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useGridList();

    return (
        <Show when={control.isEmpty()}>
            {renderPart("div", "grid-list-empty", properties, styles.note)}
        </Show>
    );
}

/** Render the rows of a grid list in view between spacers that stand in for the rest. */
function GridListRows<Item>(properties: GridListProperties<Item>): JSX.Element {
    // render the rows in the window
    const control = properties.control;
    const shown = (): readonly GridRow[] => {
        const { first, end } = control.window();

        return control.delegate.rows().slice(first, end);
    };

    return (
        <>
            <div
                aria-hidden="true"
                {...style.attrs(styles.spacer(`${String(control.spacers().before)}px`))}
            />
            <For each={shown()}>
                {(row, offset) =>
                    renderRow(properties, row, () => control.window().first + offset())
                }
            </For>
            <div
                aria-hidden="true"
                {...style.attrs(styles.spacer(`${String(control.spacers().after)}px`))}
            />
        </>
    );
}

/** Render a row of a grid list at its place among every row, the ones out of view included. */
function renderRow<Item>(
    properties: GridListProperties<Item>,
    row: GridRow,
    place: () => number,
): JSX.Element {
    // number the row from 1 for assistive technology and from 0 for its measuring
    const control = properties.control;
    const attributes = {
        role: "row" as const,
        get "aria-rowindex"() {
            return String(place() + 1);
        },
        get "data-index"() {
            return String(place());
        },
        get xstyle() {
            return control.isFitted ? styles.fit(control.columns(), control.gap) : undefined;
        },
        ref: (element: Element) => control.measure(element, untrack(place)),
    };

    // render a heading, else a row of cells
    if (row.kind === "heading") {
        const heading: GridListHeadingAttributes = merge(attributes, { label: row.label });

        return createComponent(properties.sectionHeader ?? GridListSectionHeader, heading);
    } else {
        return createComponent(
            GridListRow,
            merge(attributes, {
                get children() {
                    return row.keys.map((key, column) => renderCell(properties, key, column));
                },
            }),
        );
    }
}

/** Render a cell of a grid list with its id, place and focus, and its pointer handlers. */
function renderCell<Item>(
    properties: GridListProperties<Item>,
    key: string,
    column: number,
): JSX.Element {
    // read the cell's item, refusing a key the collection lost
    const control = properties.control;
    const item = control.collection.item(key);
    if (item === undefined) {
        throw new TypeError(`a grid list has no item '${key}'`);
    }
    const label = properties.label(item);
    const isVirtual = control.focus.mode === "virtual";
    const isSelected = () =>
        control.selection === undefined
            ? isVirtual && control.focus.isActive(key)
            : control.selection.isSelected(key);
    const shared = {
        id: control.focus.id(key),
        role: "gridcell" as const,
        "aria-colindex": String(column + 1),
        get "aria-selected"() {
            // say nothing about selection on a grid list that selects nothing
            if (control.selection === undefined && !isVirtual) {
                return undefined;
            }

            return isSelected() ? ("true" as const) : ("false" as const);
        },
        "aria-label": label,
        get "aria-disabled"() {
            return control.collection.isDisabled(key) ? ("true" as const) : undefined;
        },
        get isActive() {
            return control.focus.isActive(key);
        },
        get isSelected() {
            return control.selection?.isSelected(key) === true;
        },
    };

    // focus a cell under a search field as the pointer moves onto it, and open it on a click
    if (isVirtual) {
        const hover = new Hover({ onHoverStart: () => control.focus.focus(key) }).attributes();
        const pointed = {
            get "data-hovered"() {
                return hover["data-hovered"];
            },
            onPointerEnter: hover.onPointerEnter,
            onPointerLeave: hover.onPointerLeave,
            onClick: () => control.act(key),
        };

        return renderItem(properties, merge(shared, pointed), item, label);
    }

    // give a cell without a search field a tab stop, and open or select it on a press
    let element: HTMLElement | undefined;
    control.focus.bind(
        () => key,
        () => element,
    );
    const press = new Press({
        onPress: (event) => control.press(key, event),
        keys: ["Enter"],
        isDisabled: () => control.collection.isDisabled(key),
    });
    const focused = {
        get tabindex() {
            return control.focus.isActive(key) ? 0 : -1;
        },
        onFocus: () => control.focus.focusIn(key),
        ref: (created: HTMLElement) => (element = created),
    };

    return renderItem(
        properties,
        merge(shared, mergeProperties(focused, press.attributes())),
        item,
        label,
    );
}

/** Render a cell with the caller's function, else the item's label in the default cell. */
function renderItem<Item>(
    properties: GridListProperties<Item>,
    attributes: GridListCellFrameProperties,
    item: Item,
    label: string,
): JSX.Element {
    const cell = properties.children;
    if (cell === undefined) {
        return <GridListCell {...attributes}>{label}</GridListCell>;
    }

    return createComponent(cell, merge(attributes, { item }));
}
