import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, createMemo, createSignal, untrack } from "solid-js";
import { access, type MaybeAccessor, TRANSPARENT } from "./utils.ts";
import { makeEventListener } from "./event-listener.ts";
import { createSingletonRoot } from "./rootless.ts";

/** A modifier key, as keyboard events name it. */
export type ModifierKey = "Alt" | "Control" | "Meta" | "Shift";

/** A key, as keyboard events name it, such as `Control`, `A` or `Escape`. */
export type KbdKey = ModifierKey | (string & {});

/** How a shortcut fires. */
export type ShortcutOptions = {
    /** Whether the shortcut's key presses have their default prevented, true by default. */
    readonly preventDefault?: boolean;
    /** Whether the shortcut fires once until every key is released, false by default. */
    readonly requireReset?: boolean;
    /** Whether the shortcut stays quiet while a field takes the typing, false by default. */
    readonly ignoreWithinInputs?: boolean;
    /** Whether the keys may be pressed in any order, false by default. */
    readonly anyOrder?: boolean;
};

/** How a key-down listener attaches. */
export interface CreateKeyDownOptions {
    /** Whether the listener is inactive. */
    readonly disabled?: MaybeAccessor<boolean | undefined>;
    /** The document to listen on, the window's by default, such as an iframe's. */
    readonly ownerDocument?: Accessor<Document | undefined>;
}

/** The modifier keys, upper-cased as the primitives track keys. */
const MODIFIERS = new Set(["ALT", "CONTROL", "META", "SHIFT"]);

/** Follow the latest key-down event, null again after the event's task, through one listener shared by every user. */
export const useKeyDownEvent: () => Accessor<KeyboardEvent | null> = createSingletonRoot(() => {
    // follow nothing on the server
    if (isServer) {
        return () => null;
    }
    const [event, setEvent] = createSignal<KeyboardEvent | null>(null, { ownedWrite: true });
    makeEventListener(window, "keydown", (latest) => {
        setEvent(latest);
        setTimeout(() => setEvent(null));
    });

    return event;
});

/** Follow the keys held down, in the order pressed, through one listener shared by every user. */
export const useKeyDownList: () => Accessor<string[]> = createSingletonRoot(() => {
    // follow nothing on the server
    if (isServer) {
        return () => [];
    }
    const [pressed, setPressed] = createSignal<string[]>([], { ownedWrite: true });
    const reset = (): void => {
        setPressed([]);
    };

    // add each newly pressed key, with the modifiers held before listening began
    makeEventListener(window, "keydown", (event) => {
        // read the newly pressed key
        const key = keyOf(event);
        const current = pressed();
        if (event.repeat || key === undefined || current.includes(key)) {
            return;
        }
        setPressed(current.length === 0 ? [...heldModifiers(event, key), key] : [...current, key]);
    });

    // drop released keys, and everything once Meta goes, as some platforms send no other key-up while it is held
    makeEventListener(window, "keyup", (event) => {
        const key = keyOf(event);
        if (key === undefined) {
            return;
        }
        if (key === "META") {
            reset();
        } else {
            setPressed((previous) => previous.filter((held) => held !== key));
        }
    });

    // forget every key when the window loses focus or a context menu opens
    makeEventListener(window, "blur", reset);
    makeEventListener(window, "contextmenu", (event) => {
        if (!event.defaultPrevented) {
            reset();
        }
    });

    return pressed;
});

/** Follow the key held alone, pressed while nothing else was, through one listener shared by every user. */
export const useCurrentlyHeldKey: () => Accessor<string | null> = createSingletonRoot(() => {
    // follow nothing on the server
    if (isServer) {
        return () => null;
    }
    const keys = useKeyDownList();
    let previous = untrack(keys);

    return createMemo(() => {
        // report a key pressed while nothing else was held
        const current = keys();
        const before = previous;
        previous = current;

        return before.length === 0 && current.length === 1 ? (current[0] ?? null) : null;
    }, TRANSPARENT);
});

