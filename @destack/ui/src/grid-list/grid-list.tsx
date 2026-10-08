import type { Direction } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, size, space } from "@destack/theme/tokens.stylex";
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
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import type { AutocompleteControl } from "../autocomplete/index.ts";
import { Collection, type CollectionOptions, type CollectionState } from "../collection/index.ts";
import { Focus, GridDelegate, type GridRow } from "../focus/index.ts";
import { createVirtualizer, type Virtualizer } from "../virtualizer/index.ts";

/** The height a row is estimated at before it is measured, in pixels. */
const ROW_HEIGHT = 36;

/** The rows a grid list renders before its viewport is measured. */
const UNMEASURED_ROWS = 24;

/** The grid list of the nearest grid list as its parts read it, null outside one. */
const GridListContext = createContext<GridListParts | null>(null);

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
        width: size[2],
        height: size[2],
        borderRadius: radius[1],
        cursor: "pointer",
    },
    active: {
        backgroundColor: color.muted,
    },
    note: {
        color: color.mutedForeground,
    },
    spacer: (height: string) => ({ height }),
});

/** The items, layout, focus and window of a grid list. */
export class GridListControl<Item> {
    /** The id of the grid element. */
    readonly id: string;
    /** The items by section. */
    readonly collection: Collection<Item>;
    /** The rows of the items and the keys that move through them. */
    readonly delegate: GridDelegate<Item>;
    /** The focused item, kept virtual behind the grid's or a search field's `aria-activedescendant`. */
    readonly focus: Focus<string>;
    /** The cells in each row. */
    readonly columns: Accessor<number>;
    /** The options the grid list was created with. */
    readonly #options: GridListOptions<Item>;
    /** The element that scrolls the rows, undefined without a viewport. */
    readonly #viewport: Accessor<HTMLElement | undefined>;
    /** Replace the element that scrolls the rows. */
    readonly #setViewport: (viewport: HTMLElement | undefined) => void;
    /** The rows in or near the viewport, each measured as it renders. */
    readonly #virtualizer: Virtualizer;

    /** Lay the sections of items out in rows of a column count, driven by a search field when one is given. */
    constructor(options: GridListOptions<Item>) {
        // lay the items out and follow the viewport that scrolls them
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.id = createUniqueId();
        this.collection = new Collection(options);
        this.columns = options.columns;
        this.delegate = new GridDelegate(this.collection, options.columns);
        this.#options = options;
        this.#viewport = viewport;
        this.#setViewport = setViewport;
        this.#virtualizer = createVirtualizer({
            count: () => this.delegate.rows().length,
            itemHeight: ROW_HEIGHT,
            scrollElement: viewport,
            initialHeight: UNMEASURED_ROWS * ROW_HEIGHT,
        });

        // focus the first item virtually, bringing each moved-to row into view
        this.focus = new Focus<string>({
            delegate: this.delegate,
            mode: "virtual",
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

    /** Run the action of a key's item, as a click or Enter does. */
    act(key: string): void {
        const item = this.collection.item(key);
        if (item !== undefined) {
            this.#options.onAction(item);
        }
    }

    /** The rows the viewport shows with some to spare, every row without a viewport. */
    window(): GridListWindow {
        const items = this.#virtualizer.items();
        const [first, last] = [items[0], items.at(-1)];

        return this.#viewport() === undefined || first === undefined || last === undefined
            ? { first: 0, end: this.delegate.rows().length }
            : { first: first.index, end: last.index + 1 };
    }

    /** The height in pixels of the rows before and after the window, which spacers stand in for. */
    spacers(): { readonly before: number; readonly after: number } {
        const items = this.#virtualizer.items();
        if (this.#viewport() === undefined) {
            return { before: 0, after: 0 };
        }

        return {
            before: items[0]?.start ?? 0,
            after: this.#virtualizer.height() - (items.at(-1)?.end ?? 0),
        };
    }

    /** Measure a rendered row's element, which holds its row in `data-index`. */
    readonly measure = (element: Element | undefined): void => this.#virtualizer.measure(element);

    /** Follow the element that scrolls the rows once it mounts, until it unmounts. */
    observe(element: () => HTMLElement | undefined): void {
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
            this.#virtualizer.reveal(place.row);
        }
    }
}

/** The items, layout and action of a grid list. */
export interface GridListOptions<Item> extends CollectionOptions<Item> {
    /** The cells in each row. */
    readonly columns: Accessor<number>;
    /** The search field that drives the grid list's focus, none for a grid list the keyboard steers itself. */
    readonly autocomplete?: AutocompleteControl | undefined;
    /** Handle an item being chosen with a click or Enter. */
    readonly onAction: (item: Item) => void;
}

/** The rows of a grid list a viewport shows, the first and the one after the last. */
export interface GridListWindow {
    /** The first row rendered. */
    readonly first: number;
    /** The row after the last one rendered. */
    readonly end: number;
}

/** The properties of an element of a grid list, the native element's attributes included. */
export type GridListElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

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
    /** Whether the cell is the focused one, which Enter chooses. */
    readonly isActive: boolean;
}

/** The properties a grid list renders a cell with, the native element's attributes included. */
export interface GridListCellProperties<Item> extends GridListCellFrameProperties {
    /** The item the cell offers. */
    readonly item: Item;
}

/** The parts a grid list renders its headings, rows and cells with, each the grid list's own by default. */
export interface GridListRendering<Item> {
    /** Render a section's heading. */
    readonly SectionHeader?: (properties: GridListSectionHeaderProperties) => JSX.Element;
    /** Render a row of cells. */
    readonly Row?: (properties: GridListRowProperties) => JSX.Element;
    /** Render a cell. */
    readonly Cell?: (properties: GridListCellProperties<Item>) => JSX.Element;
}

