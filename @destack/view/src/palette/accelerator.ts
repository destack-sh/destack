import { defineSchema, schema } from "@destack/schema";

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

/** The labels of named keys. */
const KEY_LABELS: Readonly<Record<string, string>> = {
    space: "Space",
    enter: "Enter",
    escape: "Esc",
    tab: "Tab",
    backspace: "Backspace",
    delete: "Delete",
    up: "↑",
    down: "↓",
    left: "←",
    right: "→",
};

/** The keys keyboard events name for the accelerator words on Command platforms. */
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
    up: "ArrowUp",
    down: "ArrowDown",
    left: "ArrowLeft",
    right: "ArrowRight",
};

/** The keys keyboard events name for the accelerator words elsewhere. */
const KEYS: Readonly<Record<string, string>> = { ...COMMAND_KEYS, mod: "Control" };

/** The accelerator words of the keys whose event names differ from them. */
const EVENT_KEY_NAMES: Readonly<Record<string, string>> = {
    " ": "space",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
};

/** The user agents of platforms whose `mod` key is Command. */
const COMMAND_PLATFORM = /Mac|iPhone|iPad/u;

/** A key combination in the keybinding syntax editors write, such as `alt+space` or `mod+shift+k`, `mod` being Command on Command platforms and Control elsewhere. */
export const Accelerator = Object.assign(
    defineSchema(
        schema
            .string()
            .regex(
                /^(?:(?:mod|ctrl|alt|shift|meta)\+)*(?:[a-z0-9]|space|enter|escape|tab|backspace|delete|up|down|left|right|f[0-9]{1,2}|[,./;'[\]\\`=-])$/u,
            ),
    ),
    {
        /** Write an accelerator for people: modifier symbols on Command platforms, words joined by plus signs elsewhere. */
        format(accelerator: string, isCommand: boolean): string {
            // name each modifier and the key the way each platform labels them
            const parts = accelerator.split("+");
            const key = parts.at(-1) ?? accelerator;
            const modifiers = parts
                .slice(0, -1)
                .map((modifier) => (isCommand ? COMMAND_MODIFIERS[modifier] : MODIFIERS[modifier]));
            const label = KEY_LABELS[key] ?? key.toUpperCase();

            return isCommand ? [...modifiers, label].join("") : [...modifiers, label].join("+");
        },

        /** List the keys of an accelerator as keyboard events name them, `mod` being Meta on Command platforms and Control elsewhere. */
        keys(accelerator: string, isCommand: boolean): string[] {
            return accelerator
                .split("+")
                .map((part) => (isCommand ? COMMAND_KEYS[part] : KEYS[part]) ?? part.toUpperCase());
        },

        /** Report whether a key press presses an accelerator, `mod` being Command on Command platforms and Control elsewhere. */
        matches(accelerator: string, event: KeyPress, isCommand: boolean): boolean {
            // read the modifiers and the key the accelerator names
            const parts = accelerator.split("+");
            const modifiers = new Set(parts.slice(0, -1));
            const hasMod = modifiers.has("mod");

            // require exactly the named modifiers and the key
            return (
                parts.at(-1) === keyName(event) &&
                event.altKey === modifiers.has("alt") &&
                event.shiftKey === modifiers.has("shift") &&
                event.ctrlKey === (modifiers.has("ctrl") || (hasMod && !isCommand)) &&
                event.metaKey === (modifiers.has("meta") || (hasMod && isCommand))
            );
        },
    },
);
/** A key combination in the keybinding syntax editors write. */
export type Accelerator = schema.Infer<typeof Accelerator>;

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

/** Name a pressed key as accelerators do: lowercase, with named keys spelled out. */
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

/** Report whether the browser runs on a platform whose `mod` key is Command. */
export function isCommandPlatform(): boolean {
    return COMMAND_PLATFORM.test(navigator.userAgent);
}
