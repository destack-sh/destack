import { type Accessor, createContext, onCleanup, untrack, useContext } from "@destack/view";
import { DialogControl } from "../dialog/index.ts";
import { tabbablesIn } from "../focus/index.ts";
import { OpenChange } from "../layer/index.ts";
import { Scale } from "./scale.ts";

/** What a viewer shows. */
export interface ViewerOptions<Item> {
    /** The items, in order. */
    readonly items: Accessor<readonly Item[]>;
    /** Read an item's key, unique among them. */
    readonly key: (item: Item) => string;
    /** Whether the last item is followed by the first. */
    readonly isLooping?: boolean;
}

/** An item as a viewer's stage renders it: shown, before it or after it. */
export interface ViewerPane<Item> {
    /** The item's key. */
    readonly key: string;
    /** The item. */
    readonly item: Item;
    /** Where it is: -1 before the one shown, 0 shown, 1 after. */
    readonly place: number;
}

/** The items of a viewer and the one it shows. */
export class ViewerControl<Item> {
    /** The dialog it shows them in. */
    readonly dialog: DialogControl<string>;
    /** How the picture shown is scaled. */
    readonly scale: Scale;
    /** The options. */
    readonly #options: ViewerOptions<Item>;
    /** The element that stands for each item on the page, by its key. */
    readonly #thumbnails: Map<string, HTMLElement>;

    /** Create a viewer that shows nothing yet. */
    constructor(options: ViewerOptions<Item>) {
        // show nothing in a dialog of its own
        this.dialog = new DialogControl<string>();
        this.scale = new Scale();
        this.#options = options;
        this.#thumbnails = new Map();
    }

    /** The items, in order. */
    get items(): readonly Item[] {
        return this.#options.items();
    }

    /** Count the items. */
    count(): number {
        return this.items.length;
    }

    /** Read the place of the item shown, -1 for one that is gone. */
    index(): number {
        const key = this.dialog.state.payload();

        return this.items.findIndex((item) => this.#options.key(item) === key);
    }

    /** Read the key of the item shown, undefined before one is. */
    key(): string | undefined {
        return this.dialog.state.payload();
    }

    /** Read the item shown, undefined before one is or once it is gone. */
    item(): Item | undefined {
        return this.items[this.index()];
    }

    /** Take the element that stands for an item on the page, which it grows from. */
    thumbnail(item: Item): (element: HTMLElement) => void {
        // forget it as it unmounts
        const key = this.#options.key(item);
        let taken: HTMLElement | undefined;
        onCleanup(() => {
            if (this.#thumbnails.get(key) === taken) {
                this.#thumbnails.delete(key);
            }
        });

        return (element) => {
            taken = element;
            this.#thumbnails.set(key, element);
        };
    }

    /** Read the element that stands for the item shown, undefined for one without any on screen. */
    shown(): HTMLElement | undefined {
        const key = untrack(() => this.key());
        const element = key === undefined ? undefined : this.#thumbnails.get(key);

        return element?.isConnected === true ? element : undefined;
    }

    /** Show an item, from the event that asks for it. */
    open(item: Item, event?: Event & { readonly currentTarget: HTMLElement }): void {
        // open from what asks for it, else from what stands for it
        const key = this.#options.key(item);
        const element = event?.currentTarget ?? this.#thumbnails.get(key) ?? document.body;
        const trigger = { element, payload: key };
        this.dialog.open(new OpenChange("trigger-press", event, trigger));
    }

    /** Report whether there is an item a step away. */
    has(step: 1 | -1): boolean {
        return this.#at(step) !== undefined;
    }

    /** Show the item a step away, if there is one. */
    step(step: 1 | -1): void {
        const item = untrack(() => this.#at(step));
        if (item !== undefined) {
            this.#show(item);
        }
    }

    /** Show the first item, or the last. */
    end(end: "first" | "last"): void {
        const items = untrack(() => this.items);
        const item = end === "first" ? items[0] : items.at(-1);
        if (item !== undefined) {
            this.#show(item);
        }
    }

    /** Hand the focus to the element that stands for the item shown last. */
    restore(): void {
        const thumbnail = this.shown();
        if (thumbnail !== undefined) {
            (tabbablesIn(thumbnail)[0] ?? thumbnail).focus({ preventScroll: true });
        }
    }

    /** List the item before the one shown, the one shown and the one after, where they are. */
    panes(): readonly ViewerPane<Item>[] {
        const places = [
            { item: this.#at(-1), place: -1 },
            { item: this.item(), place: 0 },
            { item: this.#at(1), place: 1 },
        ];

        // show an item once, where two steps lead to it
        const panes = places.flatMap(({ item, place }) =>
            item === undefined ? [] : [{ key: this.#options.key(item), item, place }],
        );

        return panes.filter(
            ({ key, place }, i) =>
                place === 0 ||
                !panes.some((each, at) => each.key === key && (each.place === 0 || at < i)),
        );
    }

    /** Show an item in the open viewer. */
    #show(item: Item): void {
        // fit it, keeping where the viewer opened from
        this.scale.reset();
        const element = untrack(this.dialog.state.trigger)?.element ?? document.body;
        const trigger = { element, payload: this.#options.key(item) };
        this.dialog.open(new OpenChange("none", undefined, trigger));
    }

    /** Read the item a step from the one shown. */
    #at(step: 1 | -1): Item | undefined {
        // find the one shown among several
        const { items } = this;
        const index = this.index();
        const next = index + step;
        if (index === -1 || items.length < 2) {
            return undefined;
        }

        return this.#options.isLooping === true
            ? items[(next + items.length) % items.length]
            : items[next];
    }
}

/** A viewer as its parts read it, whatever it shows. */
export type ViewerState = Pick<
    ViewerControl<never>,
    "dialog" | "scale" | "count" | "index" | "key" | "has" | "step" | "end" | "shown" | "restore"
>;

/** The nearest viewer, null outside one. */
export const ViewerContext = createContext<ViewerState | null>(null);

/** Read the nearest viewer, refusing a part outside one. */
export function useViewer(): ViewerState {
    const viewer = useContext(ViewerContext);
    if (viewer === null) {
        throw new TypeError("a part of a viewer needs a viewer around it");
    }

    return viewer;
}