/** Follow the sequence of key sets held since all keys were last released, through one listener shared by every user. */
export const useKeyDownSequence: () => Accessor<string[][]> = createSingletonRoot(() => {
    // follow nothing on the server
    if (isServer) {
        return () => [];
    }
    const keys = useKeyDownList();

    return createMemo(
        (previous: string[][] | undefined) =>
            keys().length === 0 ? [] : [...(previous ?? []), keys()],
        TRANSPARENT,
    );
});

/** Follow whether a key is held alone, preventing its default by default. */
export function createKeyHold(
    key: KbdKey,
    options: { readonly preventDefault?: boolean } = {},
): Accessor<boolean> {
    // follow nothing on the server
    if (isServer) {
        return () => false;
    }
    const wanted = key.toUpperCase();
    const held = useCurrentlyHeldKey();

    // prevent the key's default right in the event, which signals read only after the batch
    if (options.preventDefault ?? true) {
        makeEventListener(window, "keydown", (event) => {
            if (keyOf(event) === wanted) {
                event.preventDefault();
            }
        });
    }

    return createMemo(() => held() === wanted, TRANSPARENT);
}

/** Call back when a key combination is pressed, in order unless told otherwise. */
export function createShortcut(
    keys: KbdKey[],
    callback: (event: KeyboardEvent | null) => void,
    options: ShortcutOptions = {},
): void {
    // listen nowhere on the server or for no keys
    if (isServer || keys.length === 0) {
        return;
    }
    const wanted = keys.map((key) => key.toUpperCase());
    const {
        preventDefault = true,
        requireReset = false,
        ignoreWithinInputs = false,
        anyOrder = false,
    } = options;

    // track the pressed keys and the sets held so far in plain state, which the event reads right away
    let pressed: string[] = [];
    let sequence: string[][] = [];
    let isReset = false;
    const resetAll = (): void => {
        pressed = [];
        sequence = [];
        isReset = false;
    };

    // record each newly pressed key, then match the held sets against the shortcut
    makeEventListener(window, "keydown", (event) => {
        // skip repeats, unnamed keys and fields when told to
        const key = keyOf(event);
        if (
            event.repeat ||
            key === undefined ||
            (ignoreWithinInputs && isEditableTarget(event.target))
        ) {
            return;
        }
        if (!pressed.includes(key)) {
            pressed =
                pressed.length === 0 ? [...heldModifiers(event, key), key] : [...pressed, key];
            sequence = [...sequence, [...pressed]];
        }

        // fire once until released, cancelling the shortcut at the first wrong key
        if (requireReset) {
            if (isReset) {
                return;
            }
            const holding = sequence.at(-1) ?? [];
            if (sequence.length < wanted.length) {
                const isOnTrack = anyOrder
                    ? isSubset(holding, wanted)
                    : isHoldSequence(sequence, wanted.slice(0, sequence.length));
                if (isOnTrack) {
                    prevent(event, preventDefault);
                } else {
                    isReset = true;
                }
            } else {
                isReset = true;
                if (anyOrder ? isSameSet(holding, wanted) : isHoldSequence(sequence, wanted)) {
                    prevent(event, preventDefault);
                    callback(event);
                }
            }

            return;
        }

        // prevent the default one key short of the shortcut, and fire on the full set
        const last = sequence.at(-1);
        if (last === undefined) {
            return;
        }
        if (last.length < wanted.length) {
            const isOneKeyAway = anyOrder
                ? last.length === wanted.length - 1 && isSubset(last, wanted)
                : isEqual(last, wanted.slice(0, -1));
            if (preventDefault && isOneKeyAway) {
                event.preventDefault();
            }

            return;
        }
        if (anyOrder ? isSameSet(last, wanted) : isEqual(last, wanted)) {
            const previous = sequence.at(-2);
            const wasOneKeyAway =
                previous !== undefined &&
                (anyOrder
                    ? previous.length === wanted.length - 1 && isSubset(previous, wanted)
                    : isEqual(previous, wanted.slice(0, -1)));
            if (previous === undefined || wasOneKeyAway) {
                prevent(event, preventDefault);
                callback(event);
            }
        }
    });

    // drop released keys, and everything once Meta goes, keeping the held keys to press again
    makeEventListener(window, "keyup", (event) => {
        // read the released key
        const key = keyOf(event);
        if (key === undefined) {
            return;
        }
        if (key === "META") {
            resetAll();
        } else {
            pressed = pressed.filter((held) => held !== key);
            sequence = pressed.length === 0 ? [] : [[...pressed]];
            isReset = isReset && pressed.length > 0;
        }
    });

    // forget every key when the window loses focus or a context menu opens
    makeEventListener(window, "blur", resetAll);
    makeEventListener(window, "contextmenu", (event) => {
        if (!event.defaultPrevented) {
            resetAll();
        }
    });
}

