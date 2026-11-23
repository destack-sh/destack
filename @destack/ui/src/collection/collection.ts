import {
    type Accessor,
    createEffect,
    createMemo,
    createProjection,
    createSignal,
    isHydrating,
    isServer,
    onCleanup,
    onSettled,
} from "@destack/view";
import { inDocumentOrder, observeDocumentOrder } from "./order.ts";

/** A section of a collection: its items under an optional heading. */
export interface CollectionSection<Item> {
    /** The key of the section. */
    readonly key: string;
    /** The heading shown above the section's items, none when absent. */
    readonly label?: string | undefined;
    /** The items of the section in order. */
    readonly items: readonly Item[];
}

/** Whether a collection's items are still loading, loaded, or failed to load. */
export type CollectionState = "loading" | "loaded" | "failed";

/** The sections of a collection and how it reads the key, text and availability of each item. */
export interface CollectionOptions<Item> {
    /** The sections in order, read once the load the items wait for has finished. */
    readonly sections: Accessor<readonly CollectionSection<Item>[]>;
    /** Read the key of an item, unique in the collection. */
    readonly key: (item: Item) => string;
    /** Read the text of an item, which typeahead matches. */
    readonly text: (item: Item) => string;
    /** Report whether an item is unavailable, every item available by default. */
    readonly isDisabled?: (item: Item) => boolean;
    /** The load the items wait for, loaded at once without one. */
    readonly load?: Load<unknown>;
}

/** The items of a collection laid out for lookups by key and by place. */
interface CollectionIndex<Item> {
    /** The sections in order. */
    readonly sections: readonly CollectionSection<Item>[];
    /** The keys of every item in order. */
    readonly keys: readonly string[];
    /** The item of each key. */
    readonly items: ReadonlyMap<string, Item>;
    /** The place of each key among every item. */
    readonly places: ReadonlyMap<string, number>;
}

/** The items of a collection component in order by key. */
export class Collection<Item> {
    /** Whether the items are still loading, loaded, or failed to load. */
    readonly state: Accessor<CollectionState>;
    /** How the collection reads each item. */
    readonly #options: CollectionOptions<Item>;
    /** The items laid out by key and place, none while loading, throwing a failed load's error. */
    readonly #index: Accessor<CollectionIndex<Item>>;
    /** Whether each key has an item, which only the keys that come or go read anew. */
    readonly #members: Readonly<Record<string, boolean>>;

    /** Lay out a collection's sections once their load finishes. */
    constructor(options: CollectionOptions<Item>) {
        // lay the items out by key and place once their load finishes
        this.#options = options;
        this.state = options.load?.state ?? (() => "loaded");
        this.#index = createMemo(() => {
            // throw a failed load to the nearest error boundary and list no items while loading
            options.load?.check();
            if (this.state() === "loading") {
                return layOut([], options.key);
            }

            return layOut(options.sections(), options.key);
        });

        // mark each key, so a change reruns only the items that come or go
        this.#members = createProjection<Record<string, boolean>>((draft) => {
            const places = this.#index().places;
            for (const name of Object.keys(draft)) {
                if (!places.has(name)) {
                    delete draft[name];
                }
            }
            for (const name of places.keys()) {
                draft[name] = true;
            }
        }, {});
    }

    /** The sections in order. */
    sections(): readonly CollectionSection<Item>[] {
        return this.#index().sections;
    }

    /** The keys of every item in order. */
    keys(): readonly string[] {
        return this.#index().keys;
    }

    /** The number of items. */
    size(): number {
        return this.#index().keys.length;
    }

    /** Report whether an item has a key. */
    has(key: string): boolean {
        return this.#members[key] === true;
    }

    /** Read the item of a key, undefined for a key of no item. */
    item(key: string): Item | undefined {
        return this.#index().items.get(key);
    }

    /** Read the place of a key among every item, undefined for a key of no item. */
    index(key: string): number | undefined {
        return this.#index().places.get(key);
    }

    /** Read the key at a place among every item, undefined past the ends. */
    at(index: number): string | undefined {
        return this.#index().keys[index];
    }

    /** Read the key of an item. */
    keyOf(item: Item): string {
        return this.#options.key(item);
    }

    /** Read the text of a key's item, which typeahead matches, empty for a key of no item. */
    text(key: string): string {
        const item = this.item(key);

        return item === undefined ? "" : this.#options.text(item);
    }

    /** Report whether a key's item is unavailable, or no item has the key. */
    isDisabled(key: string): boolean {
        const item = this.item(key);

        return item === undefined || this.#options.isDisabled?.(item) === true;
    }
}

