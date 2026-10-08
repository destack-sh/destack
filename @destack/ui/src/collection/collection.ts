import {
    type Accessor,
    createMemo,
    createProjection,
    createSignal,
    onCleanup,
    onSettled,
} from "@destack/view";

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

/** The sections a collection holds and how it reads the key, text and availability of each item. */
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

    /** Lay out the sections a collection holds once their load finishes. */
    constructor(options: CollectionOptions<Item>) {
        // lay the items out by key and place once their load finishes
        this.#options = options;
        this.state = options.load?.state ?? (() => "loaded");
        this.#index = createMemo(() => {
            // throw a failed load to the nearest error boundary and hold no items while loading
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
        // start loading and keep the outcome the promise settles with
        const [outcome, setOutcome] = createSignal<LoadOutcome<Value>>({ kind: "loading" });
        this.#outcome = outcome;
        this.state = () => this.#outcome().kind;
        promise.then(
            (value) => setOutcome({ kind: "loaded", value }),
            (error: unknown) => setOutcome({ kind: "failed", error }),
        );
    }

    /** Read the loaded value, refusing to read it before it loads and throwing a failure's error. */
    value(): Value {
        const outcome = this.#outcome();
        if (outcome.kind === "loading") {
            throw new TypeError("a load has no value until it loads");
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
    /** The items in the order of their elements in the document, else in mount order. */
    readonly items: Accessor<readonly Item[]>;
    /** The items as they mount, the unmounted ones dropped once per change. */
    #items: Item[];
    /** The items unmounted since the last change. */
    readonly #unmounted: Set<Item>;
    /** Count each change to the items. */
    readonly #setRevision: (revise: (revision: number) => number) => void;

    /** Start without items, ordering them by the element each item reads, if any. */
    constructor(element?: (item: Item) => Element | undefined) {
        // count each change to the items, which start empty
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#items = [];
        this.#unmounted = new Set();
        this.#setRevision = setRevision;
        this.items = createMemo(() => {
            // drop the unmounted items in one pass however many leave at once
            revision();
            if (this.#unmounted.size > 0) {
                this.#items = this.#items.filter((item) => !this.#unmounted.has(item));
                this.#unmounted.clear();
            }

            // order the items as their elements stand in the document
            return element === undefined
                ? [...this.#items]
                : this.#items.toSorted((left, right) =>
                      documentOrder(element(left), element(right)),
                  );
        });
    }

    /** Add an item until the component adding it unmounts. */
    add(item: Item): void {
        // append in place, one copy per flush however many items mount
        this.#items.push(item);
        this.#setRevision((revision) => revision + 1);

        // order the items again once the item's element stands in the document
        onSettled(() => {
            this.#setRevision((revision) => revision + 1);
        });
        onCleanup(() => {
            this.#unmounted.add(item);
            this.#setRevision((revision) => revision + 1);
        });
    }
}

/** Compare two elements by their place in the document, keeping the order of a missing one. */
function documentOrder(left: Element | undefined, right: Element | undefined): number {
    // keep the order of an item without an element in the document
    if (left === undefined || right === undefined || !left.isConnected || !right.isConnected) {
        return 0;
    }

    return left.compareDocumentPosition(right) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1;
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
                throw new TypeError(`two items of a collection have the key ${name}`);
            }

            // place the item after every earlier one
            places.set(name, keys.length);
            items.set(name, item);
            keys.push(name);
        }
    }

    return { sections, keys, items, places };
}