/** Call back when a key goes down on a document, the window's by default. */
export function createKeyDown(
    key: KbdKey,
    callback: (event: KeyboardEvent) => void,
    options?: CreateKeyDownOptions,
): void {
    // listen nowhere on the server
    if (isServer) {
        return;
    }

    // listen on the document while enabled, again whenever either changes
    createEffect(
        () => ({
            isDisabled: access(options?.disabled) === true,
            target: options?.ownerDocument?.() ?? window.document,
        }),
        ({ isDisabled, target }) => {
            if (isDisabled) {
                return undefined;
            }
            const listener = (event: KeyboardEvent): void => {
                if (event.key === key) {
                    callback(event);
                }
            };
            target.addEventListener("keydown", listener);

            return () => target.removeEventListener("keydown", listener);
        },
        TRANSPARENT,
    );
}

/**
 * Name an event's key upper-cased, as the primitives track keys.
 *
 * With Option held, Command platforms report the character it types, so the letter or digit comes from the physical key instead.
 * Some autocomplete keydowns have no key at all, which reads as undefined.
 */
function keyOf(event: KeyboardEvent): string | undefined {
    // read nothing from an event without a key
    if (typeof event.key !== "string") {
        return undefined;
    }

    // read the physical letter or digit where Option typed another character
    const physical = /^(?:Key([A-Z])|Digit(\d))$/u.exec(event.code);
    if (event.altKey && !/^[a-z0-9]$/iu.test(event.key) && physical !== null) {
        return physical[1] ?? physical[2];
    }

    return event.key.toUpperCase();
}

/** List the modifiers held for a first non-modifier key, as they were pressed before listening began. */
function heldModifiers(event: KeyboardEvent, key: string): string[] {
    if (MODIFIERS.has(key)) {
        return [];
    }

    return [
        ...(event.metaKey ? ["META"] : []),
        ...(event.ctrlKey ? ["CONTROL"] : []),
        ...(event.altKey ? ["ALT"] : []),
        ...(event.shiftKey ? ["SHIFT"] : []),
    ];
}

/** Prevent an event's default when asked to. */
function prevent(event: KeyboardEvent, shouldPrevent: boolean): void {
    if (shouldPrevent) {
        event.preventDefault();
    }
}

/** Check whether each set of a held sequence matches the prefix of the shortcut it should have reached. */
function isHoldSequence(sequence: string[][], model: string[]): boolean {
    return sequence.every((held, index) => isEqual(held, model.slice(0, index + 1)));
}

/** Check whether two key lists hold the same keys in the same order. */
function isEqual(first: readonly string[], second: readonly string[]): boolean {
    return first.length === second.length && first.every((key, index) => key === second[index]);
}

/** Check whether every key of one list is in another. */
function isSubset(subset: readonly string[], superset: readonly string[]): boolean {
    return subset.every((key) => superset.includes(key));
}

/** Check whether two key lists hold the same keys in any order. */
function isSameSet(first: readonly string[], second: readonly string[]): boolean {
    return new Set(first).size === new Set(second).size && isSubset(first, second);
}

/** Check whether an event's target is a field that takes typing. */
function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) {
        return false;
    }

    return (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        target.isContentEditable
    );
}