/** A value a promise delivers, followed from loading to loaded or failed. */
export class Load<Value> {
    /** Whether the value is still loading, loaded, or failed to load. */
    readonly state: Accessor<CollectionState>;
    /** The outcome once the promise settles. */
    readonly #outcome: Accessor<LoadOutcome<Value>>;

    /** Follow a promise until it settles. */
    constructor(promise: Promise<Value>) {
        // start loading and store the outcome the promise settles with
        const [outcome, setOutcome] = createSignal<LoadOutcome<Value>>({ kind: "loading" });
        this.#outcome = outcome;
        this.state = () => this.#outcome().kind;
        promise.then(
            (value) => setOutcome({ kind: "loaded", value }),
            (error: unknown) => setOutcome({ kind: "failed", error }),
        );
    }

    /** Read the loaded value. */
    value(): Value {
        const outcome = this.#outcome();
        if (outcome.kind === "loading") {
            throw new TypeError("the load has no value yet");
        } else if (outcome.kind === "failed") {
            throw outcome.error;
        }

        return outcome.value;
    }

    /** Throw a failure's error, so the nearest error boundary shows it. */
    check(): void {
        const outcome = this.#outcome();
        if (outcome.kind === "failed") {
            throw outcome.error;
        }
    }
}

/** How a load settled: still loading, loaded with its value, or failed with its error. */
type LoadOutcome<Value> =
    | { readonly kind: "loading" }
    | { readonly kind: "loaded"; readonly value: Value }
    | { readonly kind: "failed"; readonly error: unknown };

/** The items child components add as they mount, in the order of their elements. */
export class CollectionBuilder<Item> {
    /** The mounts so far, only appended to, so a snapshot of their first stays as it was. */
    #mounts: CollectionMount<Item>[] = [];
    /** The number of mounts unmounted since. */
    #unmounted = 0;
    /** The latest snapshot, which a server and a hydrating page read, whose signals stand still. */
    #latest: CollectionSnapshot<Item>;
    /** The snapshot of the frame on screen. */
    readonly #shown: Accessor<CollectionSnapshot<Item>>;
    /** Replace the snapshot. */
    readonly #show: (snapshot: CollectionSnapshot<Item>) => void;
    /** Read an item's element, the items staying in mount order without one. */
    readonly #element: ((item: Item) => Element | undefined) | undefined;
    /** Read an item's key, the latest item of a key standing for it, as a move remounts. */
    readonly #key: ((item: Item) => string) | undefined;
    /** The items laid out last, and the snapshot they came from. */
    #layout: CollectionLayout<Item> | undefined;

