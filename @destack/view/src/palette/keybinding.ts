import { defineSchema, schema } from "@destack/schema";

/** A key combination in the keybinding syntax VS Code writes, such as `alt+space` or `mod+shift+k`, `mod` being Command on macOS and Control elsewhere. */
export const Accelerator = Object.assign(
    defineSchema(
        schema
            .string()
            .regex(
                /^(?:(?:mod|ctrl|alt|shift|meta)\+)*(?:[a-z0-9]|space|enter|escape|tab|backspace|delete|up|down|left|right|f[0-9]{1,2}|[,./;'[\]\\`=-])$/u,
            ),
    ),
    {
        /** Report whether a key press presses an accelerator, `mod` being Command on macOS and Control elsewhere. */
        matches(accelerator: string, event: KeyPress, isMac: boolean): boolean {
            // read the modifiers and the key the accelerator names
            const parts = accelerator.split("+");
            const modifiers = new Set(parts.slice(0, -1));
            const isCommand = modifiers.has("mod");

            // require exactly the named modifiers and the key
            return (
                parts.at(-1) === keyName(event.key) &&
                event.altKey === modifiers.has("alt") &&
                event.shiftKey === modifiers.has("shift") &&
                event.ctrlKey === (modifiers.has("ctrl") || (isCommand && !isMac)) &&
                event.metaKey === (modifiers.has("meta") || (isCommand && isMac))
            );
        },
    },
);
/** A key combination in VS Code's keybinding syntax. */
export type Accelerator = schema.Infer<typeof Accelerator>;

/** A key press as a keyboard event reports it. */
export interface KeyPress {
    /** The key's value. */
    readonly key: string;
    /** Whether Alt or Option was held. */
    readonly altKey: boolean;
    /** Whether Control was held. */
    readonly ctrlKey: boolean;
    /** Whether Command or the Windows key was held. */
    readonly metaKey: boolean;
    /** Whether Shift was held. */
    readonly shiftKey: boolean;
}

/** Name a key as accelerators do: lowercase, with named keys spelled out. */
function keyName(key: string): string {
    const names: Readonly<Record<string, string>> = {
        " ": "space",
        ArrowUp: "up",
        ArrowDown: "down",
        ArrowLeft: "left",
        ArrowRight: "right",
    };

    return names[key] ?? key.toLowerCase();
}
