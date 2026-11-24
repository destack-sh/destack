import { schema } from "@destack/schema";

/** The user agents of platforms whose `mod` key is Command. */
const COMMAND_PLATFORM = /Mac|iPhone|iPad/u;

/** The symbols Command platforms label each modifier with. */
const COMMAND_MODIFIERS: Readonly<Record<string, string>> = {
    mod: "⌘",
    meta: "⌘",
    ctrl: "⌃",
    alt: "⌥",
    shift: "⇧",
};

/** The words other platforms label each modifier with. */
const MODIFIERS: Readonly<Record<string, string>> = {
    mod: "Ctrl",
    meta: "Win",
    ctrl: "Ctrl",
    alt: "Alt",
    shift: "Shift",
};

/** The symbols Command platforms label named keys with, after Apple's menus. */
const COMMAND_KEY_LABELS: Readonly<Record<string, string>> = {
    enter: "↵",
    escape: "⎋",
    tab: "⇥",
    backspace: "⌫",
    delete: "⌦",
};

/** The labels of named keys. */
const KEY_LABELS: Readonly<Record<string, string>> = {
    space: "Space",
    enter: "Enter",
    escape: "Esc",
    tab: "Tab",
    backspace: "Backspace",
    delete: "Delete",
    plus: "+",
    up: "↑",
    down: "↓",
    left: "←",
    right: "→",
};

/** The keys keyboard events name for the shortcut words on Command platforms. */
const COMMAND_KEYS: Readonly<Record<string, string>> = {
    mod: "Meta",
    meta: "Meta",
    ctrl: "Control",
    alt: "Alt",
    shift: "Shift",
    space: " ",
    enter: "Enter",
    escape: "Escape",
    tab: "Tab",
    backspace: "Backspace",
    delete: "Delete",
    plus: "+",
    up: "ArrowUp",
    down: "ArrowDown",
    left: "ArrowLeft",
    right: "ArrowRight",
};

/** The keys keyboard events name for the shortcut words elsewhere. */
const KEYS: Readonly<Record<string, string>> = { ...COMMAND_KEYS, mod: "Control" };

/** The keys screen readers are told by another word than their event's. */
const ARIA_KEYS: Readonly<Record<string, string>> = { " ": "Space", "+": "Plus" };

/** The shortcut words of the keys whose event names differ from them. */
const EVENT_KEY_NAMES: Readonly<Record<string, string>> = {
    " ": "space",
    "+": "plus",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
};

/** A modifier that is pressed with a shortcut's key. */
type ShortcutModifier = "mod" | "ctrl" | "alt" | "shift" | "meta";

/** A letter key. */
type Letter =
    | "a"
    | "b"
    | "c"
    | "d"
    | "e"
    | "f"
    | "g"
    | "h"
    | "i"
    | "j"
    | "k"
    | "l"
    | "m"
    | "n"
    | "o"
    | "p"
    | "q"
    | "r"
    | "s"
    | "t"
    | "u"
    | "v"
    | "w"
    | "x"
    | "y"
    | "z";

/** A digit key. */
type Digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";

/** A key a shortcut presses, as its schema reads it. */
type ShortcutKey =
    | Letter
    | Digit
    | "space"
    | "enter"
    | "escape"
    | "tab"
    | "backspace"
    | "delete"
    | "plus"
    | "up"
    | "down"
    | "left"
    | "right"
    | `f${Digit}`
    | `f${Digit}${Digit}`
    | ","
    | "."
    | "/"
    | ";"
    | "'"
    | "["
    | "]"
    | "\\"
    | "`"
    | "="
    | "-";

/** A literal shortcut the compiler checks, never for one the syntax refuses. */
export type ShortcutText<Text extends string> = Text extends `${ShortcutModifier}+${infer Rest}`
    ? [ShortcutText<Rest>] extends [never]
        ? never
        : Text
    : Text extends ShortcutKey
      ? Text
      : never;

