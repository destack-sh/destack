import type { Direction } from "@destack/locale";
import type { Accessor } from "@destack/view";
import { Collection, CollectionBuilder } from "../collection/index.ts";
import { Focus, type FocusMode, isTypeaheadKey, type KeyboardDelegate } from "./focus.ts";

/** The arrow keys that move the focus through a list: left and right, up and down, or all four. */
export type Orientation = "horizontal" | "vertical" | "both";

/** The axis, wrapping and typeahead of a list's keys. */
export interface ListDelegateOptions {
    /** The arrow keys that move the focus. */
    readonly orientation: Orientation;
    /** Whether the arrow keys wrap from the last key to the first and back. */
    readonly isLooping: boolean;
    /** Whether a typed letter moves the focus to the next key whose text starts with it. */
    readonly isTypeahead: boolean;
}

/** The step a list key moves the focus by, or the end it moves the focus to. */
type ListMove = "next" | "previous" | "first" | "last";

/** The keys of a collection in order, moved through along an axis past unavailable ones. */
export class ListDelegate<Item> implements KeyboardDelegate<string> {
    /** The collection whose keys the focus moves through. */
    readonly collection: Collection<Item>;
    /** The axis, wrapping and typeahead, read on each key press. */
    readonly #options: ListDelegateOptions;

    /** Move through a collection's keys along an axis. */
    constructor(collection: Collection<Item>, options: ListDelegateOptions) {
        this.collection = collection;
        this.#options = options;
    }

    /** Read the key a key press moves the focus to: the arrows along the axis, Home, End and typed letters. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // move along the axis or to an end
        const move = listMoveOf(event.key, this.#options.orientation, direction);
        if (move === "next") {
            return this.after(from);
        } else if (move === "previous") {
            return this.before(from);
        } else if (move === "first") {
            return this.first();
        } else if (move === "last") {
            return this.last();
        }
        // move to the next key starting with a typed letter
        else if (this.#options.isTypeahead && isTypeaheadKey(event)) {
            return this.search(event.key, from);
        }
        // leave every other key alone
        else {
            return undefined;
        }
    }

    /** Read the first available key. */
    first(): string | undefined {
        return this.#scan(0, 1, this.collection.size());
    }

    /** Read the last available key. */
    last(): string | undefined {
        return this.#scan(this.collection.size() - 1, -1, this.collection.size());
    }

    /** Read the available key after one, wrapping to the first in a looping list. */
    after(key: string): string | undefined {
        const index = this.collection.index(key) ?? -1;
        const found = this.#scan(index + 1, 1, this.collection.size() - index - 1);

        return found ?? (this.#options.isLooping ? this.first() : undefined);
    }

    /** Read the available key before one, wrapping to the last in a looping list. */
    before(key: string): string | undefined {
        const index = this.collection.index(key) ?? this.collection.size();
        const found = this.#scan(index - 1, -1, index);

        return found ?? (this.#options.isLooping ? this.last() : undefined);
    }

    /** Report whether a key is an available one. */
    has(key: string): boolean {
        return !this.collection.isDisabled(key);
    }

    /** Read the next available key after one whose text starts with a letter, wrapping around. */
    search(letter: string, from: string): string | undefined {
        // search from the key after the focused one, wrapping once around the list
        const size = this.collection.size();
        const start = (this.collection.index(from) ?? -1) + 1;
        const lowered = letter.toLowerCase();
        for (let offset = 0; offset < size; offset++) {
            const key = this.collection.at((start + offset) % size);
            if (
                key !== undefined &&
                this.has(key) &&
                startsWith(this.collection.text(key), lowered)
            ) {
                return key;
            }
        }

        return undefined;
    }

    /** Read the first available key from a place in a direction, looking at most a number of keys. */
    #scan(start: number, step: 1 | -1, count: number): string | undefined {
        for (let offset = 0; offset < count; offset++) {
            const key = this.collection.at(start + offset * step);
            if (key !== undefined && this.has(key)) {
                return key;
            }
        }

        return undefined;
    }
}

/** Read the move a key asks for in a list of an orientation and direction, if any. */
function listMoveOf(
    key: string,
    orientation: Orientation,
    direction: Direction,
): ListMove | undefined {
    // arrows along the list's axes, mirrored left and right in right-to-left text
    const isHorizontal = orientation !== "vertical";
    const isVertical = orientation !== "horizontal";
    const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    // the move of each key
    if ((isHorizontal && key === forward) || (isVertical && key === "ArrowDown")) {
        return "next";
    } else if ((isHorizontal && key === backward) || (isVertical && key === "ArrowUp")) {
        return "previous";
    } else if (key === "Home") {
        return "first";
    } else if (key === "End") {
        return "last";
    } else {
        return undefined;
    }
}

/** Report whether a text starts with a lowercased letter, ignoring case and leading spaces. */
function startsWith(text: string, letter: string): boolean {
    return text.trim().toLowerCase().startsWith(letter);
}

/** An item of a list state: its key, text and availability. */
export interface ListItem {
    /** The key of the item, unique in its list. */
    readonly key: string;
    /** The text of the item, which typeahead matches. */
    readonly text: Accessor<string>;
    /** Whether the item is unavailable. */
    readonly isDisabled: Accessor<boolean>;
    /** The item's element, which orders it among the others, undefined until it renders. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** The axis, focus mode and starting key of a list state. */
export interface ListStateOptions extends ListDelegateOptions {
    /** Whether the focused key holds the DOM focus or stays virtual, roving by default. */
    readonly mode?: FocusMode;
    /** Read the key that holds the focus until one is focused from the items, the first available one by default. */
    readonly initial?: (collection: Collection<ListItem>) => string | undefined;
}

/** The items child components add in document order and the one focused key moving through them. */
export class ListState {
    /** The items in document order. */
    readonly items: CollectionBuilder<ListItem>;
    /** The items as a collection by key. */
    readonly collection: Collection<ListItem>;
    /** The keys the focus moves through along the axis. */
    readonly delegate: ListDelegate<ListItem>;
    /** The focused key. */
    readonly focus: Focus<string>;

    /** Start without items. */
    constructor(options: ListStateOptions) {
        // collect the items in the order of their elements and move one focused key through them
        this.items = new CollectionBuilder((item) => item.element());
        this.collection = new Collection({
            sections: () => [{ key: "items", items: this.items.items() }],
            key: (item) => item.key,
            text: (item) => item.text(),
            isDisabled: (item) => item.isDisabled(),
        });
        this.delegate = new ListDelegate(this.collection, options);
        this.focus = new Focus({
            delegate: this.delegate,
            mode: options.mode ?? "roving",
            initial: () => options.initial?.(this.collection),
        });
    }

    /** Add an item until the component adding it unmounts, giving its element the DOM focus as the focus reaches it. */
    add(item: ListItem): void {
        this.items.add(item);
        this.focus.bind(() => item.key, item.element);
    }
}
