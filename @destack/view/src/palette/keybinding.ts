import { DeclarationReference } from "@destack/package/declare";
import { Shortcut } from "@destack/object";
import { schema } from "@destack/schema";
import { ActionFocus } from "../declare/action.ts";

/** The user agents of platforms whose `mod` key is Command. */
const COMMAND_PLATFORM = /Mac|iPhone|iPad/u;

/** A key combination running a command or an action, outside form fields or anywhere. */
const keybinding = schema.object({
    /** The command or action it runs. */
    command: DeclarationReference,
    /** The key combination. */
    key: Shortcut,
    /** Where focus may be for the keys to run it. */
    focus: ActionFocus,
});

/** A key combination running a command or an action, outside form fields or anywhere. */
export const Keybinding = Object.assign(keybinding, {
    /** Bind a command to the person's key combination for it, else its declared one, if any. */
    of(
        command: DeclarationReference,
        declared: { readonly key: Shortcut | undefined; readonly focus: ActionFocus },
        overrides: Readonly<Record<string, Shortcut | null>>,
    ): Keybinding | undefined {
        const settingKey = DeclarationReference.key(command);
        const key = settingKey in overrides ? overrides[settingKey] : declared.key;

        return key === null || key === undefined
            ? undefined
            : { command, key, focus: declared.focus };
    },
});
/** A key combination running a command or an action. */
export type Keybinding = schema.Infer<typeof keybinding>;

/** A command a package declares, with the key combination it declares. */
export const CommandDeclaration = DeclarationReference.extend({
    /** The key combination the package declares, absent for none. */
    keybinding: Shortcut.exactOptional(),
});
/** A command a package declares, with its key combination. */
export type CommandDeclaration = schema.Infer<typeof CommandDeclaration>;

/** Report whether the browser runs on a platform whose `mod` key is Command. */
export function isCommandPlatform(): boolean {
    return COMMAND_PLATFORM.test(navigator.userAgent);
}
