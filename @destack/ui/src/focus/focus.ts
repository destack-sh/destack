import type { Direction } from "@destack/locale";
import {
    type Accessor,
    createEffect,
    createMemo,
    createProjection,
    createSignal,
    createUniqueId,
    latest,
    untrack,
} from "@destack/view";

/** How a collection's keys lay out for the keyboard. */
export interface KeyboardDelegate<Key> {
    /** Read the key a key press moves the focus to from a key, undefined for a press the layout leaves alone. */
    target(event: KeyboardEvent, from: Key, direction: Direction): Key | undefined;
    /** Read the first key that takes the focus, undefined while none does. */
    first(): Key | undefined;
    /** Report whether a key takes the focus. */
    has(key: Key): boolean;
}

/** Whether the focused key holds the DOM focus as a roving tab stop, or stays virtual behind `aria-activedescendant`. */
export type FocusMode = "roving" | "virtual";

/** The layout, mode and starting key of a collection's focus. */
export interface FocusOptions<Key> {
    /** The layout the keys move through. */
    readonly delegate: KeyboardDelegate<Key>;
    /** Whether the focused key holds the DOM focus or stays virtual. */
    readonly mode: FocusMode;
    /** The key that holds the focus until one is focused, the delegate's first by default. */
    readonly initial?: () => Key | undefined;
    /** The focused key its owner holds, which makes the focus controlled while it returns a key. */
    readonly controlled?: () => Key | undefined;
    /** Handle a key taking the focus. */
    readonly onChange?: (key: Key) => void;
    /** Bring a key's item into view after the keys move the focus, its element by id by default. */
    readonly reveal?: (key: Key) => void;
    /** Write a key as text, unique among the keys, its string form by default. */
    readonly keyText?: (key: Key) => string;
}

/** The key focused by the keys or the person, none until one is, starting from the initial key. */
type FocusedKey<Key> =
    | { readonly kind: "initial" }
    | { readonly kind: "focused"; readonly key: Key };

/** The one focused key of a collection, moved by a keyboard delegate. */
export class Focus<Key> {
    /** The prefix of each item's id in virtual focus. */
    readonly prefix: string;
    /** Whether the focused key holds the DOM focus or stays virtual. */
    readonly mode: FocusMode;
    /** The focused key, the initial or first one until one is focused, undefined while none takes the focus. */
    readonly active: Accessor<Key | undefined>;
    /** Whether the DOM focus rests inside the collection. */
    readonly isFocused: Accessor<boolean>;
    /** The options the focus follows. */
    readonly #options: FocusOptions<Key>;
    /** The focused key of each item, by its key's text, which only the two items it moves between read. */
    readonly #actives: Readonly<Record<string, boolean>>;
    /** The focused key, or the initial one until one is focused. */
    readonly #focused: Accessor<FocusedKey<Key>>;
    /** Replace the focused key. */
    readonly #setFocused: (key: FocusedKey<Key>) => void;
    /** Replace whether the DOM focus rests inside the collection. */
    readonly #setFocusedWithin: (isFocused: boolean) => void;

    /** Start on the initial key, outside the DOM focus. */
    constructor(options: FocusOptions<Key>) {
        // follow the focused key and whether the collection holds the DOM focus
        const [focused, setFocused] = createSignal<FocusedKey<Key>>({ kind: "initial" });
        const [isFocused, setFocusedWithin] = createSignal(false);
        this.prefix = createUniqueId();
        this.mode = options.mode;
        this.isFocused = isFocused;
        this.#options = options;
        this.#setFocused = setFocused;
        this.#setFocusedWithin = setFocusedWithin;
        this.#focused = focused;

        // resolve the controlled, focused or initial key to one the layout takes, else its first
        this.active = createMemo(() => this.#resolve(focused()));

        // mark the focused key alone, so a move reruns only the items it leaves and lands on
        this.#actives = createProjection<Record<string, boolean>>((draft) => {
            const key = this.active();
            for (const name of Object.keys(draft)) {
                delete draft[name];
            }
            if (key !== undefined) {
                draft[this.#text(key)] = true;
            }
        }, {});
    }