/** A key combination in the keybinding syntax editors write. */
export const Shortcut = Object.assign(
    schema
        .string()
        .regex(
            /^(?:(?:(?:mod|ctrl|alt|shift|meta)\+)*(?:[a-z0-9]|space|enter|escape|tab|backspace|delete|up|down|left|right|f[0-9]{1,2}|[,./;'[\]\\`=-])|(?:(?:mod|ctrl|alt|meta)\+)*plus)$/u,
        ),
    {
        /** Report whether a user agent runs on a platform whose `mod` key is Command. */
        isCommand(userAgent: string): boolean {
            return COMMAND_PLATFORM.test(userAgent);
        },

        /** List the labels of a shortcut's modifiers and key the way each platform prints them. */
        labels(shortcut: string, isCommand: boolean): string[] {
            // name each modifier, refusing one the syntax lacks, then the key
            const parts = shortcut.split("+");
            const key = parts.at(-1) ?? shortcut;
            const modifiers = parts.slice(0, -1).map((modifier) => {
                const label = isCommand ? COMMAND_MODIFIERS[modifier] : MODIFIERS[modifier];
                if (label === undefined) {
                    throw new RangeError(`shortcut '${shortcut}' has no modifier '${modifier}'`);
                }

                return label;
            });

            // label the key with its symbol on Command platforms, else its name
            const symbol = isCommand ? COMMAND_KEY_LABELS[key] : undefined;

            return [...modifiers, symbol ?? KEY_LABELS[key] ?? key.toUpperCase()];
        },

        /** Write a shortcut for people. */
        format(shortcut: string, isCommand: boolean): string {
            return Shortcut.labels(shortcut, isCommand).join(isCommand ? "" : "+");
        },

        /** List the keys of a shortcut as keyboard events call them. */
        keys(shortcut: string, isCommand: boolean): string[] {
            return shortcut
                .split("+")
                .map((part) => (isCommand ? COMMAND_KEYS[part] : KEYS[part]) ?? part.toUpperCase());
        },

        /** Write a shortcut's keys for screen readers, as `aria-keyshortcuts` does. */
        aria(shortcut: string, isCommand: boolean): string {
            return Shortcut.keys(shortcut, isCommand)
                .map((key) => ARIA_KEYS[key] ?? key)
                .join("+");
        },

        /** Report whether a key press presses a shortcut. */
        matches(shortcut: string, event: KeyPress, isCommand: boolean): boolean {
            // read the modifiers and the shortcut's key
            const parts = shortcut.split("+");
            const modifiers = new Set(parts.slice(0, -1));
            const hasMod = modifiers.has("mod");

            // require exactly the named modifiers and the key, leaving out the shift that types plus
            const key = parts.at(-1);

            return (
                key === keyName(event) &&
                event.altKey === modifiers.has("alt") &&
                (key === "plus" || event.shiftKey === modifiers.has("shift")) &&
                event.ctrlKey === (modifiers.has("ctrl") || (hasMod && !isCommand)) &&
                event.metaKey === (modifiers.has("meta") || (hasMod && isCommand))
            );
        },
    },
);
/** A key combination in the keybinding syntax editors write. */
export type Shortcut = schema.Infer<typeof Shortcut>;

/** A key press as a keyboard event reports it. */
export interface KeyPress {
    /** The key's value. */
    readonly key: string;
    /** The physical key, such as `KeyD`, which Option leaves unchanged on Command platforms. */
    readonly code: string;
    /** Whether Alt or Option was held. */
    readonly altKey: boolean;
    /** Whether Control was held. */
    readonly ctrlKey: boolean;
    /** Whether Command or the system key was held. */
    readonly metaKey: boolean;
    /** Whether Shift was held. */
    readonly shiftKey: boolean;
}

/** Name a pressed key as shortcuts do: lowercase, with named keys spelled out. */
function keyName(press: KeyPress): string {
    // read the key the layout produced
    const key = EVENT_KEY_NAMES[press.key] ?? press.key.toLowerCase();

    // read the letter or digit from the physical key where Option typed another character
    const physical = /^(?:Key([A-Z])|Digit([0-9]))$/u.exec(press.code);
    if (press.altKey && !/^[a-z0-9]$/u.test(key) && physical !== null) {
        return (physical[1] ?? physical[2] ?? key).toLowerCase();
    }

    return key;
}