/** The properties of a grid list, the native element's attributes included. */
export interface GridListProperties<Item> extends GridListElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "children">
> {
    /** The grid list state its owner created. */
    readonly control: GridListControl<Item>;
    /** Label an item as assistive technology reads it. */
    readonly label: (item: Item) => string;
    /** The parts the grid list renders with in place of its own. */
    readonly components?: GridListRendering<Item>;
}

/** The attributes a grid list gives a heading row. */
interface GridListHeadingAttributes extends Pick<JSX.HTMLAttributes<HTMLDivElement>, "ref"> {
    /** The section's name. */
    readonly label: string;
    /** The row role. */
    readonly role: "row";
    /** The row's place among every row, from 1. */
    readonly "aria-rowindex": string;
    /** The row's place among every row, from 0, which its measuring reads. */
    readonly "data-index": string;
}

/** What a grid list's parts read of it, whatever its items' type. */
export type GridListParts = Pick<GridListControl<never>, "observe" | "state" | "isEmpty">;

/** Read the grid list of the nearest grid list, refusing parts outside one. */
export function useGridList(): GridListParts {
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

/** Render a grid of items by section, which arrow keys, Home and End move through and Enter or a click choose, only the rows in view rendered. */
export function GridList<Item>(properties: GridListProperties<Item>): JSX.Element {
    // read the grid list and the parts to render with
    const control = properties.control;
    const locale = useLocale();
    const rest = omit(properties, "control", "label", "components", "xstyle", "style");

    return (
        <div
            id={control.id}
            role="grid"
            tabindex={0}
            aria-rowcount={control.delegate.rows().length}
            aria-colcount={control.columns()}
            aria-activedescendant={control.focus.descendant()}
            data-slot="grid-list"
            {...rest}
            onKeyDown={(event) => steer(event, control, locale.direction)}
            {...style.attributes([styles.list, properties.xstyle], properties.style)}
        >
            <GridListRows {...properties} />
        </div>
    );
}

/** Render the element that scrolls the nearest grid list, which then renders only the rows in view. */
export function GridListViewport(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // measure the viewport once it mounts so the list renders the rows in view
    const control = useGridList();
    const rest = omit(properties, "xstyle", "style");
    let element: HTMLDivElement | undefined;
    control.observe(() => element);

    return (
        <div
            data-slot="grid-list-viewport"
            {...rest}
            ref={(viewport) => (element = viewport)}
            {...style.attributes([styles.viewport, properties.xstyle], properties.style)}
        />
    );
}

/** Render a section's heading in a grid list. */
export function GridListSectionHeader(properties: GridListSectionHeaderProperties): JSX.Element {
    const rest = omit(properties, "label", "xstyle", "style");

    return (
        <div
            data-slot="grid-list-section-header"
            {...rest}
            {...style.attributes(
                [text.caption, styles.header, properties.xstyle],
                properties.style,
            )}
        >
            <span role="columnheader">{properties.label}</span>
        </div>
    );
}

/** Render a row of a grid list. */
export function GridListRow(properties: GridListRowProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="grid-list-row"
            {...rest}
            {...style.attributes([styles.row, properties.xstyle], properties.style)}
        />
    );
}

/** Render a cell's element of a grid list, marked while focused. */
export function GridListCell(properties: GridListCellFrameProperties): JSX.Element {
    const rest = omit(properties, "isActive", "xstyle", "style");

    return (
        <div
            data-slot="grid-list-cell"
            data-active={properties.isActive ? "" : undefined}
            {...rest}
            {...style.attributes(
                [styles.cell, properties.isActive && styles.active, properties.xstyle],
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={control.state() === "loading"}>
            <div
                role="status"
                data-slot="grid-list-loading"
                {...rest}
                {...style.attributes([styles.note, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Render what the nearest grid list shows once its items load and none are there. */
export function GridListEmpty(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useGridList();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={control.isEmpty()}>
            <div
                data-slot="grid-list-empty"
                {...rest}
                {...style.attributes([styles.note, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Render the rows of a grid list in view between spacers that hold the place of the rest. */
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
        ref: control.measure,
    };

    // render a heading, else a row of cells
    if (row.kind === "heading") {
        const heading: GridListHeadingAttributes = merge(attributes, { label: row.label });

        return createComponent(
            properties.components?.SectionHeader ?? GridListSectionHeader,
            heading,
        );
    } else {
        return createComponent(
            properties.components?.Row ?? GridListRow,
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
        throw new TypeError(`a grid list has no item ${key}`);
    }
    const label = properties.label(item);
    const attributes = {
        id: control.focus.id(key),
        role: "gridcell" as const,
        "aria-colindex": String(column + 1),
        get "aria-selected"() {
            return control.focus.isActive(key) ? ("true" as const) : ("false" as const);
        },
        "aria-label": label,
        get isActive() {
            return control.focus.isActive(key);
        },
        onPointerEnter: () => control.focus.focus(key),
        onClick: () => control.act(key),
    };

    // render the owner's cell, else the item's label in the grid list's own
    const Cell = properties.components?.Cell;
    if (Cell === undefined) {
        return <GridListCell {...attributes}>{label}</GridListCell>;
    }

    return createComponent(Cell, merge(attributes, { item }));
}

/** Move a grid list's focus on its keys, or run the focused item's action on Enter. */
function steer<Item>(
    event: KeyboardEvent,
    control: GridListControl<Item>,
    direction: Direction,
): void {
    const active = control.focus.current();
    const target = control.focus.move(event, direction);
    if (target === undefined && event.key === "Enter" && active !== undefined) {
        event.preventDefault();
        control.act(active);
    }
}