    /** Read the focused key as last written, which a handler reads right after the DOM focus moved, before the next flush. */
    current(): Key | undefined {
        return this.#resolve(latest(this.#focused));
    }

    /** Report whether a key is the focused one. */
    isActive(key: Key): boolean {
        return this.#actives[this.#text(key)] === true;
    }

    /** Focus a key without moving the DOM focus there unless the collection holds it. */
    focus(key: Key): void {
        // tell the owner only of a key other than the focused one
        const current = this.current();
        const isChanged = current === undefined || this.#text(current) !== this.#text(key);
        this.#write({ kind: "focused", key });
        if (isChanged) {
            this.#options.onChange?.(key);
        }
    }

    /** Focus a key and move the DOM focus there in roving focus, as opening a menu does. */
    enter(key: Key): void {
        this.focus(key);
        this.#setFocusedWithin(true);
    }

    /** Forget the focused key, returning to the initial or first one, as a new search does. */
    reset(): void {
        this.#write({ kind: "initial" });
    }

    /** Move the focus on a key press the layout takes, keeping the key from scrolling the page, and return the key it lands on, undefined for a press that moves nothing. */
    move(event: KeyboardEvent, direction: Direction): Key | undefined {
        // read the key the press lands on from the focused one
        const from = this.current();
        const target =
            from === undefined ? undefined : this.#options.delegate.target(event, from, direction);
        if (target === undefined) {
            return undefined;
        }

        // focus it and bring it into view
        event.preventDefault();
        this.focus(target);
        this.reveal(target);

        return target;
    }

    /** Bring a key's item into view: by the owner's reveal, else its element by id in virtual focus. */
    reveal(key: Key): void {
        if (this.#options.reveal !== undefined) {
            this.#options.reveal(key);
        } else if (this.mode === "virtual") {
            document.getElementById(this.id(key))?.scrollIntoView({ block: "nearest" });
        }
    }

    /** Take the key of an item the DOM focus entered. */
    focusIn(key: Key): void {
        this.#write({ kind: "focused", key });
        this.#setFocusedWithin(true);
    }

    /** Follow the DOM focus leaving the collection's element for one outside it. */
    focusOut(event: FocusEvent & { readonly currentTarget: Element }): void {
        if (
            !(event.relatedTarget instanceof Node) ||
            !event.currentTarget.contains(event.relatedTarget)
        ) {
            this.#setFocusedWithin(false);
        }
    }

    /** Give an item's element the DOM focus whenever its key is focused while the collection holds the focus. */
    bind(key: () => Key, element: () => HTMLElement | undefined): void {
        createEffect(
            () => this.mode === "roving" && this.isFocused() && this.isActive(key()),
            (isTaking) => {
                // focus the element unless it already holds the focus, its handlers reading untracked
                const target = element();
                if (isTaking && target !== undefined && target !== document.activeElement) {
                    untrack(() => target.focus());
                }
            },
        );
    }

    /** Build the id of a key's item, which `aria-activedescendant` names in virtual focus. */
    id(key: Key): string {
        return `${this.prefix}-${encodeURIComponent(this.#text(key))}`;
    }

    /** Build the id of the focused key's item, undefined while none is focused. */
    descendant(): string | undefined {
        const key = this.active();

        return key === undefined ? undefined : this.id(key);
    }

    /** Write the focused key. */
    #write(focused: FocusedKey<Key>): void {
        this.#setFocused(focused);
    }

    /** Write a key as text. */
    #text(key: Key): string {
        return this.#options.keyText?.(key) ?? String(key);
    }

    /** Resolve the controlled, focused or initial key to one the layout takes, else its first. */
    #resolve(own: FocusedKey<Key>): Key | undefined {
        // take the owner's key, else the focused or initial one, while the layout takes it
        const delegate = this.#options.delegate;
        const held = this.#options.controlled?.();
        const key = held ?? (own.kind === "focused" ? own.key : this.#options.initial?.());

        return key !== undefined && delegate.has(key) ? key : delegate.first();
    }
}

/** Report whether a key types a letter that moves the focus by typeahead: printable, without Space or shortcut modifiers. */
export function isTypeaheadKey(event: KeyboardEvent): boolean {
    return (
        event.key.length === 1 &&
        event.key !== " " &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
    );
}
