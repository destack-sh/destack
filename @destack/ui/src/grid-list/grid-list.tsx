import { state } from "@destack/theme/state";
import type { Direction } from "@destack/locale";
import type { Action, ActionInputs } from "@destack/object";
import { present } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import * as style from "@destack/style";
import { color, radius, shadow, size, space, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    type ActionRegistry,
    createComponent,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    createUniqueId,
    flush,
    For,
    handle,
    isTextField,
    type JSX,
    merge,
    omit,
    onSettled,
    untrack,
    Show,
    useActions,
    useContext,
    useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { Hover, type InteractionEvent, Press } from "../interaction/index.ts";
import { provideItems } from "../action/place.ts";
import {
    clearSelection,
    moveItemsDown,
    moveItemsUp,
    nextItem,
    openItem,
    previousItem,
    selectAll,
    toggleSelection,
} from "../action/action.ts";
import type { AutocompleteControl } from "../autocomplete/index.ts";
import {
    Collection,
    CollectionBuilder,
    type CollectionOptions,
    type CollectionState,
} from "../collection/index.ts";
import { Focus, GridDelegate, type GridLine, type GridRow } from "../focus/index.ts";
import type { RowLayout } from "../gallery/layout.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { Selection, type SelectionProperties } from "../selection/index.ts";
import {
    dropStyle,
    Sortable,
    SortableContainer,
    SortableItem,
    type SortableMove,
    useSortable,
} from "../sortable/index.ts";
import { type VirtualHost, VirtualHostContext } from "../virtual-list/index.ts";
import { type VirtualRange, Virtualizer } from "../virtualizer/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";

/** The height a row is estimated at before it is measured, in pixels. */
const ROW_HEIGHT = 36;

/** The rows a grid list renders before its viewport is measured. */
const UNMEASURED_ROWS = 24;

/** The grid list of the nearest grid list as its parts read it, null outside one. */
const GridListContext = createContext<Pick<
    GridListControl<never>,
    "focus" | "observe" | "state" | "isEmpty" | "isFitted" | "isFocusedOnHover"
> | null>(null);

/** The overlays that keep the focus while they are open. */
const OVERLAY = ":popover-open, dialog[open]";

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
    // raise the copy of a row a drag lifts
    lifted: {
        borderRadius: radius[2],
        backgroundColor: { default: null, ":is([data-slot=drag-preview] > *)": color.popover },
        boxShadow: { default: null, ":is([data-slot=drag-preview] > *)": shadow.overlay },
    },
    pointed: {
        backgroundColor: color.hover,
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
        borderRadius: radius[2],
        backgroundColor: { default: null, ":is([data-menu-open])": color.hover },
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
    // keep a heading over the rows of its section as they scroll under it
    sticky: {
        position: "sticky",
        insetBlockStart: 0,
        zIndex: 1,
    },
    // cover the rows that scroll under a heading that sticks
    covering: {
        backgroundColor: color.background,
    },
    spacer: (height: string) => ({ height }),
    // leave the room of the rows out of view before it, where an element would move among rows
    after: (height: string) => ({ marginBlockStart: height }),
    sized: (columns: string, height: string, gap: number) => ({
        display: "grid",
        gridTemplateColumns: columns,
        gridTemplateRows: height,
        columnGap: `${String(gap)}px`,
        paddingBlockEnd: `${String(gap)}px`,
        height: "auto",
    }),
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
    /** The rows' keys, places and sections, read once per change to the rows. */
    readonly #rows: Accessor<GridListRowIndex>;
    /** The lists a drag moves the rows in, one at most. */
    readonly #hosts = new CollectionBuilder<VirtualHost>();
    /** The grid list's element. */
    #root: HTMLElement | undefined;

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
        const gap = options.layout?.gap ?? options.gap ?? 0;
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
        const { layout } = options;
        const width = (): number => measured.width ?? 0;
        this.delegate = new GridDelegate(
            this.collection,
            this.columns,
            layout === undefined ? undefined : (keys) => this.#break(keys, layout, width()),
            options.hasEmptySections,
        );
        this.#rows = createMemo(() => this.#index());

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

        // render the rows in view, the focused one, and those a list around asks for
        const rowHeight = options.rowHeight ?? ROW_HEIGHT;
        const persisted = createMemo(
            () => {
                // mount the focused item and those a drag holds
                const { places } = this.#rows();
                const focused = this.focus.isFocused() ? this.focus.active() : undefined;
                const keys = [focused, ...(this.#hosts.items()[0]?.persisted() ?? [])];

                return keys.flatMap((key) => (key === undefined ? [] : (places.get(key) ?? [])));
            },
            { equals: (previous, next) => previous.join() === next.join() },
        );

        // change the estimate only when a kept row's size changes
        const sizes = createMemo(
            () => {
                const { keys } = this.#rows();

                return new Map(
                    this.delegate
                        .rows()
                        .flatMap((row, index) =>
                            row.kind === "cells" && row.size !== undefined
                                ? [[keys[index] ?? index, row.size.height] as const]
                                : [],
                        ),
                );
            },
            {
                equals: (previous, next) =>
                    [...next].every(([key, height]) => (previous.get(key) ?? height) === height),
            },
        );
        this.#virtualizer = new Virtualizer({
            count: () => this.delegate.rows().length,
            estimateSize: createMemo(() => {
                // take the height of a row of cells with a size, else the estimate
                sizes();

                return (index: number) => {
                    const row = untrack(() => this.delegate.rows())[index];

                    return row?.kind === "cells" && row.size !== undefined
                        ? row.size.height + gap
                        : rowHeight;
                };
            }),
            itemKey: () => {
                const { keys } = this.#rows();

                return (index) => keys[index] ?? index;
            },
            persisted,
            rangeExtractor: (range) => this.#extract(range),
            scrollPaddingStart: options.hasStickyHeadings === true ? rowHeight : 0,
            scrollElement: viewport,
            initialRect: { width: 0, height: UNMEASURED_ROWS * rowHeight },
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

    /** Take a key: run an action on the items, toggle with Space, open with Enter, or move. */
    keyDown(
        event: KeyboardEvent,
        direction: Direction,
        actions: ActionRegistry,
        isCommand: boolean,
    ): void {
        // leave keys an item or a field in it took, and keys bound to actions on the items
        const focused = this.focus.current();
        const isRoving = this.focus.mode === "roving";
        const selection = isRoving ? this.selection : undefined;
        const isTaken = event.defaultPrevented || (isRoving && isTextField(event.target));
        const isOnItem =
            event.target instanceof Element && event.target === this.#elementOf(focused);
        if (isTaken || (isRoving && actions.press(event, isCommand, this.#root))) {
            return;
        }
        // leave Space to an action something around the grid list binds it to
        else if (isRoving && event.key === " " && actions.press(event, isCommand)) {
            return;
        }
        // toggle the focused item's selection with Space
        else if (
            selection !== undefined &&
            event.key === " " &&
            isOnItem &&
            focused !== undefined
        ) {
            event.preventDefault();
            selection.toggle(focused);
        }
        // move the focus through the rows, extending the selection with shift, or open with Enter
        else {
            const target = this.focus.move(event, direction);
            const isExtending = event.shiftKey && selection?.isMultiple() === true;
            if (target !== undefined && focused !== undefined && isExtending) {
                selection.extendFrom(focused, target, this.collection);
            } else if (
                target === undefined &&
                event.key === "Enter" &&
                focused !== undefined &&
                (isOnItem || !isRoving)
            ) {
                event.preventDefault();
                this.act(focused);
            }
        }
    }

    /** List what handles the actions on the items, for the grid list's `ref`. */
    handlers(): ((element: Element) => void)[] {
        // handle no keys a search field takes
        if (this.focus.mode === "virtual") {
            return [];
        }

        // handle an action where an item is
        const { selection } = this;
        const onItem = <Inputs extends ActionInputs>(
            action: Action<Inputs>,
            isEnabled: (key: string) => boolean,
            run: (key: string) => void,
        ) =>
            handle(action, {
                isEnabled: (element) => {
                    const key = this.keyAt(element);

                    return key !== undefined && this.collection.has(key) && isEnabled(key);
                },
                run: ({ element }) => run(present(this.keyAt(element), "the item it runs on")),
            });
        const canStep = () => this.#options.onMove !== undefined;

        return [
            // give the items to the actions on them
            provideItems({
                targets: (element) => this.targets(element),
                reference: (key) => this.#options.reference?.(key),
            }),

            // open the item, and move through the items
            onItem(
                openItem,
                (key) => this.#options.onAction !== undefined && !this.collection.isDisabled(key),
                (key) => this.act(key),
            ),
            onItem(
                nextItem,
                () => this.#isRows(),
                (key) => this.#focusBeside(key, 1),
            ),
            onItem(
                previousItem,
                () => this.#isRows(),
                (key) => this.#focusBeside(key, -1),
            ),

            // select the item, every item, or none
            onItem(
                toggleSelection,
                () => selection !== undefined,
                (key) => selection?.toggle(key),
            ),
            handle(selectAll, {
                isEnabled: () => selection?.isMultiple() === true,
                run: () => selection?.selectAll(this.collection),
            }),
            handle(clearSelection, {
                isEnabled: () => (selection?.values().length ?? 0) > 0,
                run: () => selection?.clear(),
            }),

            // move the item with the selection it is in
            onItem(moveItemsUp, canStep, (key) => this.step(key, -1)),
            onItem(moveItemsDown, canStep, (key) => this.step(key, 1)),
        ];
    }

    /** Read the key of the item around an element, undefined outside one. */
    keyAt(element: Element | null): string | undefined {
        const cell = element?.closest<HTMLElement>("[role=gridcell][data-item]") ?? null;

        return cell?.dataset["item"];
    }

    /** List the items a person is on at an element: an item's selection, else the selection. */
    targets(element: Element | null): readonly string[] {
        const key = this.keyAt(element);
        if (key === undefined || !this.collection.has(key)) {
            return this.selection?.values().filter((each) => this.collection.has(each)) ?? [];
        }

        return this.selection?.targets(key, this.collection) ?? [key];
    }

    /** Report whether a drag or a key moves the items. */
    isMoving(): boolean {
        return this.#options.onMove !== undefined;
    }

    /** Report whether a row's heading stays in view over the rows of its section. */
    isSticky(): boolean {
        return this.#options.hasStickyHeadings === true;
    }

    /** Report whether an item takes the focus as the pointer moves onto it. */
    isFocusedOnHover(): boolean {
        return this.#options.shouldFocusOnHover === true;
    }

    /** Read the key of a row, which a drag knows it by: its heading's, else its first item's. */
    rowKey(row: GridRow): string {
        return row.kind === "heading" ? `${this.id} ${row.key}` : (row.keys[0] ?? row.key);
    }

    /** Report whether a row scrolled past the start of the viewport, where a heading sticks. */
    isPast(index: number): boolean {
        const offset = this.#virtualizer.scrollOffset() ?? 0;

        return offset > (this.#virtualizer.offsetOf(index, "start") ?? 0) + this.#pinned();
    }

    /** Hand a list around the rows' window until the rows unmount. */
    join(host: VirtualHost): void {
        this.#hosts.add(host);
        host.add({
            keys: () => this.#rows().keys,
            positions: () => this.#rows().places,
            reveal: (key) => {
                const index = this.#rows().places.get(key);
                if (index === undefined) {
                    throw new RangeError(`grid list reveal of a missing row '${key}'`);
                }
                this.#virtualizer.scrollToIndex(index);
            },
            scroller: this.#viewport,
        });
    }

    /** Hand on a drop: each item moved, with the selection it is in. */
    drop(move: SortableMove): void {
        // find the section it lands in: the row before's, else the first
        const { sections, order } = this.#rows();
        const section =
            move.previous === undefined ? order[0] : (sections.get(move.previous) ?? order[0]);
        if (section === undefined) {
            throw new TypeError("grid list drop into a grid list without sections");
        }

        // place it between the items of that section it lands between
        const keys = this.#itemsIn(section);
        const previous = keys.includes(move.previous ?? "") ? move.previous : undefined;
        this.place(
            this.selection?.targets(move.id, this.collection) ?? [move.id],
            section,
            previous,
        );
    }

    /** Move items into a section after one of its items, to its start without one. */
    place(keys: readonly string[], section: string, after?: string): void {
        // leave sorted items of the section alone, their owner placing them by its order
        const moving = new Set(keys);
        const { sections } = this.#rows();
        const isSorted = this.#options.order === "sorted";
        if (isSorted && keys.every((key) => sections.get(key) === section)) {
            return;
        }

        // place the first after the nearest item before that stays, and each after the one before
        const items = this.#itemsIn(section);
        const at = after === undefined ? -1 : items.indexOf(after);
        const next = items.slice(at + 1).find((key) => !moving.has(key));
        let previous = items.slice(0, at + 1).findLast((key) => !moving.has(key));
        for (const key of keys) {
            this.#options.onMove?.({ id: key, section, previous, next });
            previous = key;
        }
    }

    /** Move an item and the selection it is in past the next item, or into the next section. */
    step(key: string, step: 1 | -1): void {
        // read where the items stand
        const { sections, order } = this.#rows();
        const keys = this.selection?.targets(key, this.collection) ?? [key];
        const moving = new Set(keys);
        const section = present(sections.get(key), "the section of a grid list's item");
        const items = this.#itemsIn(section);
        const edge = step === 1 ? items.findLastIndex((each) => moving.has(each)) : -1;
        const ahead =
            step === 1
                ? items.slice(edge + 1)
                : items.slice(
                      0,
                      items.findIndex((each) => moving.has(each)),
                  );
        const beside = step === 1 ? ahead[0] : ahead.at(-1);
        const next = order[order.indexOf(section) + step];

        // step past the item beside, where a person orders them
        if (beside !== undefined && this.#options.order !== "sorted") {
            const before = ahead.at(-2);
            this.place(keys, section, step === 1 ? beside : before);
        }
        // else step into the next section, at its near end
        else if (next !== undefined) {
            this.place(keys, next, step === 1 ? undefined : this.#itemsIn(next).at(-1));
        }
        this.refocus(key);
    }

    /** Read the keys of the rows in order, which a change moves. */
    order(): readonly string[] {
        return this.#rows().keys;
    }

    /** Give the focused item the focus back where moving its row in the document dropped it. */
    keepFocus(): void {
        const key = this.focus.current();
        const isDropped = document.activeElement === document.body;
        if (this.focus.mode === "roving" && this.focus.isFocused() && isDropped) {
            this.#elementOf(key)?.focus({ preventScroll: true });
        }
    }

    /** Focus an item once its owner's change has rendered. */
    refocus(key: string): void {
        queueMicrotask(() => {
            flush();
            if (this.collection.has(key)) {
                this.focus.enter(key);
                this.focus.reveal(key);
                this.#elementOf(key)?.focus({ preventScroll: true });
            }
        });
    }

    /** Count the cells of the longest row. */
    columnCount(): number {
        return this.#options.layout === undefined
            ? this.columns()
            : this.delegate
                  .rows()
                  .reduce(
                      (most, row) => Math.max(most, row.kind === "cells" ? row.keys.length : 0),
                      1,
                  );
    }

    /** Follow the grid element, which a grid list without a viewport measures. */
    attach(element: HTMLElement | undefined): void {
        this.#root = element;
        this.#setElement(element);
    }

    /** The rows the viewport shows with some to spare by their places, every row without one. */
    shown(): readonly number[] {
        // show the rows in view, a first few until a viewport that will come is measured
        const items = this.#virtualizer.items();
        const count = this.delegate.rows().length;
        if (this.#viewport() !== undefined && items.length > 0) {
            return items.map((item) => item.index).filter((index) => index < count);
        }
        const length = this.#isObserved ? Math.min(count, UNMEASURED_ROWS) : count;

        return Array.from({ length }, (_row, index) => index);
    }

    /** The first of the rows in view, after the persisted ones before them. */
    start(): number {
        return this.#viewport() === undefined ? 0 : this.#virtualizer.runStart();
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

    /** Break a section's keys into rows as a layout breaks their items. */
    #break(keys: readonly string[], layout: RowLayout, width: number): readonly GridLine[] {
        // stand one in a row until the grid list is measured
        const ratios = keys.map((key) => {
            const item = this.collection.item(key);

            return item === undefined ? 1 : (this.#options.ratio?.(item) ?? 1);
        });
        if (width === 0) {
            return keys.map((key) => ({ keys: [key], size: undefined }));
        }

        return layout.rows(ratios, width).map(({ start, end, height }) => ({
            keys: keys.slice(start, end),
            size: {
                height,
                widths: ratios.slice(start, end).map((ratio) => ratio * height),
                gap: layout.gap,
            },
        }));
    }

    /** Scroll a key's row into the viewport. */
    #reveal(key: string): void {
        const place = this.delegate.place(key);
        if (this.#viewport() !== undefined && place !== undefined) {
            this.#virtualizer.scrollToIndex(place.row);
        }
    }

    /** Focus the item beside one, where a key steps through the items. */
    #focusBeside(key: string, step: 1 | -1): void {
        const index = this.collection.index(key);
        const target = index === undefined ? undefined : this.collection.at(index + step);
        if (target !== undefined) {
            this.focus.enter(target);
            this.focus.reveal(target);
        }
    }

    /** Report whether each item has a row of its own. */
    #isRows(): boolean {
        return this.columns() === 1;
    }

    /** Find the mounted element of an item, undefined for one not mounted. */
    #elementOf(key: string | undefined): HTMLElement | undefined {
        const found =
            key === undefined
                ? undefined
                : this.#root?.querySelector<HTMLElement>(`[data-item="${CSS.escape(key)}"]`);

        return found ?? undefined;
    }

    /** List the items of a section in order. */
    #itemsIn(section: string): readonly string[] {
        const found = this.collection.sections().find((each) => each.key === section);

        return (found?.items ?? []).map((item) => this.collection.keyOf(item));
    }

    /** Read the height a sticking heading takes from the rows, in pixels. */
    #pinned(): number {
        return this.isSticky() ? (this.#options.rowHeight ?? ROW_HEIGHT) : 0;
    }

    /** Index the rows by key: their places, and the section each stands in. */
    #index(): GridListRowIndex {
        // key and place each row
        const rows = this.delegate.rows();
        const keys = rows.map((row) => this.rowKey(row));
        const places = new Map(keys.map((key, index) => [key, index] as const));

        // find the section of each heading and item
        const sections = new Map<string, string>();
        const order = this.collection.sections().map((section) => section.key);
        for (const section of this.collection.sections()) {
            sections.set(`${this.id} ${section.key}`, section.key);
            for (const item of section.items) {
                sections.set(this.collection.keyOf(item), section.key);
            }
        }

        return { keys, places, sections, order };
    }

    /** List the rows to render for a visible range, with the heading that sticks over it. */
    #extract({ startIndex, endIndex, overscan, count }: VirtualRange): number[] {
        // take the rows in view with some to spare
        const start = Math.max(0, startIndex - overscan);
        const end = Math.min(count - 1, endIndex + overscan);
        const shown = Array.from({ length: Math.max(0, end - start + 1) }, (_row, i) => start + i);
        if (!this.isSticky()) {
            return shown;
        }

        // add the heading of the first one in view
        const rows = untrack(() => this.delegate.rows());
        const heading = rows.findLastIndex(
            (row, index) => index <= startIndex && row.kind === "heading",
        );

        return heading === -1 || heading >= start ? shown : [heading, ...shown];
    }
}

/** The rows of a grid list by key. */
interface GridListRowIndex {
    /** The rows' keys in order. */
    readonly keys: readonly string[];
    /** Each row's place among them, by key. */
    readonly places: ReadonlyMap<string, number>;
    /** The key of the section each heading and item stands in, by the heading's or item's key. */
    readonly sections: ReadonlyMap<string, string>;
    /** The sections' keys in order. */
    readonly order: readonly string[];
}

/** How a grid list orders the items of a section: by hand, or by an order its owner applies. */
export type GridListOrder = "manual" | "sorted";

/** A move a person made: the item, the section it lands in and the items it lands between. */
export interface GridListMove {
    /** The key of the moved item. */
    readonly id: string;
    /** The key of the section it lands in. */
    readonly section: string;
    /** The key of the item before it in the section, undefined at its start. */
    readonly previous: string | undefined;
    /** The key of the item after it in the section, undefined at its end. */
    readonly next: string | undefined;
}

/** The items, layout and action of a grid list. */
export interface GridListOptions<Item> extends CollectionOptions<Item> {
    /** The cells in each row, as many as fit by their least width when absent, else one. */
    readonly columns?: Accessor<number>;
    /** The layout that breaks the items into rows by their ratios, in place of columns. */
    readonly layout?: RowLayout;
    /** Read an item's ratio of width to height for a layout, square by default. */
    readonly ratio?: (item: Item) => number;
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
    /** Read the object an item stands for, which actions on its type take, absent for none. */
    readonly reference?: ((key: string) => ObjectReference | undefined) | undefined;
    /** Handle an item being opened with a press or Enter. */
    readonly onAction?: ((item: Item) => void) | undefined;
    /** Handle each item a drag or keys move, absent to leave the items in place. */
    readonly onMove?: ((move: GridListMove) => void) | undefined;
    /** How the sections order their items, by hand by default. */
    readonly order?: GridListOrder | undefined;
    /** Whether sections without items show their headings, as collapsed ones do. */
    readonly hasEmptySections?: boolean;
    /** Whether a heading stays in view while the rows of its section scroll under it. */
    readonly hasStickyHeadings?: boolean;
    /** Whether an item takes the focus as the pointer moves onto it, so keys act on it. */
    readonly shouldFocusOnHover?: boolean;
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
    /** The section's key. */
    readonly section: string;
    /** Whether it stays in view while the rows of its section scroll under it. */
    readonly isSticky: boolean;
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
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "children">
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
    /** The section's key. */
    readonly section: string;
    /** Whether it stays in view while the rows of its section scroll under it. */
    readonly isSticky: boolean;
    /** The styles of the heading's place: where it sticks, and where a drag lands beside it. */
    readonly xstyle: style.Styles;
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
    "focus" | "observe" | "state" | "isEmpty" | "isFitted" | "isFocusedOnHover"
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
    const actions = useActions();
    const isCommand = useCommandPlatform();
    const isVirtual = control.focus.mode === "virtual";
    const rest = omit(properties, "control", "label", "sectionHeader", "children");
    const attributes = {
        id: control.id,
        role: "grid" as const,
        get tabindex() {
            return isVirtual ? 0 : undefined;
        },
        get "aria-rowcount"() {
            return control.delegate.rows().length;
        },
        get "aria-colcount"() {
            return control.columnCount();
        },
        get "aria-activedescendant"() {
            return isVirtual ? control.focus.descendant() : undefined;
        },
        get "aria-multiselectable"() {
            return control.selection?.isMultiple() === true ? ("true" as const) : undefined;
        },
        ref: [(element: HTMLElement) => control.attach(element), ...control.handlers()],
        onKeyDown: (event: KeyboardEvent) =>
            control.keyDown(event, locale.direction, actions, isCommand()),
        onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
            control.focus.focusOut(event),
    };

    // keep the focus on a row a change moves
    createEffect(
        () => control.order(),
        () => untrack(() => control.keepFocus()),
    );

    // render the rows in a list a drag moves them in, or as they stand
    if (control.isMoving()) {
        return (
            <GridListContext value={control}>
                <Sortable
                    data-slot="grid-list"
                    onMove={(move) => control.drop(move)}
                    {...mergeProperties(attributes, rest)}
                    xstyle={[styles.list, rest.xstyle]}
                >
                    <SortableContainer id={control.id} accepts={[control.id]}>
                        <GridListRows {...properties} />
                    </SortableContainer>
                </Sortable>
            </GridListContext>
        );
    }

    // render the rows as they stand
    const rows = {
        get children() {
            return (
                <GridListContext value={control}>
                    <GridListRows {...properties} />
                </GridListContext>
            );
        },
    };

    return renderPart("div", "grid-list", rest, styles.list, merge(attributes, rows));
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
    const rest = omit(properties, "label", "section", "isSticky", "children");
    const look = (): style.Styles => [
        text.xs,
        styles.header,
        properties.isSticky && styles.covering,
    ];

    return renderPart("div", "grid-list-section-header", rest, look, {
        get children() {
            return contentOr(properties, () => <span role="columnheader">{properties.label}</span>);
        },
    });
}

/** Render a row of a grid list. */
export function GridListRow(properties: GridListRowProperties): JSX.Element {
    return renderPart("div", "grid-list-row", properties, styles.row);
}

/** Render a cell's element of a grid list, marked while focused. */
export function GridListCell(properties: GridListCellFrameProperties): JSX.Element {
    // fill the active cell unless its focus ring shows it
    const control = useGridList();
    const { isFitted, focus } = control;
    const isFilled = !isFitted || focus.mode === "virtual";
    const rest = omit(properties, "isActive", "isSelected");

    return renderPart(
        "div",
        "grid-list-cell",
        rest,
        () => [
            styles.cell,
            isFitted ? [state.focusInset, styles.fitted] : styles.square,
            control.isFocusedOnHover() &&
                focus.isFocused() &&
                properties.isActive &&
                styles.pointed,
            properties.isActive && isFilled && state.selected,
            properties.isSelected && state.selected,
            properties.isSelected && styles.selected,
            properties["aria-disabled"] === "true" && state.inert,
        ],
        {
            get "data-highlighted"() {
                return properties.isActive ? "" : undefined;
            },
            get "data-selected"() {
                return properties.isSelected ? "" : undefined;
            },
            get "data-disabled"() {
                return properties["aria-disabled"] === "true" ? "" : undefined;
            },
        },
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

/** Render the rows of a grid list in view, with room for those out of view around them. */
function GridListRows<Item>(properties: GridListProperties<Item>): JSX.Element {
    // hand the rows to the list a drag moves them in
    const control = properties.control;
    const host = useContext(VirtualHostContext);
    if (host !== null && control.isMoving()) {
        control.join(host);
    }

    // key the rows to render, each at the place it has now
    const shown = () => {
        const rows = control.delegate.rows();

        return control.shown().flatMap((index) => {
            const row = rows[index];

            return row === undefined ? [] : [{ key: control.rowKey(row), index }];
        });
    };

    return (
        <>
            <For each={shown()} keyed={(entry) => entry.key}>
                {(entry) => <GridListRowView properties={properties} index={() => entry().index} />}
            </For>
            <Show when={control.spacers().after > 0}>
                <div
                    aria-hidden="true"
                    {...style.attrs(styles.spacer(`${String(control.spacers().after)}px`))}
                />
            </Show>
        </>
    );
}

/** Render a row of a grid list at its place among every row, the ones out of view included. */
function GridListRowView<Item>(view: {
    /** The grid list's properties. */
    readonly properties: GridListProperties<Item>;
    /** The row's place among every row. */
    readonly index: Accessor<number>;
}): JSX.Element {
    // read the row, the last one it had while it leaves
    const { properties, index } = view;
    const control = properties.control;
    const row = createMemo<GridRow>(
        (before) => control.delegate.rows()[index()] ?? present(before, "a grid list's row"),
    );

    // size each cell of a row a layout broke, else share the row
    const attributes = rowAttributes(control, index);
    const look = (): style.Styles => [fit(), roomBefore(control, index())];
    const fit = (): style.Styles => {
        const current = row();
        if (current.kind === "cells" && current.size !== undefined) {
            const columns = current.size.widths.map((each) => `${String(each)}px`).join(" ");

            return styles.sized(columns, `${String(current.size.height)}px`, control.gap);
        }

        return control.isFitted && styles.fit(control.columns(), control.gap);
    };

    // render a heading
    const first = untrack(row);
    if (first.kind === "heading") {
        return <GridListHeading properties={properties} row={row} index={index} />;
    }

    // else a row of cells, which a drag lifts
    const keys = (): readonly string[] => {
        const current = row();

        return current.kind === "cells" ? current.keys : [];
    };
    const cells = (): JSX.Element => (
        <For each={keys()}>{(key, column) => renderCell(properties, key, column)}</For>
    );

    return (
        <Show
            when={control.isMoving()}
            fallback={
                <GridListRow {...attributes} xstyle={look()}>
                    {cells()}
                </GridListRow>
            }
        >
            <SortableItem
                id={control.rowKey(first)}
                type={control.id}
                activator="item"
                sensors={["pointer"]}
                data-slot="grid-list-row"
                {...attributes}
                xstyle={[styles.row, styles.lifted, look()]}
            >
                {cells()}
            </SortableItem>
        </Show>
    );
}

/** Leave the room of the rows out of view before the first in view. */
function roomBefore<Item>(control: GridListControl<Item>, index: number): style.Styles {
    const { before } = control.spacers();

    return index === control.start() && before > 0 && styles.after(`${String(before)}px`);
}

/** Number a row from 1 for assistive technology and from 0 for its measuring. */
function rowAttributes<Item>(control: GridListControl<Item>, index: Accessor<number>) {
    return {
        role: "row" as const,
        get "aria-rowindex"() {
            return String(index() + 1);
        },
        get "data-index"() {
            return String(index());
        },
        ref: (element: Element) => control.measure(element, untrack(index)),
    };
}

/** Render a section's heading, which a drag lands beside and which may stay in view. */
function GridListHeading<Item>(view: {
    /** The grid list's properties. */
    readonly properties: GridListProperties<Item>;
    /** The row. */
    readonly row: Accessor<GridRow>;
    /** The row's place among every row. */
    readonly index: Accessor<number>;
}): JSX.Element {
    // let a drag land beside it, apart from where it sticks over other rows
    const { properties, row, index } = view;
    const control = properties.control;
    const shared = rowAttributes(control, index);
    const sortable = control.isMoving() ? useSortable() : undefined;
    const key = control.rowKey(untrack(row));
    let element: HTMLElement | undefined;
    sortable?.addItem({
        id: key,
        container: control.id,
        element: () =>
            control.isSticky() && untrack(() => control.isPast(index())) ? undefined : element,
    });
    const heading: GridListHeadingAttributes = merge(shared, {
        get label() {
            const current = row();

            return current.kind === "heading" ? current.label : "";
        },
        get section() {
            return row().key;
        },
        isSticky: control.isSticky(),
        get xstyle(): style.Styles {
            return [
                control.isSticky() && styles.sticky,
                dropStyle(sortable?.dropOf(key), true),
                roomBefore(control, index()),
            ];
        },
        ref: (created: HTMLElement) => {
            element = created;
            shared.ref(created);
        },
    });

    return createComponent(properties.sectionHeader ?? GridListSectionHeader, heading);
}

/** Render a cell of a grid list with its id, place and focus, and its pointer handlers. */
function renderCell<Item>(
    properties: GridListProperties<Item>,
    key: string,
    column: Accessor<number>,
): JSX.Element {
    // read the cell's item, the last one it had while it leaves, refusing a key of none
    const control = properties.control;
    const item = createMemo<Item>((before) => {
        const found = control.collection.item(key) ?? before;
        if (found === undefined) {
            throw new TypeError(`a grid list has no item '${key}'`);
        }

        return found;
    });
    const isVirtual = control.focus.mode === "virtual";
    const isSelected = () =>
        control.selection === undefined
            ? isVirtual && control.focus.isActive(key)
            : control.selection.isSelected(key);
    const shared = {
        id: control.focus.id(key),
        role: "gridcell" as const,
        "data-item": key,
        get "aria-colindex"() {
            return String(column() + 1);
        },
        get "aria-selected"() {
            // say nothing about selection on a grid list that selects nothing
            if (control.selection === undefined && !isVirtual) {
                return undefined;
            }

            return isSelected() ? ("true" as const) : ("false" as const);
        },
        get "aria-label"() {
            return properties.label(item());
        },
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

        return renderItem(properties, merge(shared, pointed), item);
    }

    // give a cell without a search field a tab stop, and open or select it on a press
    let element: HTMLElement | undefined;
    control.focus.bind(
        () => key,
        () => element,
    );
    const press = new Press({
        onPressStart: (event) => {
            // let a drag lift the row around the cell
            if (control.isMoving()) {
                event.continuePropagation();
            }
        },
        onPress: (event) => control.press(key, event),
        keys: ["Enter"],
        isDisabled: () => control.collection.isDisabled(key),
        shouldCancelOnMove: control.isMoving(),
    });
    const focused = {
        get tabindex() {
            return control.focus.isActive(key) ? 0 : -1;
        },
        onFocus: () => control.focus.focusIn(key),
        onPointerMove: (event: PointerEvent & { readonly currentTarget: Element }) => {
            // take the focus as a mouse moves onto it, but from a field or an overlay it is not in
            const held = document.activeElement;
            const overlay = held?.closest(OVERLAY) ?? null;
            const isOwn = element?.contains(held) === true;
            const isCovered = overlay !== null && !overlay.contains(event.currentTarget);
            const isFree = !isTextField(held) && !isCovered;
            const isHovering = event.pointerType === "mouse" && event.buttons === 0;
            if (control.isFocusedOnHover() && isHovering && isFree && !isOwn) {
                element?.focus({ preventScroll: true });
            }
        },
        ref: (created: HTMLElement) => (element = created),
    };

    return renderItem(
        properties,
        merge(shared, mergeProperties(focused, press.attributes())),
        item,
    );
}

/** Render a cell with the caller's function, else the item's label in the default cell. */
function renderItem<Item>(
    properties: GridListProperties<Item>,
    attributes: GridListCellFrameProperties,
    item: Accessor<Item>,
): JSX.Element {
    const cell = properties.children;
    if (cell === undefined) {
        return <GridListCell {...attributes}>{properties.label(item())}</GridListCell>;
    }

    return createComponent(
        cell,
        merge(attributes, {
            get item() {
                return item();
            },
        }),
    );
}