    /** Start without items, ordering them by their elements and replacing them by their keys. */
    constructor(element?: (item: Item) => Element | undefined, key?: (item: Item) => string) {
        // start without items
        this.#latest = { mounts: this.#mounts, length: 0 };
        const [shown, show] = createSignal(this.#latest, { ownedWrite: true });
        this.#shown = shown;
        this.#show = show;
        this.#element = element;
        this.#key = key;

        // order the items again as their elements move, after Base UI's CompositeList
        if (element !== undefined) {
            createEffect(
                () => this.items().flatMap((item) => element(item) ?? []),
                (elements) => observeDocumentOrder(elements, () => this.#change()),
            );
        }
    }

    /** List the items in the order of their elements in the document, else in mount order. */
    items(): readonly Item[] {
        return this.#layOut().items;
    }

    /** Read the number of items. */
    size(): number {
        return this.#layOut().items.length;
    }

    /** Read the latest item of a key, none for a key of none. */
    item(key: string): Item | undefined {
        return this.#layOut().byKey.get(key);
    }

    /** Add an item until the component adding it unmounts, returning its place among the items. */
    add(item: Item): Accessor<number> {
        // add a mount until the component unmounts
        const mount = { item, isMounted: true };
        this.#mounts.push(mount);
        this.#change();
        onCleanup(() => {
            // mark it unmounted
            mount.isMounted = false;
            this.#unmounted += 1;

            // drop the unmounted ones once they are half of all
            if (this.#unmounted * 2 > this.#mounts.length) {
                this.#mounts = this.#mounts.filter((each) => each.isMounted);
                this.#unmounted = 0;
            }

            // show what is left
            this.#change();
        });

        // order the items again once the item's element stands in the document
        if (this.#element !== undefined) {
            onSettled(() => this.#change());
        }

        return () => this.items().indexOf(item);
    }

    /** Take a new snapshot, and show it with the frame of the browser's change. */
    #change(): void {
        this.#latest = { mounts: this.#mounts, length: this.#mounts.length };
        if (!isServer) {
            this.#show(this.#latest);
        }
    }

    /** Lay the mounted items out once per change. */
    #layOut(): CollectionLayout<Item> {
        // reuse the layout of the same snapshot
        const shown = this.#shown();
        const snapshot = isServer || isHydrating() ? this.#latest : shown;
        if (this.#layout?.snapshot === snapshot) {
            return this.#layout;
        }

        // take the mounted items, the latest of each key
        const key = this.#key;
        const byKey = new Map<string, Item>();
        let items: readonly Item[] = snapshot.mounts
            .slice(0, snapshot.length)
            .flatMap((mount) => (mount.isMounted ? [mount.item] : []));
        if (key !== undefined) {
            items.forEach((item) => byKey.set(key(item), item));
            items = items.filter((item) => byKey.get(key(item)) === item);
        }

        // order the items as their elements stand in the document
        if (this.#element !== undefined) {
            items = inDocumentOrder(items, this.#element);
        }
        this.#layout = { snapshot, items, byKey };

        return this.#layout;
    }
}

/** One mount of an item. */
interface CollectionMount<Item> {
    /** The item. */
    readonly item: Item;
    /** Whether the component that added it is still mounted. */
    isMounted: boolean;
}

/** The mounts up to a change. */
interface CollectionSnapshot<Item> {
    /** The mounts, with later ones past the length. */
    readonly mounts: readonly CollectionMount<Item>[];
    /** The number of mounts up to the change. */
    readonly length: number;
}

/** The mounted items laid out for lookups by place and by key. */
interface CollectionLayout<Item> {
    /** The snapshot laid out. */
    readonly snapshot: CollectionSnapshot<Item>;
    /** The items in order. */
    readonly items: readonly Item[];
    /** The latest item of each key. */
    readonly byKey: ReadonlyMap<string, Item>;
}

/** Lay sections out by key and place. */
function layOut<Item>(
    sections: readonly CollectionSection<Item>[],
    key: (item: Item) => string,
): CollectionIndex<Item> {
    // number every item across the sections
    const keys: string[] = [];
    const items = new Map<string, Item>();
    const places = new Map<string, number>();
    for (const section of sections) {
        for (const item of section.items) {
            // refuse a key another item has
            const name = key(item);
            if (places.has(name)) {
                throw new TypeError(`two items of a collection have the key '${name}'`);
            }

            // place the item after every earlier one
            places.set(name, keys.length);
            items.set(name, item);
            keys.push(name);
        }
    }

    return { sections, keys, items, places };
}
